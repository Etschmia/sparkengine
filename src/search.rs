// Funken search: iterative-deepening negamax with alpha-beta, transposition
// table, quiescence, and standard selective-search heuristics.
//
// Combination (all implemented from textbook descriptions, own code):
// aspiration windows, mate-distance pruning, null-move pruning, late-move
// reductions, check extensions, futility pruning, delta pruning in
// quiescence, MVV-LVA + killer + history ordering, repetition / fifty-move /
// insufficient-material draws. Mate scores are stored ply-normalised.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::chess::*;
use crate::eval::evaluate;

pub const MATE: i32 = 32000;
pub const INF: i32 = 33000;
pub const MAX_PLY: usize = 64;

pub const FLAG_NONE: u8 = 0;
pub const FLAG_EXACT: u8 = 1;
pub const FLAG_LOWER: u8 = 2;
pub const FLAG_UPPER: u8 = 3;

#[derive(Clone, Copy)]
struct TTEntry {
    hash: u64,
    mv: Move,
    score: i32,
    depth: i8,
    flag: u8,
}

impl Default for TTEntry {
    fn default() -> Self {
        TTEntry { hash: 0, mv: Move::null(), score: 0, depth: -1, flag: FLAG_NONE }
    }
}

pub struct TransTable {
    slots: Vec<TTEntry>,
}

impl TransTable {
    pub fn new_mb(mb: usize) -> TransTable {
        let mb = mb.clamp(1, 1024);
        // ~32 bytes per entry upper bound; keeps allocation predictable.
        let n = (mb << 20) / 32;
        TransTable { slots: vec![TTEntry::default(); n] }
    }
    pub fn clear(&mut self) {
        for s in self.slots.iter_mut() {
            *s = TTEntry::default();
        }
    }
    fn idx(&self, hash: u64) -> usize {
        (hash % self.slots.len() as u64) as usize
    }
    pub fn probe(&self, hash: u64) -> Option<(Move, i32, i8, u8)> {
        let e = &self.slots[self.idx(hash)];
        if e.flag != FLAG_NONE && e.hash == hash {
            Some((e.mv, e.score, e.depth, e.flag))
        } else {
            None
        }
    }
    pub fn store(&mut self, hash: u64, mv: Move, score: i32, depth: i8, flag: u8) {
        let i = self.idx(hash);
        let e = &mut self.slots[i];
        // Depth-preferred replacement with a small tolerance so fresh
        // best-moves still enter the table for ordering.
        if e.flag == FLAG_NONE || e.hash != hash || depth + 2 >= e.depth {
            *e = TTEntry { hash, mv, score, depth, flag };
        }
    }
    pub fn hashfull(&self) -> u32 {
        let n = self.slots.len().min(1000);
        let used = self.slots.iter().take(n).filter(|e| e.flag != FLAG_NONE).count();
        (used * 1000 / n) as u32
    }
}

#[derive(Clone, Default)]
pub struct SearchLimits {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u32>,
    pub movetime: Option<u64>,
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
    pub infinite: bool,
}

pub struct SearchInfo {
    pub best: Move,
    pub score: i32,
    pub depth_completed: u8,
    pub nodes: u64,
    pub seldepth: u32,
    pub elapsed_ms: u64,
    pub pv: Vec<Move>,
}

pub struct Searcher {
    pub board: Board,
    pub tt: TransTable,
    killers: [[Move; 2]; MAX_PLY],
    history: [[i32; 64]; 12],
    nodes: u64,
    seldepth: u32,
    stop: Arc<AtomicBool>,
    t0: Instant,
    soft: Option<Instant>,
    hard: Option<Instant>,
    max_nodes: u64,
    max_depth: u8,
    completed_depth: u8,
    stack: Vec<u64>,
    pv_len: [usize; MAX_PLY],
    pv: [[Move; MAX_PLY]; MAX_PLY],
    aborted: bool,
    scratch: Vec<Move>,
}

impl Searcher {
    pub fn new(board: Board, tt: TransTable, game_hashes: Vec<u64>, stop: Arc<AtomicBool>) -> Searcher {
        Searcher {
            board,
            tt,
            killers: [[Move::null(); 2]; MAX_PLY],
            history: [[0; 64]; 12],
            nodes: 0,
            seldepth: 0,
            stop,
            t0: Instant::now(),
            soft: None,
            hard: None,
            max_nodes: u64::MAX,
            max_depth: 64,
            completed_depth: 0,
            stack: game_hashes,
            pv_len: [0; MAX_PLY],
            pv: [[Move::null(); MAX_PLY]; MAX_PLY],
            aborted: false,
            scratch: Vec::with_capacity(128),
        }
    }

    fn should_abort(&mut self) -> bool {
        if self.aborted {
            return true;
        }
        if self.stop.load(Ordering::Relaxed) {
            self.aborted = true;
            return true;
        }
        if self.nodes >= self.max_nodes {
            self.aborted = true;
            return true;
        }
        if (self.nodes & 2047) == 0 {
            let now = Instant::now();
            if let Some(h) = self.hard {
                if now >= h {
                    self.aborted = true;
                    return true;
                }
            }
            if let Some(s) = self.soft {
                // Soft limit: abort mid-iteration once a usable result
                // (depth >= 1 completed) exists. The previous iteration's
                // best move is kept.
                if now >= s && self.completed_depth >= 1 {
                    self.aborted = true;
                    return true;
                }
            }
        }
        false
    }

    fn has_non_pawn(&self, side: u8) -> bool {
        for s in 0..64 {
            let p = self.board.sq[s];
            if p != EMPTY && color_of(p) == side {
                let t = type_of(p);
                if t >= 1 && t <= 4 {
                    return true;
                }
            }
        }
        false
    }

    /// True triple repetition: `stack` always contains the current hash
    /// (pushed by the ID driver / before recursing), so a draw needs the
    /// hash 3 times in total, i.e. twice *before* the current occurrence.
    fn is_repetition(&self) -> bool {
        let h = self.board.hash;
        let mut count = 0;
        for &x in self.stack.iter() {
            if x == h {
                count += 1;
                if count >= 3 {
                    return true;
                }
            }
        }
        false
    }

    #[inline]
    fn mate_to_tt(score: i32, ply: i32) -> i32 {
        if score > MATE - 1000 {
            score + ply
        } else if score < -MATE + 1000 {
            score - ply
        } else {
            score
        }
    }

    #[inline]
    fn mate_from_tt(score: i32, ply: i32) -> i32 {
        if score > MATE - 1000 {
            score - ply
        } else if score < -MATE + 1000 {
            score + ply
        } else {
            score
        }
    }

    // Null move: side passes; en-passant right disappears.
    fn make_null(&mut self) -> (u8, u16) {
        let z = zobrist();
        if self.board.ep != NO_SQ {
            self.board.hash ^= z.ep_file[(self.board.ep & 7) as usize];
        }
        let old_ep = self.board.ep;
        self.board.ep = NO_SQ;
        self.board.hash ^= z.side;
        self.board.side = opp(self.board.side);
        self.board.half += 1;
        (old_ep, self.board.half)
    }

    fn unmake_null(&mut self, old_ep: u8, old_half: u16) {
        let z = zobrist();
        self.board.hash ^= z.side;
        self.board.side = opp(self.board.side);
        if old_ep != NO_SQ {
            self.board.hash ^= z.ep_file[(old_ep & 7) as usize];
        }
        self.board.ep = old_ep;
        self.board.half = old_half;
    }

    fn order_score(&self, m: &Move, tt_move: &Move) -> i32 {
        if m.from == tt_move.from && m.to == tt_move.to && m.promo == tt_move.promo {
            return i32::MAX - 1;
        }
        if m.capture {
            let victim = if m.en_passant {
                100
            } else {
                PIECE_VALUE[type_of(self.board.sq[m.to as usize]) as usize]
            };
            let attacker = PIECE_VALUE[type_of(self.board.sq[m.from as usize]) as usize];
            let mut s = 10 * victim - attacker / 16 + 1_000_000;
            if m.promo != 0 {
                s += 800_000 + (m.promo as i32) * 10_000;
            }
            return s;
        }
        if m.promo != 0 {
            return 900_000 + (m.promo as i32) * 10_000;
        }
        // killers are ply-relative; caller passes ply. (Handled via history fallback here.)
        let h = self.history[self.board.sq[m.from as usize] as usize - 1][m.to as usize];
        h
    }

    fn pick_next(&self, moves: &mut [Move], scores: &mut [i32], start: usize) {
        let mut best = start;
        for i in (start + 1)..moves.len() {
            if scores[i] > scores[best] {
                best = i;
            }
        }
        moves.swap(start, best);
        scores.swap(start, best);
    }

    pub fn quiescence(&mut self, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if ply >= MAX_PLY - 1 {
            let s = evaluate(&self.board);
            return if self.board.side == WHITE { s } else { -s };
        }
        // No PV tracked inside quiescence; terminate any parent PV copy here.
        self.pv_len[ply] = ply;
        self.nodes += 1;
        if self.should_abort() {
            return 0;
        }
        if self.board.half >= 100 || self.board.insufficient_material() || self.is_repetition() {
            return 0;
        }
        let in_check = self.board.in_check(self.board.side);
        if !in_check {
            let stand = if self.board.side == WHITE { evaluate(&self.board) } else { -evaluate(&self.board) };
            if stand >= beta {
                return beta;
            }
            if stand > alpha {
                alpha = stand;
            }
            // Delta pruning: skip captures that cannot raise alpha even with a
            // generous bonus (queen value + promotion swing).
            let mut moves = std::mem::take(&mut self.scratch);
            self.board.gen_captures(&mut moves);
            let mut scores = Vec::with_capacity(moves.len());
            let tt_null = Move::null();
            for m in moves.iter() {
                scores.push(self.order_score(m, &tt_null));
            }
            for i in 0..moves.len() {
                self.pick_next(&mut moves, &mut scores, i);
                let m = moves[i];
                if !in_check {
                    let gain = if m.en_passant {
                        100
                    } else {
                        PIECE_VALUE[type_of(self.board.sq[m.to as usize]) as usize]
                    } + if m.promo == 4 { 800 } else { 0 };
                    if stand + gain + 200 < alpha {
                        continue;
                    }
                }
                let u = self.board.make(&m);
                let us = opp(self.board.side);
                if self.board.is_attacked(self.board.king[us as usize], self.board.side) {
                    self.board.unmake(&m, &u);
                    continue;
                }
                self.stack.push(self.board.hash);
                let score = -self.quiescence(ply + 1, -beta, -alpha);
                self.stack.pop();
                self.board.unmake(&m, &u);
                if self.aborted {
                    self.scratch = moves;
                    return 0;
                }
                if score >= beta {
                    self.scratch = moves;
                    return beta;
                }
                if score > alpha {
                    alpha = score;
                }
            }
            self.scratch = moves;
            alpha
        } else {
            // In check: search all evasions.
            let mut pseudo = Vec::with_capacity(16);
            self.board.gen_pseudo(&mut pseudo);
            let mut legal = 0;
            let mut best = -INF;
            for m in pseudo {
                let u = self.board.make(&m);
                let us = opp(self.board.side);
                if self.board.is_attacked(self.board.king[us as usize], self.board.side) {
                    self.board.unmake(&m, &u);
                    continue;
                }
                legal += 1;
                self.stack.push(self.board.hash);
                let score = -self.quiescence(ply + 1, -beta, -alpha);
                self.stack.pop();
                self.board.unmake(&m, &u);
                if self.aborted {
                    return 0;
                }
                if score >= beta {
                    return beta;
                }
                if score > best {
                    best = score;
                    if score > alpha {
                        alpha = score;
                    }
                }
            }
            if legal == 0 {
                return -MATE + ply as i32;
            }
            alpha
        }
    }

    pub fn negamax(&mut self, ply: usize, depth: i8, alpha: i32, beta: i32) -> i32 {
        self.negamax_impl(ply, depth, alpha, beta, None)
    }

    fn negamax_impl(&mut self, ply: usize, depth: i8, mut alpha: i32, beta: i32, root_allow: Option<&[Move]>) -> i32 {
        if ply >= MAX_PLY - 1 {
            return self.quiescence(ply, alpha, beta);
        }
        self.nodes += 1;
        if self.should_abort() {
            return 0;
        }
        // Mate-distance pruning (lower bound only): we can always do at
        // least as well as being mated right now.
        let mated = -MATE + ply as i32;
        if mated > alpha {
            alpha = mated;
            if alpha >= beta {
                return alpha;
            }
        }
        if self.board.half >= 100 || self.board.insufficient_material() {
            self.pv_len[ply] = ply;
            return 0;
        }
        // Repetition: never shortcut the root (ply 0). The engine must always
        // search the legal moves and return a reasoned move, even if the
        // current position already occurred twice before (claimable draw in
        // a real game, but UCI still requires a move). Exact triple only.
        if ply > 0 && self.is_repetition() {
            self.pv_len[ply] = ply;
            return 0;
        }
        let in_check = self.board.in_check(self.board.side);
        if depth <= 0 {
            return self.quiescence(ply, alpha, beta);
        }
        // Triangular PV: child nodes extend pv[ply+1..]; default = empty.
        self.pv_len[ply] = ply;

        // Transposition-table probe. At the root (ply 0) the TT move is used
        // for ordering only: a cutoff here would return without a PV, and the
        // driver would fall back to the first generated move.
        let mut tt_move = Move::null();
        if let Some((mv, s, d, flag)) = self.tt.probe(self.board.hash) {
            tt_move = mv;
            if ply > 0 && d as i32 >= depth as i32 {
                let score = Self::mate_from_tt(s, ply as i32);
                match flag {
                    FLAG_EXACT => return score,
                    FLAG_LOWER => {
                        if score >= beta {
                            return score;
                        }
                    }
                    FLAG_UPPER => {
                        if score <= alpha {
                            return score;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Static eval for pruning decisions.
        let static_eval = if in_check {
            -INF
        } else if self.board.side == WHITE {
            evaluate(&self.board)
        } else {
            -evaluate(&self.board)
        };

        // Reverse futility: clearly good enough to fail high.
        if !in_check && depth <= 4 && static_eval - 90 * depth as i32 >= beta && static_eval < MATE - 1000 {
            return static_eval;
        }

        // Null-move pruning.
        if !in_check && depth >= 3 && static_eval >= beta && self.has_non_pawn(self.board.side) && beta < MATE - 1000 {
            let r = 2 + if depth > 6 { 1 } else { 0 };
            let (old_ep, old_half) = self.make_null();
            self.stack.push(self.board.hash);
            let score = -self.negamax(ply + 1, depth - 1 - r, -beta, -beta + 1);
            self.stack.pop();
            self.unmake_null(old_ep, old_half);
            if self.aborted {
                return 0;
            }
            if score >= beta {
                return beta;
            }
        }

        // Generate moves. At the root with a `searchmoves` allow-list only
        // those moves are considered.
        let mut pseudo: Vec<Move> = Vec::with_capacity(64);
        if ply == 0 {
            if let Some(a) = root_allow {
                pseudo.extend_from_slice(a);
            } else {
                self.board.gen_pseudo(&mut pseudo);
            }
        } else {
            self.board.gen_pseudo(&mut pseudo);
        }
        let mut scores = Vec::with_capacity(pseudo.len());
        for m in pseudo.iter() {
            let mut s = self.order_score(m, &tt_move);
            // killer boost (ply-relative, quiet moves only)
            if !m.capture && m.promo == 0 {
                if self.killers[ply][0].from == m.from && self.killers[ply][0].to == m.to {
                    s = s.max(850_000);
                } else if self.killers[ply][1].from == m.from && self.killers[ply][1].to == m.to {
                    s = s.max(840_000);
                }
            }
            scores.push(s);
        }

        let mut legal = 0;
        let mut best_score = -INF;
        let mut best_move = Move::null();
        let mut flag = FLAG_UPPER;

        // Futility margin for shallow depths.
        let futile_base = if !in_check && depth <= 2 { static_eval } else { -INF };
        let fut_margin = [0, 150, 250][depth.min(2) as usize];

        for i in 0..pseudo.len() {
            self.pick_next(&mut pseudo, &mut scores, i);
            let m = pseudo[i];
            let is_cap = m.capture;
            let is_promo = m.promo != 0;
            // Futility: skip quiet moves that cannot catch up.
            if !in_check && depth <= 2 && legal > 0 && !is_cap && !is_promo && futile_base + fut_margin < alpha {
                continue;
            }
            let u = self.board.make(&m);
            let us = opp(self.board.side);
            if self.board.is_attacked(self.board.king[us as usize], self.board.side) {
                self.board.unmake(&m, &u);
                continue;
            }
            legal += 1;
            // Check extension.
            let gives_check = self.board.is_attacked(self.board.king[self.board.side as usize], us);
            let ext: i8 = if gives_check { 1 } else { 0 };
            self.stack.push(self.board.hash);
            if (ply + 1) as u32 > self.seldepth {
                self.seldepth = (ply + 1) as u32;
            }
            let score;
            if legal == 1 {
                score = -self.negamax(ply + 1, depth - 1 + ext, -beta, -alpha);
            } else {
                // Late-move reduction for quiet moves.
                let mut red: i8 = 0;
                if depth >= 3 && !in_check && !is_cap && !is_promo && !gives_check && legal > 3 {
                    red = 1;
                    if depth >= 6 && legal > 8 {
                        red = 2;
                    }
                }
                if red > 0 {
                    let rscore = -self.negamax(ply + 1, depth - 1 - red + ext, -alpha - 1, -alpha);
                    if self.aborted {
                        self.stack.pop();
                        self.board.unmake(&m, &u);
                        return 0;
                    }
                    if rscore > alpha {
                        score = -self.negamax(ply + 1, depth - 1 + ext, -beta, -alpha);
                    } else {
                        score = rscore;
                    }
                } else {
                    // Principal-variation search.
                    let pvs = -self.negamax(ply + 1, depth - 1 + ext, -alpha - 1, -alpha);
                    if self.aborted {
                        self.stack.pop();
                        self.board.unmake(&m, &u);
                        return 0;
                    }
                    if pvs > alpha && pvs < beta {
                        score = -self.negamax(ply + 1, depth - 1 + ext, -beta, -alpha);
                    } else {
                        score = pvs;
                    }
                }
            }
            self.stack.pop();
            self.board.unmake(&m, &u);
            if self.aborted {
                return 0;
            }
            if score > best_score {
                best_score = score;
                best_move = m;
                if score > alpha {
                    alpha = score;
                    flag = FLAG_EXACT;
                    // update PV
                    self.pv[ply][ply] = m;
                    let mut j = ply + 1;
                    while j < self.pv_len[ply + 1] {
                        self.pv[ply][j] = self.pv[ply + 1][j];
                        j += 1;
                    }
                    self.pv_len[ply] = self.pv_len[ply + 1];
                    if score >= beta {
                        flag = FLAG_LOWER;
                        // killer + history update on quiet cutoffs
                        if !is_cap && is_promo == false {
                            if self.killers[ply][0].from != m.from || self.killers[ply][0].to != m.to {
                                self.killers[ply][1] = self.killers[ply][0];
                                self.killers[ply][0] = m;
                            }
                            let p = self.board.sq[m.from as usize];
                            // NB: board already unmade, so from-square holds the mover again.
                            if p != EMPTY {
                                let e = &mut self.history[p as usize - 1][m.to as usize];
                                *e = (*e + depth as i32 * depth as i32).min(1_000_000);
                            }
                        }
                        break;
                    }
                }
            }
        }

        if legal == 0 {
            // No legal moves: mated or stalemated.
            self.pv_len[ply] = ply;
            if in_check {
                return -MATE + ply as i32;
            } else {
                return 0;
            }
        }
        if !self.aborted {
            self.tt.store(self.board.hash, best_move, Self::mate_to_tt(best_score, ply as i32), depth, flag);
        }
        best_score
    }

    fn compute_time(&mut self, us: u8, lim: &SearchLimits, overhead: u64) {
        self.soft = None;
        self.hard = None;
        if let Some(mt) = lim.movetime {
            let t = mt.saturating_sub(overhead.min(mt));
            let now = Instant::now();
            self.soft = Some(now + std::time::Duration::from_millis(t));
            self.hard = Some(now + std::time::Duration::from_millis(t));
            return;
        }
        let (rem, inc) = if us == WHITE {
            (lim.wtime, lim.winc.unwrap_or(0))
        } else {
            (lim.btime, lim.binc.unwrap_or(0))
        };
        if let Some(r) = rem {
            // Own time formula: base = remaining/25 + inc/2 (sudden death),
            // or remaining/movestogo when given. Documented in KONZEPT.md.
            let mut base: u64 = if let Some(mtg) = lim.movestogo {
                r / mtg.max(1) as u64 + inc / 2
            } else {
                r / 25 + inc / 2
            };
            let margin = overhead + 10;
            if r <= margin {
                base = 0;
            } else {
                let max_use = r - margin;
                if base > max_use {
                    base = max_use;
                }
                if base < 10 && max_use >= 10 {
                    base = 10;
                }
            }
            let hard = (base * 4).min(r.saturating_sub(margin));
            let now = Instant::now();
            self.soft = Some(now + std::time::Duration::from_millis(base));
            self.hard = Some(now + std::time::Duration::from_millis(hard.max(base)));
        }
    }

    pub fn setup_limits(&mut self, lim: &SearchLimits, overhead: u64) {
        self.t0 = Instant::now();
        self.nodes = 0;
        self.seldepth = 0;
        self.aborted = false;
        self.max_nodes = lim.nodes.unwrap_or(u64::MAX);
        self.max_depth = lim.depth.unwrap_or(64).min(64);
        self.completed_depth = 0;
        self.compute_time(self.board.side, lim, overhead);
    }

    pub fn search(&mut self, lim: &SearchLimits, overhead: u64) -> SearchInfo {
        self.setup_limits(lim, overhead);
        let mut noop = |_d: u8, _s: i32, _n: u64, _sd: u32, _pv: &[Move]| {};
        self.id_loop(lim, None, &mut noop)
    }

    pub fn search_report(&mut self, lim: &SearchLimits, overhead: u64, report: &mut dyn FnMut(u8, i32, u64, u32, &[Move])) -> SearchInfo {
        self.setup_limits(lim, overhead);
        self.id_loop(lim, None, report)
    }

    pub fn search_root_restricted(
        &mut self,
        lim: &SearchLimits,
        overhead: u64,
        allow: &[Move],
        report: &mut dyn FnMut(u8, i32, u64, u32, &[Move]),
    ) -> SearchInfo {
        self.setup_limits(lim, overhead);
        self.id_loop(lim, Some(allow), report)
    }

    /// Iterative-deepening driver. `history` (game positions before the
    /// current one) is in `self.stack`; the current hash is pushed here.
    fn id_loop(
        &mut self,
        lim: &SearchLimits,
        allow: Option<&[Move]>,
        report: &mut dyn FnMut(u8, i32, u64, u32, &[Move]),
    ) -> SearchInfo {
        // Ensure current hash is on the stack for repetition detection.
        self.stack.push(self.board.hash);

        // Verify the allow-list against legal moves.
        let mut allowed_owned: Vec<Move> = vec![];
        if let Some(a) = allow {
            let mut legal = Vec::new();
            self.board.gen_legal(&mut legal);
            for m in a.iter() {
                if legal.iter().any(|x| x.from == m.from && x.to == m.to && x.promo == m.promo) {
                    allowed_owned.push(*m);
                }
            }
            if allowed_owned.is_empty() {
                self.board.gen_legal(&mut allowed_owned);
            }
        }
        let allow_ref: Option<&[Move]> = if allow.is_some() { Some(&allowed_owned) } else { None };

        let mut root_moves = Vec::new();
        self.board.gen_legal(&mut root_moves);
        if root_moves.is_empty() {
            self.stack.pop();
            return SearchInfo {
                best: Move::null(),
                score: if self.board.in_check(self.board.side) { -MATE } else { 0 },
                depth_completed: 0,
                nodes: 0,
                seldepth: 0,
                elapsed_ms: 0,
                pv: vec![],
            };
        }
        // Effective root set (for info only; negamax_impl enforces it).
        let root_count = allow_ref.map(|a| a.len()).unwrap_or(root_moves.len());

        let mut best = if allow_ref.map(|a| !a.is_empty()).unwrap_or(false) {
            allowed_owned[0]
        } else {
            root_moves[0]
        };
        let mut best_score = 0;
        let mut completed: u8 = 0;
        let mut completed_pv: Vec<Move> = vec![best];

        // Iterative deepening with aspiration windows from depth 4 on.
        let mut alpha0 = -INF;
        let mut beta0 = INF;
        let mut depth: u8 = 1;
        while depth <= self.max_depth {
            self.pv_len[0] = 0;
            let (mut alpha, mut beta) = if depth >= 4 { (alpha0 - 25, beta0 + 25) } else { (-INF, INF) };
            let score = loop {
                let s = self.negamax_impl(0, depth as i8, alpha, beta, allow_ref);
                if self.aborted {
                    break s;
                }
                if s <= alpha {
                    alpha = (alpha - 120).max(-INF);
                    beta = (alpha + beta) / 2;
                    continue;
                } else if s >= beta {
                    beta = (beta + 120).min(INF);
                    continue;
                } else {
                    break s;
                }
            };
            if self.aborted {
                break;
            }
            // Triangular PV of the completed iteration.
            let mut pv_moves = Vec::new();
            if self.pv_len[0] > 0 {
                for j in 0..self.pv_len[0] {
                    pv_moves.push(self.pv[0][j]);
                }
                best = self.pv[0][0];
            } else {
                pv_moves.push(best);
            }
            best_score = score;
            alpha0 = score;
            beta0 = score;
            completed = depth;
            self.completed_depth = depth;
            completed_pv = pv_moves.clone();
            report(depth, score, self.nodes, self.seldepth, &pv_moves);
            // Stop starting new iterations past the soft limit (depth 1 always completes).
            if let Some(soft) = self.soft {
                if Instant::now() >= soft {
                    break;
                }
            }
            if self.nodes >= self.max_nodes {
                break;
            }
            let _ = (lim, root_count);
            depth += 1;
            if depth > self.max_depth {
                break;
            }
        }

        self.stack.pop();
        let elapsed_ms = self.t0.elapsed().as_millis() as u64;
        SearchInfo { best, score: best_score, depth_completed: completed, nodes: self.nodes, seldepth: self.seldepth, elapsed_ms, pv: completed_pv }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn search_depth(fen: &str, depth: u8) -> SearchInfo {
        let b = Board::from_fen(fen).unwrap();
        let tt = TransTable::new_mb(1);
        let stop = Arc::new(AtomicBool::new(false));
        let mut s = Searcher::new(b, tt, vec![], stop);
        let mut lim = SearchLimits::default();
        lim.depth = Some(depth);
        s.search(&lim, 0)
    }

    #[test]
    fn mate_in_one_found() {
        let info = search_depth("6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1", 3);
        assert_eq!(info.best.to_uci(), "e1e8");
        assert!(info.score > MATE - 500, "score = {}", info.score);
    }

    #[test]
    fn stalemate_scores_zero() {
        let info = search_depth("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 3);
        assert!(info.best.is_null());
        assert_eq!(info.score, 0);
    }

    #[test]
    fn fifty_move_scores_zero() {
        let info = search_depth(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 100 1",
            3,
        );
        assert_eq!(info.score, 0);
    }

    /// Play UCI moves on a fresh startpos board, returning the final board
    /// plus the game history (hashes of all positions *before* the current).
    fn play_startpos(moves: &[&str]) -> (Board, Vec<u64>) {
        let mut b = Board::startpos();
        let mut hist = Vec::new();
        for mv in moves {
            hist.push(b.hash);
            let (ff, rf, tf, rt) = (
                (mv.as_bytes()[0] - b'a') as i8,
                (mv.as_bytes()[1] - b'1') as i8,
                (mv.as_bytes()[2] - b'a') as i8,
                (mv.as_bytes()[3] - b'1') as i8,
            );
            let (from, to) = (crate::chess::sq_at(ff, rf), crate::chess::sq_at(tf, rt));
            let promo = if mv.len() >= 5 {
                match mv.as_bytes()[4] as char {
                    'n' => 1,
                    'b' => 2,
                    'r' => 3,
                    'q' => 4,
                    _ => 0,
                }
            } else {
                0
            };
            let mut legal = Vec::new();
            b.gen_legal(&mut legal);
            let m = legal
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promo == promo)
                .unwrap_or_else(|| panic!("illegal test move {mv}"));
            let u = b.make(&m);
            let us = crate::chess::opp(b.side);
            assert!(!b.is_attacked(b.king[us as usize], b.side), "test move {mv} leaves king in check");
            let _ = u;
        }
        (b, hist)
    }

    fn search_with_history(board: Board, history: Vec<u64>, depth: u8) -> SearchInfo {
        let tt = TransTable::new_mb(1);
        let stop = Arc::new(AtomicBool::new(false));
        let mut s = Searcher::new(board, tt, history, stop);
        let mut lim = SearchLimits::default();
        lim.depth = Some(depth);
        s.search(&lim, 0)
    }

    #[test]
    fn second_occurrence_still_searches() {
        // Italienisch einmal ohne, einmal mit Vorgeschichte (selbe Stellung
        // zum ZWEITEN Mal auf dem Brett): noch kein Remis, die Wurzel muss
        // eine echte Suche liefern (Tiefe > 1, begründeter Zug).
        let italian = ["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5"];
        let shuffle = ["c4b5", "c5b4", "b5c4", "b4c5"];
        let (plain_board, _) = play_startpos(&italian);
        let mut seq = italian.to_vec();
        seq.extend_from_slice(&shuffle);
        let (rep_board, rep_hist) = play_startpos(&seq);
        assert_eq!(plain_board.hash, rep_board.hash);
        // Die Stellung kam genau einmal in der Historie vor (2. Auftreten).
        assert_eq!(rep_hist.iter().filter(|&&h| h == rep_board.hash).count(), 1);

        let base = search_with_history(plain_board, vec![], 3);
        let info = search_with_history(rep_board, rep_hist, 3);
        assert!(info.depth_completed > 1, "depth = {}", info.depth_completed);
        assert!(info.nodes > 100, "nodes = {}", info.nodes);
        assert!(!info.best.is_null());
        assert_eq!(info.best.to_uci(), base.best.to_uci());
        assert_eq!(info.score, base.score);
    }

    #[test]
    fn forced_triple_repetition_scores_zero() {
        // Weiß steht klar schlechter (Dame + Bauern minus), kann aber per Kh1
        // in eine Stellung zurück, die schon zweimal vorkam -> drittes
        // Auftreten = 0. Alternativzüge verlieren deutlich, also muss Kh1 mit
        // Score 0 gewählt werden. Mit nur einem früheren Auftreten (2.
        // insgesamt) gilt das nicht: dann muss der Score klar negativ sein.
        // Hinweis: Die Vorgeschichte ist synthetisch injiziert (derselbe Hash
        // zweimal), keine legal ausgespielte Wiederholungssequenz — getestet
        // wird die Zählschwelle.
        let q = Board::from_fen("3q1k2/5ppp/8/8/8/8/8/4R1K1 w - - 0 1").unwrap();
        let mut tmp = q.clone();
        let mut legal = Vec::new();
        tmp.gen_legal(&mut legal);
        let kh1 = legal.iter().find(|m| m.to_uci() == "g1h1").unwrap().clone();
        let u = tmp.make(&kh1);
        let p_hash = tmp.hash;
        let _ = u;

        for (hist, expect_draw) in [(vec![p_hash, p_hash], true), (vec![p_hash], false)] {
            let info = search_with_history(q.clone(), hist, 3);
            assert!(info.depth_completed > 1, "depth = {}", info.depth_completed);
            if expect_draw {
                assert_eq!(info.best.to_uci(), "g1h1");
                assert_eq!(info.score, 0, "score = {}", info.score);
            } else {
                assert!(info.score < -200, "second occurrence must not draw, score = {}", info.score);
            }
        }
    }

    #[test]
    fn reused_tt_still_searches_root() {
        // TT über Suchen hinweg wiederverwenden (wie die UCI-Schleife):
        // auch dann muss die Wurzel vollständig suchen (kein TT-Cutoff ohne
        // PV, kein Rückfall auf den erstgenerierten Zug).
        let (board, _) = play_startpos(&["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5"]);
        let tt = TransTable::new_mb(1);
        let stop = Arc::new(AtomicBool::new(false));
        let mut s = Searcher::new(board, tt, vec![], stop);
        let mut lim = SearchLimits::default();
        lim.depth = Some(4);
        let first = s.search(&lim, 0);
        assert_eq!(first.depth_completed, 4);
        let second = s.search(&lim, 0);
        assert_eq!(second.depth_completed, 4);
        assert!(second.nodes > 100, "nodes = {}", second.nodes);
        assert!(!second.pv.is_empty());
        assert_eq!(second.best.to_uci(), second.pv[0].to_uci());
    }
}
