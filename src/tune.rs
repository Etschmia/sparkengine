// Texel-Tuning der Eval-Parameter auf selbst erzeugten Daten.
//
// Verfahren: Koordinaten-Abstieg auf MSE(Resultat, Sigmoid(Eval)).
// Sigmoid E = 1/(1+10^(-s/400)) (s in cp, Weiss-Sicht). Tabellen werden pro
// Kandidat einmal gebaut. Start: STANDARD. Nur `funken texel-tune`, kein
// Einfluss auf Engine-Verhalten (evaluate() ohne FUNKEN_PARAMS unverändert).

use crate::chess::Board;
use crate::eval::{EvalParams, STANDARD, build_tables, evaluate_with};

const NAMES: [&str; 39] = [
    "pawn_adv_sq_mg",
    "pawn_adv_sq_eg",
    "pawn_center",
    "pawn_base",
    "knight_base_mg",
    "knight_slope_mg",
    "knight_base_eg",
    "knight_slope_eg",
    "bishop_base_mg",
    "bishop_slope_mg",
    "bishop_base_eg",
    "bishop_slope_eg",
    "rook_seventh",
    "rook_center_mg",
    "rook_center_eg",
    "queen_base_mg",
    "queen_base_eg",
    "king_base_mg",
    "king_slope_mg",
    "king_base_eg",
    "king_slope_eg",
    "mob_n",
    "mob_b",
    "mob_r",
    "mob_q",
    "doubled_mg",
    "doubled_eg",
    "isolated_mg",
    "isolated_eg",
    "passed_base_mg",
    "passed_adv_mg",
    "passed_base_eg",
    "passed_adv_eg",
    "bishop_pair_mg",
    "bishop_pair_eg",
    "rook_open",
    "rook_half",
    "shield",
    "tempo",
];

fn get(p: &EvalParams, i: usize) -> i32 {
    match NAMES[i] {
        "pawn_adv_sq_mg" => p.pawn_adv_sq_mg,
        "pawn_adv_sq_eg" => p.pawn_adv_sq_eg,
        "pawn_center" => p.pawn_center,
        "pawn_base" => p.pawn_base,
        "knight_base_mg" => p.knight_base_mg,
        "knight_slope_mg" => p.knight_slope_mg,
        "knight_base_eg" => p.knight_base_eg,
        "knight_slope_eg" => p.knight_slope_eg,
        "bishop_base_mg" => p.bishop_base_mg,
        "bishop_slope_mg" => p.bishop_slope_mg,
        "bishop_base_eg" => p.bishop_base_eg,
        "bishop_slope_eg" => p.bishop_slope_eg,
        "rook_seventh" => p.rook_seventh,
        "rook_center_mg" => p.rook_center_mg,
        "rook_center_eg" => p.rook_center_eg,
        "queen_base_mg" => p.queen_base_mg,
        "queen_base_eg" => p.queen_base_eg,
        "king_base_mg" => p.king_base_mg,
        "king_slope_mg" => p.king_slope_mg,
        "king_base_eg" => p.king_base_eg,
        "king_slope_eg" => p.king_slope_eg,
        "mob_n" => p.mob[1],
        "mob_b" => p.mob[2],
        "mob_r" => p.mob[3],
        "mob_q" => p.mob[4],
        "doubled_mg" => p.doubled_mg,
        "doubled_eg" => p.doubled_eg,
        "isolated_mg" => p.isolated_mg,
        "isolated_eg" => p.isolated_eg,
        "passed_base_mg" => p.passed_base_mg,
        "passed_adv_mg" => p.passed_adv_mg,
        "passed_base_eg" => p.passed_base_eg,
        "passed_adv_eg" => p.passed_adv_eg,
        "bishop_pair_mg" => p.bishop_pair_mg,
        "bishop_pair_eg" => p.bishop_pair_eg,
        "rook_open" => p.rook_open,
        "rook_half" => p.rook_half,
        "shield" => p.shield,
        "tempo" => p.tempo,
        _ => 0,
    }
}

fn set(p: &mut EvalParams, i: usize, v: i32) {
    match NAMES[i] {
        "pawn_adv_sq_mg" => p.pawn_adv_sq_mg = v,
        "pawn_adv_sq_eg" => p.pawn_adv_sq_eg = v,
        "pawn_center" => p.pawn_center = v,
        "pawn_base" => p.pawn_base = v,
        "knight_base_mg" => p.knight_base_mg = v,
        "knight_slope_mg" => p.knight_slope_mg = v,
        "knight_base_eg" => p.knight_base_eg = v,
        "knight_slope_eg" => p.knight_slope_eg = v,
        "bishop_base_mg" => p.bishop_base_mg = v,
        "bishop_slope_mg" => p.bishop_slope_mg = v,
        "bishop_base_eg" => p.bishop_base_eg = v,
        "bishop_slope_eg" => p.bishop_slope_eg = v,
        "rook_seventh" => p.rook_seventh = v,
        "rook_center_mg" => p.rook_center_mg = v,
        "rook_center_eg" => p.rook_center_eg = v,
        "queen_base_mg" => p.queen_base_mg = v,
        "queen_base_eg" => p.queen_base_eg = v,
        "king_base_mg" => p.king_base_mg = v,
        "king_slope_mg" => p.king_slope_mg = v,
        "king_base_eg" => p.king_base_eg = v,
        "king_slope_eg" => p.king_slope_eg = v,
        "mob_n" => p.mob[1] = v,
        "mob_b" => p.mob[2] = v,
        "mob_r" => p.mob[3] = v,
        "mob_q" => p.mob[4] = v,
        "doubled_mg" => p.doubled_mg = v,
        "doubled_eg" => p.doubled_eg = v,
        "isolated_mg" => p.isolated_mg = v,
        "isolated_eg" => p.isolated_eg = v,
        "passed_base_mg" => p.passed_base_mg = v,
        "passed_adv_mg" => p.passed_adv_mg = v,
        "passed_base_eg" => p.passed_base_eg = v,
        "passed_adv_eg" => p.passed_adv_eg = v,
        "bishop_pair_mg" => p.bishop_pair_mg = v,
        "bishop_pair_eg" => p.bishop_pair_eg = v,
        "rook_open" => p.rook_open = v,
        "rook_half" => p.rook_half = v,
        "shield" => p.shield = v,
        "tempo" => p.tempo = v,
        _ => {}
    }
}

fn sigmoid(cp: i32) -> f64 {
    1.0 / (1.0 + 10f64.powf(-(cp as f64) / 400.0))
}

fn mse(data: &[(Board, f64)], p: &EvalParams) -> f64 {
    let t = build_tables(p);
    let mut sum = 0.0;
    for (b, r) in data.iter() {
        let s = evaluate_with(b, p, &t);
        let d = r - sigmoid(s);
        sum += d * d;
    }
    sum / data.len() as f64
}

fn load(path: &str) -> Vec<(Board, f64)> {
    let text = std::fs::read_to_string(path).expect("datendatei lesbar");
    let mut out = Vec::new();
    for line in text.lines() {
        let mut it = line.split('\t');
        let (fen, res) = match (it.next(), it.next()) {
            (Some(f), Some(r)) => (f, r),
            _ => continue,
        };
        if let Ok(b) = Board::from_fen(fen) {
            if let Ok(r) = res.parse::<f64>() {
                out.push((b, r));
            }
        }
    }
    out
}

pub fn cmd_tune(train_path: &str, hold_path: &str, out_path: &str, sweeps: usize) {
    let train = load(train_path);
    let hold = load(hold_path);
    println!("train {} hold {}", train.len(), hold.len());
    let mut p = STANDARD;
    let mut steps: Vec<i32> = (0..NAMES.len())
        .map(|i| (get(&p, i).abs() / 4).max(2))
        .collect();
    let e0 = mse(&train, &p);
    println!("MSE STANDARD train {e0:.5} hold {:.5}", mse(&hold, &p));
    for sw in 0..sweeps {
        let mut improved = false;
        for i in 0..NAMES.len() {
            if steps[i] <= 0 {
                continue;
            }
            let base = mse(&train, &p);
            let v = get(&p, i);
            set(&mut p, i, v + steps[i]);
            let ep = mse(&train, &p);
            if ep < base {
                improved = true;
                continue;
            }
            set(&mut p, i, v - steps[i]);
            let em = mse(&train, &p);
            if em < base {
                improved = true;
            } else {
                set(&mut p, i, v);
                steps[i] /= 2;
            }
        }
        println!(
            "sweep {} train {:.5} hold {:.5}{}",
            sw + 1,
            mse(&train, &p),
            mse(&hold, &p),
            if improved { "" } else { " (kein Fortschritt)" }
        );
        if !improved {
            break;
        }
    }
    std::fs::write(out_path, crate::eval::params_to_text(&p)).expect("params schreibbar");
    println!("geschrieben: {out_path}");
}
