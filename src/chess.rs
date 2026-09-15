// Funken chess core: board representation, move generation, make/unmake.
// Design (own implementation, standard techniques):
// - Mailbox board [u8; 64], squares a1=0 .. h8=63, file = sq & 7, rank = sq >> 3.
// - Pieces: 0 empty, 1..6 white P N B R Q K, 7..12 black P N B R Q K.
// - Pseudo-legal generation + full legality check (king safety after make).
//   This automatically covers tricky cases like en-passant pins.
// - Incremental Zobrist hashing (own SplitMix64 stream, fixed seed).

use std::sync::OnceLock;

pub const EMPTY: u8 = 0;
pub const WP: u8 = 1;
pub const WN: u8 = 2;
pub const WB: u8 = 3;
pub const WR: u8 = 4;
pub const WQ: u8 = 5;
pub const WK: u8 = 6;
pub const BP: u8 = 7;
pub const BN: u8 = 8;
pub const BB: u8 = 9;
pub const BR: u8 = 10;
pub const BQ: u8 = 11;
pub const BK: u8 = 12;

pub const WHITE: u8 = 0;
pub const BLACK: u8 = 1;

pub const NO_SQ: u8 = 64;

// Castling rights bits
pub const CR_WK: u8 = 1;
pub const CR_WQ: u8 = 2;
pub const CR_BK: u8 = 4;
pub const CR_BQ: u8 = 8;

#[inline]
pub fn color_of(p: u8) -> u8 {
    (p - 1) / 6
}

#[inline]
pub fn type_of(p: u8) -> u8 {
    (p - 1) % 6
}

// piece types: 0 pawn, 1 knight, 2 bishop, 3 rook, 4 queen, 5 king
#[inline]
pub fn make_piece(color: u8, ptype: u8) -> u8 {
    color * 6 + ptype + 1
}

#[inline]
pub fn file_of(sq: u8) -> i8 {
    (sq & 7) as i8
}

#[inline]
pub fn rank_of(sq: u8) -> i8 {
    (sq >> 3) as i8
}

#[inline]
pub fn sq_at(file: i8, rank: i8) -> u8 {
    (rank as u8) * 8 + (file as u8)
}

#[inline]
pub fn opp(color: u8) -> u8 {
    color ^ 1
}

// ---------------------------------------------------------------------------
// Zobrist hashing (own deterministic PRNG stream)
// ---------------------------------------------------------------------------

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub struct Zobrist {
    pub piece: [[u64; 64]; 13],
    pub side: u64,
    pub castle: [u64; 16],
    pub ep_file: [u64; 8],
}

static ZOBRIST: OnceLock<Zobrist> = OnceLock::new();

pub fn zobrist() -> &'static Zobrist {
    ZOBRIST.get_or_init(|| {
        let mut z = Zobrist {
            piece: [[0; 64]; 13],
            side: 0,
            castle: [0; 16],
            ep_file: [0; 8],
        };
        // placeholder replaced below by deterministic seed
        let mut st: u64 = 0x1234_5678_9ABC_DEF1;
        for p in 0..13 {
            for sq in 0..64 {
                z.piece[p][sq] = splitmix64(&mut st);
            }
        }
        z.side = splitmix64(&mut st);
        for i in 0..16 {
            z.castle[i] = splitmix64(&mut st);
        }
        for i in 0..8 {
            z.ep_file[i] = splitmix64(&mut st);
        }
        z
    })
}

// ---------------------------------------------------------------------------
// Move + Undo
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    /// 0 = none, else 1..4 = N/B/R/Q (knight..queen)
    pub promo: u8,
    pub capture: bool,
    pub double_push: bool,
    pub en_passant: bool,
    pub castle_k: bool,
    pub castle_q: bool,
}

impl Move {
    pub fn quiet(from: u8, to: u8) -> Move {
        Move { from, to, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: false, castle_q: false }
    }
    pub fn null() -> Move {
        Move { from: 0, to: 0, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: false, castle_q: false }
    }
    pub fn is_null(&self) -> bool {
        self.from == self.to && self.promo == 0 && !self.capture && !self.castle_k && !self.castle_q && !self.en_passant && !self.double_push
    }
    pub fn to_uci(&self) -> String {
        let mut s = String::with_capacity(5);
        s.push((b'a' + (self.from & 7)) as char);
        s.push((b'1' + (self.from >> 3)) as char);
        s.push((b'a' + (self.to & 7)) as char);
        s.push((b'1' + (self.to >> 3)) as char);
        if self.promo != 0 {
            s.push(match self.promo {
                1 => 'n',
                2 => 'b',
                3 => 'r',
                4 => 'q',
                _ => '?',
            });
        }
        s
    }
}

#[derive(Clone, Copy)]
pub struct Undo {
    pub captured: u8,
    pub castling: u8,
    pub ep: u8,
    pub half: u16,
    pub full: u16,
    pub hash: u64,
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Board {
    pub sq: [u8; 64],
    pub side: u8,
    pub castling: u8,
    pub ep: u8,
    pub half: u16,
    pub full: u16,
    pub hash: u64,
    pub king: [u8; 2],
}

impl Board {
    pub fn empty() -> Board {
        Board { sq: [EMPTY; 64], side: WHITE, castling: 0, ep: NO_SQ, half: 0, full: 1, hash: 0, king: [4, 60] }
    }

    pub fn startpos() -> Board {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn full_hash(&self) -> u64 {
        let z = zobrist();
        let mut h: u64 = 0;
        for s in 0..64 {
            let p = self.sq[s];
            if p != EMPTY {
                h ^= z.piece[p as usize][s];
            }
        }
        if self.side == BLACK {
            h ^= z.side;
        }
        h ^= z.castle[self.castling as usize];
        if self.ep != NO_SQ {
            h ^= z.ep_file[(self.ep & 7) as usize];
        }
        h
    }

    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let mut b = Board::empty();
        b.sq = [EMPTY; 64];
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err("FEN needs at least 4 fields".to_string());
        }
        let mut rank: i8 = 7;
        let mut file: i8 = 0;
        for c in parts[0].chars() {
            if c == '/' {
                if file != 8 {
                    return Err("FEN rank not complete".to_string());
                }
                rank -= 1;
                file = 0;
                if rank < 0 {
                    return Err("too many ranks".to_string());
                }
            } else if c.is_ascii_digit() {
                file += c.to_digit(10).unwrap() as i8;
                if file > 8 {
                    return Err("FEN file overflow".to_string());
                }
            } else {
                if file >= 8 {
                    return Err("FEN file overflow".to_string());
                }
                let p = match c {
                    'P' => WP, 'N' => WN, 'B' => WB, 'R' => WR, 'Q' => WQ, 'K' => WK,
                    'p' => BP, 'n' => BN, 'b' => BB, 'r' => BR, 'q' => BQ, 'k' => BK,
                    _ => return Err(format!("bad FEN char {c}")),
                };
                let s = sq_at(file, rank);
                b.sq[s as usize] = p;
                if p == WK {
                    b.king[0] = s;
                }
                if p == BK {
                    b.king[1] = s;
                }
                file += 1;
            }
        }
        b.side = match parts[1] {
            "w" => WHITE,
            "b" => BLACK,
            _ => return Err("bad side".to_string()),
        };
        b.castling = 0;
        if parts[2] != "-" {
            for c in parts[2].chars() {
                match c {
                    'K' => b.castling |= CR_WK,
                    'Q' => b.castling |= CR_WQ,
                    'k' => b.castling |= CR_BK,
                    'q' => b.castling |= CR_BQ,
                    _ => return Err("bad castling".to_string()),
                }
            }
        }
        b.ep = if parts[3] == "-" {
            NO_SQ
        } else {
            let ch: Vec<char> = parts[3].chars().collect();
            if ch.len() != 2 {
                return Err("bad ep".to_string());
            }
            let f = (ch[0] as i8) - ('a' as i8);
            let r = (ch[1] as i8) - ('1' as i8);
            if !(0..8).contains(&f) || !(0..8).contains(&r) {
                return Err("bad ep square".to_string());
            }
            sq_at(f, r)
        };
        b.half = if parts.len() > 4 { parts[4].parse().map_err(|_| "bad halfmove")? } else { 0 };
        b.full = if parts.len() > 5 { parts[5].parse().map_err(|_| "bad fullmove")? } else { 1 };
        b.hash = b.full_hash();
        Ok(b)
    }

    #[allow(dead_code)] // used in tests and debugging
    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for r in (0..8).rev() {
            let mut empty = 0;
            for f in 0..8 {
                let p = self.sq[(r * 8 + f) as usize];
                if p == EMPTY {
                    empty += 1;
                } else {
                    if empty > 0 {
                        s.push((b'0' + empty) as char);
                        empty = 0;
                    }
                    s.push(match p {
                        WP => 'P', WN => 'N', WB => 'B', WR => 'R', WQ => 'Q', WK => 'K',
                        BP => 'p', BN => 'n', BB => 'b', BR => 'r', BQ => 'q', BK => 'k',
                        _ => '?',
                    });
                }
            }
            if empty > 0 {
                s.push((b'0' + empty) as char);
            }
            if r > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.side == WHITE { 'w' } else { 'b' });
        s.push(' ');
        if self.castling == 0 {
            s.push('-');
        } else {
            if self.castling & CR_WK != 0 { s.push('K'); }
            if self.castling & CR_WQ != 0 { s.push('Q'); }
            if self.castling & CR_BK != 0 { s.push('k'); }
            if self.castling & CR_BQ != 0 { s.push('q'); }
        }
        s.push(' ');
        if self.ep == NO_SQ {
            s.push('-');
        } else {
            s.push((b'a' + (self.ep & 7)) as char);
            s.push((b'1' + (self.ep >> 3)) as char);
        }
        s.push_str(&format!(" {} {}", self.half, self.full));
        s
    }

    // --- attack detection -------------------------------------------------
    pub fn is_attacked(&self, target: u8, by: u8) -> bool {
        let f = file_of(target);
        let r = rank_of(target);
        // pawns
        if by == WHITE {
            let r2 = r - 1;
            if r2 >= 0 {
                if f - 1 >= 0 && self.sq[sq_at(f - 1, r2) as usize] == WP {
                    return true;
                }
                if f + 1 < 8 && self.sq[sq_at(f + 1, r2) as usize] == WP {
                    return true;
                }
            }
        } else {
            let r2 = r + 1;
            if r2 < 8 {
                if f - 1 >= 0 && self.sq[sq_at(f - 1, r2) as usize] == BP {
                    return true;
                }
                if f + 1 < 8 && self.sq[sq_at(f + 1, r2) as usize] == BP {
                    return true;
                }
            }
        }
        // knights
        let n = if by == WHITE { WN } else { BN };
        const KN: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
        for (df, dr) in KN {
            let ff = f + df;
            let rr = r + dr;
            if (0..8).contains(&ff) && (0..8).contains(&rr) && self.sq[sq_at(ff, rr) as usize] == n {
                return true;
            }
        }
        // king
        let k = if by == WHITE { WK } else { BK };
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                let ff = f + df;
                let rr = r + dr;
                if (0..8).contains(&ff) && (0..8).contains(&rr) && self.sq[sq_at(ff, rr) as usize] == k {
                    return true;
                }
            }
        }
        // sliders: orthogonal
        let rq = if by == WHITE { (WR, WQ) } else { (BR, BQ) };
        const ORTH: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for (df, dr) in ORTH {
            let mut ff = f + df;
            let mut rr = r + dr;
            while (0..8).contains(&ff) && (0..8).contains(&rr) {
                let p = self.sq[sq_at(ff, rr) as usize];
                if p != EMPTY {
                    if p == rq.0 || p == rq.1 {
                        return true;
                    }
                    break;
                }
                ff += df;
                rr += dr;
            }
        }
        // sliders: diagonal
        let bq = if by == WHITE { (WB, WQ) } else { (BB, BQ) };
        const DIAG: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        for (df, dr) in DIAG {
            let mut ff = f + df;
            let mut rr = r + dr;
            while (0..8).contains(&ff) && (0..8).contains(&rr) {
                let p = self.sq[sq_at(ff, rr) as usize];
                if p != EMPTY {
                    if p == bq.0 || p == bq.1 {
                        return true;
                    }
                    break;
                }
                ff += df;
                rr += dr;
            }
        }
        false
    }

    #[inline]
    pub fn in_check(&self, color: u8) -> bool {
        self.is_attacked(self.king[color as usize], opp(color))
    }

    // --- move generation ----------------------------------------------------
    fn push_pawn_moves(&self, out: &mut Vec<Move>, from: u8, to: u8, capture: bool) {
        let r = rank_of(to);
        let last = if self.side == WHITE { 7 } else { 0 };
        if r == last {
            for pr in 1..=4u8 {
                out.push(Move { from, to, promo: pr, capture, double_push: false, en_passant: false, castle_k: false, castle_q: false });
            }
        } else {
            out.push(Move { from, to, promo: 0, capture, double_push: false, en_passant: false, castle_k: false, castle_q: false });
        }
    }

    pub fn gen_pseudo(&self, out: &mut Vec<Move>) {
        out.clear();
        let us = self.side;
        let them = opp(us);
        let pawn = if us == WHITE { WP } else { BP };
        let knight = if us == WHITE { WN } else { BN };
        let bishop = if us == WHITE { WB } else { BB };
        let rook = if us == WHITE { WR } else { BR };
        let queen = if us == WHITE { WQ } else { BQ };
        let king = if us == WHITE { WK } else { BK };
        let up: i8 = if us == WHITE { 1 } else { -1 };
        let start_rank: i8 = if us == WHITE { 1 } else { 6 };

        for s in 0..64u8 {
            let p = self.sq[s as usize];
            if p == EMPTY || color_of(p) != us {
                continue;
            }
            let f = file_of(s);
            let r = rank_of(s);
            if p == pawn {
                // single push
                let r1 = r + up;
                if (0..8).contains(&r1) {
                    let t = sq_at(f, r1);
                    if self.sq[t as usize] == EMPTY {
                        self.push_pawn_moves(out, s, t, false);
                        // double push
                        if r == start_rank {
                            let r2 = r + 2 * up;
                            let t2 = sq_at(f, r2);
                            if self.sq[t2 as usize] == EMPTY {
                                out.push(Move { from: s, to: t2, promo: 0, capture: false, double_push: true, en_passant: false, castle_k: false, castle_q: false });
                            }
                        }
                    }
                    // captures
                    for df in [-1i8, 1] {
                        let ff = f + df;
                        if (0..8).contains(&ff) {
                            let t = sq_at(ff, r1);
                            let tp = self.sq[t as usize];
                            if tp != EMPTY && color_of(tp) == them {
                                self.push_pawn_moves(out, s, t, true);
                            }
                            // en passant (verified: an enemy pawn must stand
                            // next to the target; guards against bad FENs)
                            if t == self.ep {
                                let cap = if us == WHITE { t - 8 } else { t + 8 };
                                let ep_pawn = if us == WHITE { BP } else { WP };
                                if self.sq[cap as usize] == ep_pawn {
                                    out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: true, castle_k: false, castle_q: false });
                                }
                            }
                        }
                    }
                }
            } else if p == knight || p == king {
                const STEPS: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
                const KSTEPS: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
                let steps: &[(i8, i8)] = if p == knight { &STEPS } else { &KSTEPS };
                for (df, dr) in steps.iter() {
                    let ff = f + df;
                    let rr = r + dr;
                    if !(0..8).contains(&ff) || !(0..8).contains(&rr) {
                        continue;
                    }
                    let t = sq_at(ff, rr);
                    let tp = self.sq[t as usize];
                    if tp == EMPTY {
                        out.push(Move::quiet(s, t));
                    } else if color_of(tp) == them {
                        out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: false, castle_k: false, castle_q: false });
                    }
                }
                // castling
                if p == king {
                    if us == WHITE && s == 4 {
                        if self.castling & CR_WK != 0 && self.sq[5] == EMPTY && self.sq[6] == EMPTY {
                            if !self.is_attacked(4, them) && !self.is_attacked(5, them) && !self.is_attacked(6, them) {
                                out.push(Move { from: 4, to: 6, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: true, castle_q: false });
                            }
                        }
                        if self.castling & CR_WQ != 0 && self.sq[3] == EMPTY && self.sq[2] == EMPTY && self.sq[1] == EMPTY {
                            if !self.is_attacked(4, them) && !self.is_attacked(3, them) && !self.is_attacked(2, them) {
                                out.push(Move { from: 4, to: 2, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: false, castle_q: true });
                            }
                        }
                    }
                    if us == BLACK && s == 60 {
                        if self.castling & CR_BK != 0 && self.sq[61] == EMPTY && self.sq[62] == EMPTY {
                            if !self.is_attacked(60, them) && !self.is_attacked(61, them) && !self.is_attacked(62, them) {
                                out.push(Move { from: 60, to: 62, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: true, castle_q: false });
                            }
                        }
                        if self.castling & CR_BQ != 0 && self.sq[59] == EMPTY && self.sq[58] == EMPTY && self.sq[57] == EMPTY {
                            if !self.is_attacked(60, them) && !self.is_attacked(59, them) && !self.is_attacked(58, them) {
                                out.push(Move { from: 60, to: 58, promo: 0, capture: false, double_push: false, en_passant: false, castle_k: false, castle_q: true });
                            }
                        }
                    }
                }
            } else {
                // sliders
                let dirs: &[(i8, i8)] = if p == bishop {
                    &[(1, 1), (1, -1), (-1, 1), (-1, -1)]
                } else if p == rook {
                    &[(1, 0), (-1, 0), (0, 1), (0, -1)]
                } else {
                    debug_assert_eq!(p, queen);
                    &[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]
                };
                for (df, dr) in dirs.iter() {
                    let mut ff = f + df;
                    let mut rr = r + dr;
                    while (0..8).contains(&ff) && (0..8).contains(&rr) {
                        let t = sq_at(ff, rr);
                        let tp = self.sq[t as usize];
                        if tp == EMPTY {
                            out.push(Move::quiet(s, t));
                        } else {
                            if color_of(tp) == them {
                                out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: false, castle_k: false, castle_q: false });
                            }
                            break;
                        }
                        ff += df;
                        rr += dr;
                    }
                }
            }
        }
    }

    /// Pseudo-legal captures (+ promotions, which always capture or push to last rank).
    /// Used by quiescence search.
    pub fn gen_captures(&self, out: &mut Vec<Move>) {
        out.clear();
        let us = self.side;
        let them = opp(us);
        let up: i8 = if us == WHITE { 1 } else { -1 };
        for s in 0..64u8 {
            let p = self.sq[s as usize];
            if p == EMPTY || color_of(p) != us {
                continue;
            }
            let f = file_of(s);
            let r = rank_of(s);
            let pt = type_of(p);
            if pt == 0 {
                let r1 = r + up;
                if !(0..8).contains(&r1) {
                    continue;
                }
                // capture promotions and normal captures
                for df in [-1i8, 1] {
                    let ff = f + df;
                    if !(0..8).contains(&ff) {
                        continue;
                    }
                    let t = sq_at(ff, r1);
                    let tp = self.sq[t as usize];
                    if tp != EMPTY && color_of(tp) == them {
                        self.push_pawn_moves(out, s, t, true);
                    }
                    if t == self.ep {
                        let cap = if us == WHITE { t - 8 } else { t + 8 };
                        let ep_pawn = if us == WHITE { BP } else { WP };
                        if self.sq[cap as usize] == ep_pawn {
                            out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: true, castle_k: false, castle_q: false });
                        }
                    }
                }
                // push promotions (quiet promotion still tactical)
                let t = sq_at(f, r1);
                if self.sq[t as usize] == EMPTY {
                    let last = if us == WHITE { 7 } else { 0 };
                    if r1 == last {
                        self.push_pawn_moves(out, s, t, false);
                    }
                }
            } else if pt == 1 || pt == 5 {
                const STEPS: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
                const KSTEPS: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
                let steps: &[(i8, i8)] = if pt == 1 { &STEPS } else { &KSTEPS };
                for (df, dr) in steps.iter() {
                    let ff = f + df;
                    let rr = r + dr;
                    if !(0..8).contains(&ff) || !(0..8).contains(&rr) {
                        continue;
                    }
                    let t = sq_at(ff, rr);
                    let tp = self.sq[t as usize];
                    if tp != EMPTY && color_of(tp) == them {
                        out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: false, castle_k: false, castle_q: false });
                    }
                }
            } else {
                let dirs: &[(i8, i8)] = if pt == 2 {
                    &[(1, 1), (1, -1), (-1, 1), (-1, -1)]
                } else if pt == 3 {
                    &[(1, 0), (-1, 0), (0, 1), (0, -1)]
                } else {
                    &[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]
                };
                for (df, dr) in dirs.iter() {
                    let mut ff = f + df;
                    let mut rr = r + dr;
                    while (0..8).contains(&ff) && (0..8).contains(&rr) {
                        let t = sq_at(ff, rr);
                        let tp = self.sq[t as usize];
                        if tp != EMPTY {
                            if color_of(tp) == them {
                                out.push(Move { from: s, to: t, promo: 0, capture: true, double_push: false, en_passant: false, castle_k: false, castle_q: false });
                            }
                            break;
                        }
                        ff += df;
                        rr += dr;
                    }
                }
            }
        }
    }

    // --- make / unmake ------------------------------------------------------
    pub fn make(&mut self, m: &Move) -> Undo {
        let z = zobrist();
        let u = Undo { captured: EMPTY, castling: self.castling, ep: self.ep, half: self.half, full: self.full, hash: self.hash };
        let us = self.side;
        let them = opp(us);

        // remove old ep + castling from hash
        if self.ep != NO_SQ {
            self.hash ^= z.ep_file[(self.ep & 7) as usize];
        }
        self.hash ^= z.castle[self.castling as usize];

        let mut moving = self.sq[m.from as usize];
        debug_assert!(moving != EMPTY && color_of(moving) == us);
        // Captured piece and the square it stood on. For quiet moves there is
        // no captured piece; corner-rights clearing must then NOT trigger on
        // the target square (e.g. a knight moving onto an empty a1).
        let captured: u8;
        let mut cap_sq = NO_SQ;

        if m.en_passant {
            cap_sq = if us == WHITE { m.to - 8 } else { m.to + 8 };
            captured = self.sq[cap_sq as usize];
            debug_assert!(captured == if us == WHITE { BP } else { WP });
            self.hash ^= z.piece[captured as usize][cap_sq as usize];
            self.sq[cap_sq as usize] = EMPTY;
        } else {
            captured = self.sq[m.to as usize];
            if captured != EMPTY {
                cap_sq = m.to;
                self.hash ^= z.piece[captured as usize][m.to as usize];
            }
        }

        // move the piece
        self.hash ^= z.piece[moving as usize][m.from as usize];
        self.sq[m.from as usize] = EMPTY;

        if m.promo != 0 {
            // promo encoding 1..4 = N/B/R/Q matches piece types 1..4.
            moving = make_piece(us, m.promo);
        }
        self.hash ^= z.piece[moving as usize][m.to as usize];
        self.sq[m.to as usize] = moving;

        // castling rook
        if m.castle_k {
            if us == WHITE {
                self.hash ^= z.piece[WR as usize][7];
                self.hash ^= z.piece[WR as usize][5];
                self.sq[7] = EMPTY;
                self.sq[5] = WR;
            } else {
                self.hash ^= z.piece[BR as usize][63];
                self.hash ^= z.piece[BR as usize][61];
                self.sq[63] = EMPTY;
                self.sq[61] = BR;
            }
        } else if m.castle_q {
            if us == WHITE {
                self.hash ^= z.piece[WR as usize][0];
                self.hash ^= z.piece[WR as usize][3];
                self.sq[0] = EMPTY;
                self.sq[3] = WR;
            } else {
                self.hash ^= z.piece[BR as usize][56];
                self.hash ^= z.piece[BR as usize][59];
                self.sq[56] = EMPTY;
                self.sq[59] = BR;
            }
        }

        // update king square
        if type_of(moving) == 5 {
            self.king[us as usize] = m.to;
        }

        // castling rights
        let mut cr = self.castling;
        // king moved
        if type_of(moving) == 5 {
            cr &= if us == WHITE { !(CR_WK | CR_WQ) } else { !(CR_BK | CR_BQ) };
        }
        // rook moved from / rook captured on its home square.
        // Clearing is unconditional per square touched: if rights were already
        // gone the bit is already 0, so this stays correct.
        let corners = [(0u8, CR_WQ), (7, CR_WK), (56, CR_BQ), (63, CR_BK)];
        for (sqc, bit) in corners {
            if m.from == sqc || cap_sq == sqc {
                cr &= !bit;
            }
        }
        self.castling = cr;
        self.hash ^= z.castle[self.castling as usize];

        // ep square
        self.ep = NO_SQ;
        if m.double_push {
            self.ep = if us == WHITE { m.from + 8 } else { m.from - 8 };
            self.hash ^= z.ep_file[(self.ep & 7) as usize];
        }

        // clocks
        if type_of(moving) == 0 || captured != EMPTY {
            self.half = 0;
        } else {
            self.half += 1;
        }
        if us == BLACK {
            self.full += 1;
        }

        // side
        self.hash ^= z.side;
        self.side = them;

        let mut uu = u;
        uu.captured = captured;
        uu
    }

    pub fn unmake(&mut self, m: &Move, u: &Undo) {
        let us = opp(self.side);
        // restore piece from target
        let mut moving = self.sq[m.to as usize];
        self.sq[m.to as usize] = EMPTY;
        if m.promo != 0 {
            moving = make_piece(us, 0);
        }
        self.sq[m.from as usize] = moving;
        if type_of(moving) == 5 {
            self.king[us as usize] = m.from;
        }
        // restore rook (castling)
        if m.castle_k {
            if us == WHITE {
                self.sq[7] = WR;
                self.sq[5] = EMPTY;
            } else {
                self.sq[63] = BR;
                self.sq[61] = EMPTY;
            }
        } else if m.castle_q {
            if us == WHITE {
                self.sq[0] = WR;
                self.sq[3] = EMPTY;
            } else {
                self.sq[56] = BR;
                self.sq[59] = EMPTY;
            }
        }
        // restore captured
        if m.en_passant {
            let cap_sq = if us == WHITE { m.to - 8 } else { m.to + 8 };
            self.sq[cap_sq as usize] = u.captured;
        } else if u.captured != EMPTY {
            self.sq[m.to as usize] = u.captured;
        }
        self.side = us;
        self.castling = u.castling;
        self.ep = u.ep;
        self.half = u.half;
        self.full = u.full;
        self.hash = u.hash;
    }

    pub fn gen_legal(&mut self, out: &mut Vec<Move>) {
        let mut pseudo = Vec::with_capacity(64);
        self.gen_pseudo(&mut pseudo);
        out.clear();
        for m in pseudo {
            let u = self.make(&m);
            let us = opp(self.side);
            let ok = !self.is_attacked(self.king[us as usize], self.side);
            self.unmake(&m, &u);
            if ok {
                out.push(m);
            }
        }
    }

    // --- game end / draw helpers -------------------------------------------
    /// Insufficient material (automatic-draw cases): K-K, K-minor vs K,
    /// K+B vs K+B with bishops on same square color.
    pub fn insufficient_material(&self) -> bool {
        let mut minors = 0u32;
        let mut knights = 0u32;
        let mut bishop_colors = 0u32; // bitmask of square colors with bishops
        for s in 0..64 {
            let p = self.sq[s];
            if p == EMPTY || type_of(p) == 5 {
                continue;
            }
            match type_of(p) {
                0 | 3 | 4 => return false, // pawn, rook, queen can mate
                1 => {
                    knights += 1;
                    minors += 1;
                }
                2 => {
                    minors += 1;
                    bishop_colors |= 1 << (((s >> 3) + (s & 7)) & 1) as u32;
                }
                _ => return false,
            }
        }
        if minors <= 1 {
            return true;
        }
        // only bishops, all on the same color
        if knights == 0 && (bishop_colors == 1 || bishop_colors == 2) {
            return true;
        }
        false
    }

    // --- perft ----------------------------------------------------------------
    pub fn perft(&mut self, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut pseudo = Vec::with_capacity(64);
        self.gen_pseudo(&mut pseudo);
        let mut nodes = 0u64;
        for m in pseudo {
            let u = self.make(&m);
            let us = opp(self.side);
            if !self.is_attacked(self.king[us as usize], self.side) {
                nodes += if depth == 1 { 1 } else { self.perft(depth - 1) };
            }
            self.unmake(&m, &u);
        }
        nodes
    }
}

// Piece values (centipawns) shared by eval + move ordering.
pub const PIECE_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 20000];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_fen_roundtrip() {
        let b = Board::startpos();
        assert_eq!(b.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn hash_stable_under_make_unmake() {
        let mut b = Board::startpos();
        let h0 = b.hash;
        assert_eq!(h0, b.full_hash());
        let mut moves = Vec::new();
        b.gen_pseudo(&mut moves);
        for m in moves.iter().take(40) {
            let u = b.make(m);
            assert_eq!(b.hash, b.full_hash(), "incremental hash mismatch after {}", m.to_uci());
            b.unmake(m, &u);
            assert_eq!(b.hash, h0);
        }
    }

    #[test]
    fn perft_startpos() {
        let mut b = Board::startpos();
        assert_eq!(b.perft(1), 20);
        assert_eq!(b.perft(2), 400);
        assert_eq!(b.perft(3), 8902);
    }

    #[test]
    fn ep_pin_is_illegal() {
        // White pawn g5 is pinned horizontally by the black rook a5 against
        // the white king h5. Capturing en passant (g5xf6) would uncover the
        // rank, so it must not appear among legal moves -- and neither may
        // any other pawn move off the 5th rank.
        let mut b = Board::from_fen("4k3/8/8/r4pPK/8/8/8/8 w - f6 0 1").unwrap();
        let mut legal = Vec::new();
        b.gen_legal(&mut legal);
        assert!(!legal.is_empty());
        let ucis: Vec<String> = legal.iter().map(|m| m.to_uci()).collect();
        // The e.p. capture uncovers the 5th rank (g5 is vacated and the
        // block on f5 is removed) -> illegal and must be absent.
        assert!(!ucis.contains(&"g5f6".to_string()), "ep capture must be illegal: {ucis:?}");
        // g5-g6 keeps the f5 blocker -> legal. (Note: black pawns capture
        // downwards, so h5-g6 is fine but h5-g4 would walk into f5's attack.)
        assert!(ucis.contains(&"g5g6".to_string()));
        assert!(ucis.contains(&"h5g6".to_string()));
        assert!(!ucis.contains(&"h5g4".to_string()));
        // ... but the same capture is fine without the pin:
        let mut b2 = Board::from_fen("4k3/8/8/5pPK/8/8/8/8 w - f6 0 1").unwrap();
        let mut l2 = Vec::new();
        b2.gen_legal(&mut l2);
        assert!(l2.iter().any(|m| m.to_uci() == "g5f6"));
    }

    #[test]
    fn castling_through_check() {
        // Black bishop c4 attacks f1: white may not castle kingside, but
        // queenside castling stays legal.
        let mut b = Board::from_fen("4k3/8/8/8/2b5/8/8/R3K2R w KQ - 0 1").unwrap();
        let mut legal = Vec::new();
        b.gen_legal(&mut legal);
        let ucis: Vec<String> = legal.iter().map(|m| m.to_uci()).collect();
        assert!(!ucis.contains(&"e1g1".to_string()));
        assert!(ucis.contains(&"e1c1".to_string()));
    }

    #[test]
    fn mate_and_stalemate() {
        // Fool's-mate finish, white to move and mated.
        let mut m = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 0 1").unwrap();
        let mut lm = Vec::new();
        m.gen_legal(&mut lm);
        assert!(lm.is_empty());
        assert!(m.in_check(WHITE));
        // Stalemate: black to move, not in check, no moves.
        let mut s = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let mut ls = Vec::new();
        s.gen_legal(&mut ls);
        assert!(ls.is_empty());
        assert!(!s.in_check(BLACK));
    }

    #[test]
    fn insufficient_material_cases() {
        assert!(Board::from_fen("8/8/4k3/8/8/8/8/4K3 w - - 0 1").unwrap().insufficient_material());
        assert!(Board::from_fen("8/8/4k3/8/8/3B4/8/4K3 w - - 0 1").unwrap().insufficient_material());
        assert!(Board::from_fen("8/8/4k3/8/8/3N4/8/4K3 w - - 0 1").unwrap().insufficient_material());
        // Bishops on the same square color only -> draw (c1 + f4 are both dark).
        assert!(Board::from_fen("5k2/8/8/8/5B2/8/8/2B3K1 w - - 0 1").unwrap().insufficient_material());
        // Bishops on opposite colors (f2 dark, h1 light) -> mate possible.
        assert!(!Board::from_fen("5k2/8/8/8/8/8/5B2/5K1B w - - 0 1").unwrap().insufficient_material());
        assert!(!Board::startpos().insufficient_material());
    }

    #[test]
    fn promotions_generated() {
        // White pawn a7 must have 4 promotion moves (quiet + capture).
        let mut b = Board::from_fen("1r5k/P6p/8/8/8/8/6PP/6K1 w - - 0 1").unwrap();
        let mut legal = Vec::new();
        b.gen_legal(&mut legal);
        let promos: Vec<String> = legal.iter().filter(|m| m.promo != 0).map(|m| m.to_uci()).collect();
        assert_eq!(promos.len(), 8); // 4x a7a8, 4x a7xb8
    }
}
