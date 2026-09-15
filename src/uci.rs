// Funken UCI interface: protocol loop, position handling, time-managed
// search in a worker thread (so `stop` stays responsive), options.
//
// Single-threaded search; the `Threads` option is accepted but fixed at 1
// (documented in README/KONZEPT). No pondering.

use std::io::{self, BufRead};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use crate::chess::*;
use crate::search::*;


/// Print a UCI line and flush immediately (stdout is block-buffered on pipes).
fn emit(line: String) {
    use std::io::Write;
    print!("{line}\n");
    let _ = std::io::stdout().flush();
}

pub const ENGINE_NAME: &str = "Funken 1.0";
pub const ENGINE_AUTHOR: &str = "Muse Spark";

pub struct UciOptions {
    pub hash_mb: usize,
    pub move_overhead_ms: u64,
}

impl Default for UciOptions {
    fn default() -> Self {
        UciOptions { hash_mb: 64, move_overhead_ms: 100 }
    }
}

struct Position {
    board: Board,
    /// Hashes of all game positions *before* the current one (for repetition).
    history: Vec<u64>,
}

impl Position {
    fn startpos() -> Position {
        Position { board: Board::startpos(), history: vec![] }
    }
}

fn parse_uci_move(board: &mut Board, s: &str) -> Option<Move> {
    if s.len() < 4 {
        return None;
    }
    let b = s.as_bytes();
    let ff = (b[0] as i8) - ('a' as i8);
    let rf = (b[1] as i8) - ('1' as i8);
    let tf = (b[2] as i8) - ('a' as i8);
    let rt = (b[3] as i8) - ('1' as i8);
    if !(0..8).contains(&ff) || !(0..8).contains(&rf) || !(0..8).contains(&tf) || !(0..8).contains(&rt) {
        return None;
    }
    let from = sq_at(ff, rf);
    let to = sq_at(tf, rt);
    let promo = if s.len() >= 5 {
        match b[4] as char {
            'n' => 1,
            'b' => 2,
            'r' => 3,
            'q' => 4,
            _ => return None,
        }
    } else {
        0
    };
    let mut legal = Vec::new();
    board.gen_legal(&mut legal);
    legal.into_iter().find(|m| m.from == from && m.to == to && m.promo == promo)
}

fn apply_position(cmd: &str) -> Option<Position> {
    // "position startpos [moves ...]" | "position fen <fen> [moves ...]"
    let t = cmd.trim();
    let rest = t.strip_prefix("position")?.trim();
    let mut pos = if let Some(r) = rest.strip_prefix("startpos") {
        let mut p = Position::startpos();
        let _ = r;
        p.board = Board::startpos();
        p
    } else if let Some(r) = rest.strip_prefix("fen") {
        let r = r.trim();
        // FEN is 6 fields; "moves" may follow.
        let mut parts: Vec<&str> = r.split_whitespace().collect();
        let mut moves_idx: Option<usize> = None;
        for (i, w) in parts.iter().enumerate() {
            if *w == "moves" {
                moves_idx = Some(i);
                break;
            }
        }
        let (fen_fields, move_words): (Vec<&str>, Vec<&str>) = match moves_idx {
            Some(i) => (parts[..i].to_vec(), parts[i + 1..].to_vec()),
            None => (parts.clone(), vec![]),
        };
        let _ = &mut parts;
        if fen_fields.len() < 4 {
            return None;
        }
        let fen = fen_fields.join(" ");
        let board = Board::from_fen(&fen).ok()?;
        let mut p = Position { board, history: vec![] };
        for mw in move_words {
            p.history.push(p.board.hash);
            let m = parse_uci_move(&mut p.board, mw)?;
            let u = p.board.make(&m);
            let us = opp(p.board.side);
            if p.board.is_attacked(p.board.king[us as usize], p.board.side) {
                p.board.unmake(&m, &u);
                p.history.pop();
                return None;
            }
            let _ = u;
        }
        return Some(p);
    } else {
        return None;
    };
    // startpos + optional moves
    let words: Vec<&str> = rest.split_whitespace().collect();
    let mut i = 1; // skip "startpos"
    if words.get(i) == Some(&"moves") {
        i += 1;
        while let Some(mw) = words.get(i) {
            pos.history.push(pos.board.hash);
            let m = parse_uci_move(&mut pos.board, mw)?;
            let u = pos.board.make(&m);
            let us = opp(pos.board.side);
            if pos.board.is_attacked(pos.board.king[us as usize], pos.board.side) {
                pos.board.unmake(&m, &u);
                pos.history.pop();
                return None;
            }
            let _ = u;
            i += 1;
        }
    }
    Some(pos)
}

fn parse_go(words: &[&str]) -> (SearchLimits, Vec<String>) {
    let mut lim = SearchLimits::default();
    let mut searchmoves: Vec<String> = vec![];
    let mut i = 0;
    // first word is "go"
    if words.first() == Some(&"go") {
        i = 1;
    }
    while i < words.len() {
        match words[i] {
            "wtime" => {
                i += 1;
                lim.wtime = words.get(i).and_then(|v| v.parse().ok());
            }
            "btime" => {
                i += 1;
                lim.btime = words.get(i).and_then(|v| v.parse().ok());
            }
            "winc" => {
                i += 1;
                lim.winc = words.get(i).and_then(|v| v.parse().ok());
            }
            "binc" => {
                i += 1;
                lim.binc = words.get(i).and_then(|v| v.parse().ok());
            }
            "movestogo" => {
                i += 1;
                lim.movestogo = words.get(i).and_then(|v| v.parse().ok());
            }
            "movetime" => {
                i += 1;
                lim.movetime = words.get(i).and_then(|v| v.parse().ok());
            }
            "depth" => {
                i += 1;
                lim.depth = words.get(i).and_then(|v| v.parse().ok());
            }
            "nodes" => {
                i += 1;
                lim.nodes = words.get(i).and_then(|v| v.parse().ok());
            }
            "infinite" => {
                lim.infinite = true;
            }
            "ponder" => {
                // No ponder support: search until stopped.
                lim.infinite = true;
            }
            "searchmoves" => {
                i += 1;
                while i < words.len() {
                    searchmoves.push(words[i].to_string());
                    i += 1;
                }
                break;
            }
            _ => {}
        }
        i += 1;
    }
    (lim, searchmoves)
}

fn score_to_uci(score: i32) -> String {
    if score > MATE - 1000 {
        let plies = MATE - score;
        let moves = (plies + 1) / 2;
        format!("mate {moves}")
    } else if score < -MATE + 1000 {
        let plies = MATE + score;
        let moves = -((plies + 1) / 2);
        format!("mate {moves}")
    } else {
        format!("cp {score}")
    }
}

fn pv_to_string(pv: &[Move]) -> String {
    pv.iter().map(|m| m.to_uci()).collect::<Vec<_>>().join(" ")
}

struct SearchRequest {
    board: Board,
    history: Vec<u64>,
    limits: SearchLimits,
    overhead: u64,
    stop: Arc<AtomicBool>,
}

struct SearchResponse {
    info: SearchInfo,
    tt: TransTable,
}

pub struct IterInfo {
    pub depth: u8,
    pub score: i32,
    pub nodes: u64,
    pub seldepth: u32,
    pub ms: u64,
    pub pv: Vec<Move>,
}

fn run_search_thread(req: SearchRequest, tt: TransTable, info_tx: Sender<IterInfo>, done_tx: Sender<SearchResponse>) {
    let SearchRequest { board, history, limits, overhead, stop } = req;
    let mut searcher = Searcher::new(board, tt, history, stop);
    let t0 = Instant::now();
    let mut cb = |depth: u8, score: i32, nodes: u64, seldepth: u32, pv: &[Move]| {
        let ms = t0.elapsed().as_millis() as u64;
        let _ = info_tx.send(IterInfo { depth, score, nodes, seldepth, ms, pv: pv.to_vec() });
    };
    let info = searcher.search_report(&limits, overhead, &mut cb);
    let tt_back = std::mem::replace(&mut searcher.tt, TransTable::new_mb(1));
    let _ = done_tx.send(SearchResponse { info, tt: tt_back });
}

pub fn uci_loop() {
    let stdin = io::stdin();
    //Pump stdin lines into a channel so the main loop can poll while searching.
    let (cmd_tx, cmd_rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    std::thread::spawn(move || {
        let lock = stdin.lock();
        for line in lock.lines() {
            match line {
                Ok(l) => {
                    if cmd_tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut pos = Position::startpos();
    let mut opts = UciOptions::default();
    let mut tt = TransTable::new_mb(opts.hash_mb);

    // searching state
    let mut searching = false;
    let mut stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let mut done_rx: Option<Receiver<SearchResponse>> = None;
    let mut info_rx: Option<Receiver<IterInfo>> = None;
    let mut ready_pending = false;

    emit(format!("{ENGINE_NAME} by {ENGINE_AUTHOR}"));

    loop {
        // Poll for search completion / info lines first.
        if searching {
            if let Some(rx) = info_rx.as_ref() {
                while let Ok(it) = rx.try_recv() {
                    let nps = if it.ms > 0 { it.nodes * 1000 / it.ms } else { it.nodes };
                    emit(format!(
                        "info depth {} seldepth {} score {} nodes {} nps {} time {} pv {}",
                        it.depth,
                        it.seldepth,
                        score_to_uci(it.score),
                        it.nodes,
                        nps,
                        it.ms,
                        pv_to_string(&it.pv)
                    ));
                }
            }
            if let Some(rx) = done_rx.as_ref() {
                if let Ok(resp) = rx.try_recv() {
                    searching = false;
                    done_rx = None;
                    info_rx = None;
                    tt = resp.tt;
                    let info = resp.info;
                    let nps = if info.elapsed_ms > 0 { info.nodes * 1000 / info.elapsed_ms } else { info.nodes };
                    if info.depth_completed > 0 {
                        emit(format!(
                            "info depth {} seldepth {} score {} nodes {} nps {} time {} hashfull {} pv {}",
                            info.depth_completed,
                            info.seldepth,
                            score_to_uci(info.score),
                            info.nodes,
                            nps,
                            info.elapsed_ms,
                            tt.hashfull(),
                            pv_to_string(&info.pv)
                        ));
                    }
                    let bm = if info.best.is_null() { "0000".to_string() } else { info.best.to_uci() };
                    emit(format!("bestmove {bm}"));
                    if ready_pending {
                        ready_pending = false;
                        emit("readyok".to_string());
                    }
                }
            }
        }

        let line = if searching {
            match cmd_rx.recv_timeout(std::time::Duration::from_millis(5)) {
                Ok(l) => l,
                Err(_) => continue,
            }
        } else {
            match cmd_rx.recv() {
                Ok(l) => l,
                Err(_) => break,
            }
        };
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        let word = cmd.split_whitespace().next().unwrap_or("");

        match word {
            "uci" => {
                emit(format!("id name {ENGINE_NAME}"));
                emit(format!("id author {ENGINE_AUTHOR}"));
                emit("option name Hash type spin default 64 min 1 max 1024".to_string());
                emit("option name Move Overhead type spin default 100 min 0 max 5000".to_string());
                emit("option name Threads type spin default 1 min 1 max 1".to_string());
                emit("uciok".to_string());
            }
            "debug" => {}
            "isready" => {
                // Deferred while searching (answered when the search ends).
                if searching {
                    ready_pending = true;
                } else {
                    emit("readyok".to_string());
                }
            }
            "setoption" => {
                // "setoption name <name> [value <v>]"; only applied when idle.
                if searching {
                    emit("info string setoption deferred until search finishes".to_string());
                    continue;
                }
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let mut name = String::new();
                let mut value = String::new();
                let mut i = 1;
                if parts.get(i) == Some(&"name") {
                    i += 1;
                    while i < parts.len() && parts[i] != "value" {
                        if !name.is_empty() {
                            name.push(' ');
                        }
                        name.push_str(parts[i]);
                        i += 1;
                    }
                    if parts.get(i) == Some(&"value") {
                        i += 1;
                        while i < parts.len() {
                            if !value.is_empty() {
                                value.push(' ');
                            }
                            value.push_str(parts[i]);
                            i += 1;
                        }
                    }
                }
                // Option names are matched case- and space-insensitively:
                // GUIs/bridges disagree on "Move Overhead" vs "MoveOverhead".
                let lname: String = name.chars().filter(|c| *c != ' ').collect::<String>().to_lowercase();
                match lname.as_str() {
                    "hash" => {
                        if let Ok(mb) = value.parse::<usize>() {
                            opts.hash_mb = mb.clamp(1, 1024);
                            tt = TransTable::new_mb(opts.hash_mb);
                        }
                    }
                    "moveoverhead" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            opts.move_overhead_ms = ms.min(5000);
                        }
                    }
                    "threads" => {
                        if value.parse::<u32>().unwrap_or(1) != 1 {
                            emit("info string Funken is single-threaded; Threads stays 1".to_string());
                        }
                    }
                    _ => {}
                }
            }
            "ucinewgame" => {
                if !searching {
                    tt.clear();
                }
            }
            "position" => {
                if searching {
                    continue;
                }
                match apply_position(cmd) {
                    Some(p) => pos = p,
                    None => emit("info string illegal position command".to_string()),
                }
            }
            "go" => {
                if searching {
                    continue;
                }
                let words: Vec<&str> = cmd.split_whitespace().collect();
                let (lim, searchmoves) = parse_go(&words);
                // Optional root restriction (`searchmoves`): resolve to legal
                // moves; enforced inside the worker's root node.
                let allow: Option<Vec<Move>> = if searchmoves.is_empty() {
                    None
                } else {
                    let mut legal = Vec::new();
                    pos.board.gen_legal(&mut legal);
                    let mut v = vec![];
                    for sm in searchmoves {
                        let mut tmp = pos.board.clone();
                        if let Some(m) = parse_uci_move(&mut tmp, &sm) {
                            if legal.iter().any(|x| x.from == m.from && x.to == m.to && x.promo == m.promo) {
                                v.push(m);
                            }
                        }
                    }
                    Some(v)
                };
                if let Some(a) = allow.as_ref() {
                    if a.is_empty() {
                        emit("info string no legal searchmoves".to_string());
                        continue;
                    }
                    if a.len() == 1 {
                        emit(format!("bestmove {}", a[0].to_uci()));
                        continue;
                    }
                }
                stop_flag = Arc::new(AtomicBool::new(false));
                let (info_tx, info_rx_new) = mpsc::channel();
                let (done_tx, done_rx_new) = mpsc::channel();
                info_rx = Some(info_rx_new);
                done_rx = Some(done_rx_new);
                let req = SearchRequest {
                    board: pos.board.clone(),
                    history: pos.history.clone(),
                    limits: lim,
                    overhead: opts.move_overhead_ms,
                    stop: stop_flag.clone(),
                };
                // Allow-list: enforced as the root move set in the worker.
                let tt_moved = std::mem::replace(&mut tt, TransTable::new_mb(1));
                if let Some(a) = allow {
                    std::thread::spawn(move || {
                        run_search_thread_restricted(req, a, tt_moved, info_tx, done_tx);
                    });
                } else {
                    std::thread::spawn(move || {
                        run_search_thread(req, tt_moved, info_tx, done_tx);
                    });
                }
                searching = true;
            }
            "stop" => {
                if searching {
                    stop_flag.store(true, Ordering::Relaxed);
                }
            }
            "ponderhit" => {
                // No ponder: nothing to do (go ponder == infinite search).
            }
            "quit" => {
                if searching {
                    stop_flag.store(true, Ordering::Relaxed);
                    // give the worker a moment; then exit regardless.
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                break;
            }
            _ => {
                // Unknown commands are ignored per UCI spec.
            }
        }
    }
}

fn run_search_thread_restricted(
    req: SearchRequest,
    allow: Vec<Move>,
    tt: TransTable,
    info_tx: Sender<IterInfo>,
    done_tx: Sender<SearchResponse>,
) {
    // Proper root restriction: run iterative deepening where the root move
    // loop only considers `allow`. Implemented via a dedicated root search
    // here (negamax core reused for non-root nodes).
    let SearchRequest { board, history, limits, overhead, stop } = req;
    let mut searcher = Searcher::new(board, tt, history, stop);
    let t0 = Instant::now();
    let mut cb = |depth: u8, score: i32, nodes: u64, seldepth: u32, pv: &[Move]| {
        let ms = t0.elapsed().as_millis() as u64;
        let _ = info_tx.send(IterInfo { depth, score, nodes, seldepth, ms, pv: pv.to_vec() });
    };
    let info = searcher.search_root_restricted(&limits, overhead, &allow, &mut cb);
    let tt_back = std::mem::replace(&mut searcher.tt, TransTable::new_mb(1));
    let _ = done_tx.send(SearchResponse { info, tt: tt_back });
}
