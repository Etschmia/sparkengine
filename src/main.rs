// Funken - eigenstaendige UCI-Schachengine.
// Nutzung: `funken` (UCI), `funken perft <tiefe> [fen]`, `funken bench`.

mod chess;
mod eval;
mod search;
mod tune;
mod uci;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use chess::Board;
use search::{SearchLimits, Searcher, TransTable};

fn cmd_perft(depth: u32, fen: Option<&str>) {
    let mut b = match fen {
        Some(f) => Board::from_fen(f).expect("ungueltiges FEN"),
        None => Board::startpos(),
    };
    for d in 1..=depth {
        let t0 = Instant::now();
        let n = b.perft(d);
        let ms = t0.elapsed().as_millis().max(1) as u64;
        println!("perft({d}) = {n}  [{ms} ms, {} nps]", n * 1000 / ms);
    }
}

fn cmd_bench() {
    // Reproduzierbarer Mini-Benchmark: feste Tiefen, feste Stellungen.
    let positions = [
        ("startpos", Board::startpos()),
        (
            "kiwipete",
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap(),
        ),
        (
            "pos3",
            Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap(),
        ),
    ];
    let stop = Arc::new(AtomicBool::new(false));
    for (name, b) in positions {
        let tt = TransTable::new_mb(16);
        let mut s = Searcher::new(b, tt, vec![], stop.clone());
        let mut lim = SearchLimits::default();
        lim.depth = Some(8);
        let t0 = Instant::now();
        let info = s.search(&lim, 0);
        let ms = t0.elapsed().as_millis().max(1) as u64;
        let pv: Vec<String> = info.pv.iter().map(|m| m.to_uci()).collect();
        println!(
            "{name}: depth {} nodes {} nps {} time {ms}ms score {} pv {}",
            info.depth_completed,
            info.nodes,
            info.nodes * 1000 / ms,
            info.score,
            pv.join(" ")
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "perft" {
        let depth: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(4);
        let fen = args.get(3).map(|s| s.as_str());
        cmd_perft(depth, fen);
    } else if args.len() >= 2 && args[1] == "bench" {
        cmd_bench();
    } else if args.len() >= 3 && args[1] == "texel-tune" {
        // Dev-Werkzeug (wie perft/bench): Texel-Tuning auf eigener TSV-Datei.
        // Aufruf: `funken texel-tune <train.tsv> <hold.tsv> <out.params> [sweeps]`.
        let hold = args.get(3).map(|s| s.as_str()).unwrap_or("");
        let out = args.get(4).map(|s| s.as_str()).unwrap_or("tuned.params");
        let sweeps: usize = args.get(5).and_then(|v| v.parse().ok()).unwrap_or(15);
        tune::cmd_tune(args[2].as_str(), hold, out, sweeps);
    } else {
        uci::uci_loop();
    }
}
