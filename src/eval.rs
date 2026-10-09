// Funken evaluation: tapered mid/endgame score, white-relative, centipawns.
//
// All weights and tables below are my own choices, built from first
// principles (centralisation, advancement, basic pawn-structure heuristics).
// No copied piece-square tables and no external tuning data. The tables are
// generated programmatically from small rules so the construction is
// transparent and reproducible.

use crate::chess::*;

// Material values are in chess::PIECE_VALUE.

// Phase weights for tapering (own choice): N=1, R=2, Q=4 per side.
// NOT tuned (phase definition): N=1, B=1, R=2, Q=4.
const PHASE_W: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

// --- tunable parameters ------------------------------------------------------
// Every weight below was previously a hardcoded constant (hand values, see git
// history). Grouped in one struct so systematic tuning (Texel/SPSA) can vary
// them. `STANDARD` holds the Gen4 Texel tuning (24.09.2026, own 232k
// self-play positions at 200k nodes, measured +51 Elo vs. the v3 tuning in
// M5/200, cumulatively ~+100 vs. the hand values); older stages live on in
// git history. Material (PIECE_VALUE)
// and PHASE_W are deliberately NOT parameters: the search shares PIECE_VALUE
// (MVV-LVA, delta pruning) and must not change silently with eval tuning.
#[derive(Clone, Copy)]
pub struct EvalParams {
    pub pawn_adv_sq_mg: i32,
    pub pawn_adv_sq_eg: i32,
    pub pawn_center: i32,
    pub pawn_base: i32,
    pub knight_base_mg: i32,
    pub knight_slope_mg: i32,
    pub knight_base_eg: i32,
    pub knight_slope_eg: i32,
    pub bishop_base_mg: i32,
    pub bishop_slope_mg: i32,
    pub bishop_base_eg: i32,
    pub bishop_slope_eg: i32,
    pub rook_seventh: i32,
    pub rook_center_mg: i32,
    pub rook_center_eg: i32,
    pub queen_base_mg: i32,
    pub queen_base_eg: i32,
    pub king_base_mg: i32,
    pub king_slope_mg: i32,
    pub king_base_eg: i32,
    pub king_slope_eg: i32,
    pub mob: [i32; 6],
    pub doubled_mg: i32,
    pub doubled_eg: i32,
    pub isolated_mg: i32,
    pub isolated_eg: i32,
    pub passed_base_mg: i32,
    pub passed_adv_mg: i32,
    pub passed_base_eg: i32,
    pub passed_adv_eg: i32,
    pub bishop_pair_mg: i32,
    pub bishop_pair_eg: i32,
    pub rook_open: i32,
    pub rook_half: i32,
    pub shield: i32,
    pub tempo: i32,
}

pub const STANDARD: EvalParams = EvalParams {
    pawn_adv_sq_mg: 1,
    pawn_adv_sq_eg: 1,
    pawn_center: -1,
    pawn_base: -15,
    knight_base_mg: 24,
    knight_slope_mg: 13,
    knight_base_eg: 5,
    knight_slope_eg: 12,
    bishop_base_mg: 4,
    bishop_slope_mg: 9,
    bishop_base_eg: -14,
    bishop_slope_eg: 4,
    rook_seventh: 60,
    rook_center_mg: -46,
    rook_center_eg: -24,
    queen_base_mg: 50,
    queen_base_eg: 52,
    king_base_mg: 10,
    king_slope_mg: 21,
    king_base_eg: 12,
    king_slope_eg: 7,
    mob: [0, 5, 11, 10, 10, 0],
    doubled_mg: 3,
    doubled_eg: 33,
    isolated_mg: 14,
    isolated_eg: 9,
    passed_base_mg: 1,
    passed_adv_mg: -2,
    passed_base_eg: -29,
    passed_adv_eg: 28,
    bishop_pair_mg: 32,
    bishop_pair_eg: 52,
    rook_open: 55,
    rook_half: 30,
    shield: 12,
    tempo: 8,
};

// --- programmatic piece-square tables --------------------------------------
// Tables are indexed [rank][file] from White's perspective (rank 0 = rank 1).
// Black mirrors them with rank ^ 7.

fn build_pawn(p: &EvalParams, mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let adv = r as i32; // 0..7
            let center = 3 - ((3 - f as i32).abs().min((4 - f as i32).abs()));
            mg[r][f] = p.pawn_adv_sq_mg * adv * adv + p.pawn_center * center + p.pawn_base;
            eg[r][f] = p.pawn_adv_sq_eg * adv * adv + p.pawn_center * center + p.pawn_base;
        }
    }
}

fn build_knight(p: &EvalParams, mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            // chebyshev distance to the d4/e4/d5/e5 block
            let df = if f < 3 { 3 - f as i32 } else if f > 4 { f as i32 - 4 } else { 0 };
            let dr = if r < 3 { 3 - r as i32 } else if r > 4 { r as i32 - 4 } else { 0 };
            let d = df.max(dr);
            mg[r][f] = p.knight_base_mg - p.knight_slope_mg * d;
            eg[r][f] = p.knight_base_eg - p.knight_slope_eg * d;
        }
    }
}

fn build_bishop(p: &EvalParams, mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            mg[r][f] = p.bishop_base_mg - p.bishop_slope_mg * (df + dr);
            eg[r][f] = p.bishop_base_eg - p.bishop_slope_eg * (df + dr);
        }
    }
}

fn build_rook(p: &EvalParams, mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let seventh = if r == 6 { p.rook_seventh } else { 0 };
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            mg[r][f] = seventh + p.rook_center_mg - df;
            eg[r][f] = p.rook_center_eg - df;
        }
    }
}

fn build_queen(p: &EvalParams, mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            mg[r][f] = p.queen_base_mg - (df + dr);
            eg[r][f] = p.queen_base_eg - (df + dr) / 2;
        }
    }
}

fn build_king_mg(p: &EvalParams, mg: &mut [[i32; 8]; 8]) {
    // Encourage a castled king: bonus near g1/c1 (and mirrored).
    for r in 0..8 {
        for f in 0..8 {
            let dg: i32 = (f as i32 - 6).abs() + (r as i32 - 0).abs();
            let dc: i32 = (f as i32 - 2).abs() + (r as i32 - 0).abs();
            let d = dg.min(dc);
            mg[r][f] = p.king_base_mg - p.king_slope_mg * d;
        }
    }
}

fn build_king_eg(p: &EvalParams, eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            eg[r][f] = p.king_base_eg - p.king_slope_eg * (df + dr);
        }
    }
}

pub struct Tables {
    pub mg: [[[i32; 8]; 8]; 6],
    pub eg: [[[i32; 8]; 8]; 6],
}

static TABLES: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();

pub fn tables() -> &'static Tables {
    TABLES.get_or_init(|| build_tables(&STANDARD))
}

// --- Tuning-Infra (FUNKEN_PARAMS): optionale Gewichte aus Datei -------------
// Format: `name wert` pro Zeile (von `funken texel-tune` geschrieben).
// Ohne Env-Var: exakt STANDARD (Tests/Bench unverändert). Einmalig pro
// Prozess geladen (OnceLock), inkl. passender Tabellen.
static PARAM_OVERRIDE: std::sync::OnceLock<Option<(EvalParams, Tables)>> =
    std::sync::OnceLock::new();

fn param_field(p: &mut EvalParams, name: &str, v: i32) -> bool {
    match name {
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
        _ => return false,
    }
    true
}

pub fn params_from_text(text: &str) -> Option<EvalParams> {
    let mut p = STANDARD;
    for (ln, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let (name, val) = match (it.next(), it.next()) {
            (Some(n), Some(v)) => (n, v),
            _ => return None,
        };
        let _ = ln;
        let v: i32 = val.parse().ok()?;
        if !param_field(&mut p, name, v) {
            return None;
        }
    }
    Some(p)
}

pub fn params_to_text(p: &EvalParams) -> String {
    let mut s = String::new();
    let mut w = |name: &str, v: i32| {
        s.push_str(&format!("{name} {v}\n"));
    };
    w("pawn_adv_sq_mg", p.pawn_adv_sq_mg);
    w("pawn_adv_sq_eg", p.pawn_adv_sq_eg);
    w("pawn_center", p.pawn_center);
    w("pawn_base", p.pawn_base);
    w("knight_base_mg", p.knight_base_mg);
    w("knight_slope_mg", p.knight_slope_mg);
    w("knight_base_eg", p.knight_base_eg);
    w("knight_slope_eg", p.knight_slope_eg);
    w("bishop_base_mg", p.bishop_base_mg);
    w("bishop_slope_mg", p.bishop_slope_mg);
    w("bishop_base_eg", p.bishop_base_eg);
    w("bishop_slope_eg", p.bishop_slope_eg);
    w("rook_seventh", p.rook_seventh);
    w("rook_center_mg", p.rook_center_mg);
    w("rook_center_eg", p.rook_center_eg);
    w("queen_base_mg", p.queen_base_mg);
    w("queen_base_eg", p.queen_base_eg);
    w("king_base_mg", p.king_base_mg);
    w("king_slope_mg", p.king_slope_mg);
    w("king_base_eg", p.king_base_eg);
    w("king_slope_eg", p.king_slope_eg);
    w("mob_n", p.mob[1]);
    w("mob_b", p.mob[2]);
    w("mob_r", p.mob[3]);
    w("mob_q", p.mob[4]);
    w("doubled_mg", p.doubled_mg);
    w("doubled_eg", p.doubled_eg);
    w("isolated_mg", p.isolated_mg);
    w("isolated_eg", p.isolated_eg);
    w("passed_base_mg", p.passed_base_mg);
    w("passed_adv_mg", p.passed_adv_mg);
    w("passed_base_eg", p.passed_base_eg);
    w("passed_adv_eg", p.passed_adv_eg);
    w("bishop_pair_mg", p.bishop_pair_mg);
    w("bishop_pair_eg", p.bishop_pair_eg);
    w("rook_open", p.rook_open);
    w("rook_half", p.rook_half);
    w("shield", p.shield);
    w("tempo", p.tempo);
    s
}

fn override_params() -> Option<(&'static EvalParams, &'static Tables)> {
    PARAM_OVERRIDE
        .get_or_init(|| {
            let path = match std::env::var("FUNKEN_PARAMS") {
                Ok(p) => p,
                Err(_) => return None,
            };
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("FUNKEN_PARAMS: {path} nicht lesbar ({e}), STANDARD aktiv");
                    return None;
                }
            };
            match params_from_text(&text) {
                Some(p) => {
                    let tb = build_tables(&p);
                    Some((p, tb))
                }
                None => {
                    eprintln!("FUNKEN_PARAMS: {path} ungueltig, STANDARD aktiv");
                    None
                }
            }
        })
        .as_ref()
        .map(|(p, t)| (p, t))
}

pub fn build_tables(p: &EvalParams) -> Tables {
    let mut t = Tables { mg: [[[0; 8]; 8]; 6], eg: [[[0; 8]; 8]; 6] };
    build_pawn(p, &mut t.mg[0], &mut t.eg[0]);
    build_knight(p, &mut t.mg[1], &mut t.eg[1]);
    build_bishop(p, &mut t.mg[2], &mut t.eg[2]);
    build_rook(p, &mut t.mg[3], &mut t.eg[3]);
    build_queen(p, &mut t.mg[4], &mut t.eg[4]);
    build_king_mg(p, &mut t.mg[5]);
    build_king_eg(p, &mut t.eg[5]);
    t
}

// --- K+B+N vs bare K mating guidance (own heuristic, textbook idea) ---
// Without steering the search shuffles for dozens of moves at game budgets
// (9.24: no mate to 1M nodes in a KBN position) and risks repetition draws
// from a won position. Two classic steering terms: drive the defender
// towards the mating corner (whose square color matches the bishop) and
// pull the attacker king towards the defender. Small (<=154 cp) beside a
// +600 position: it only orders winning moves, never flips the assessment.
// Applies only to exact K+B+N vs bare K (either color attacking).
const KBN_CORNER_W: i32 = 12;
const KBN_KING_W: i32 = 10;

fn cheby(a: u8, b: u8) -> i32 {
    let df = (file_of(a) - file_of(b)).abs() as i32;
    let dr = (rank_of(a) - rank_of(b)).abs() as i32;
    df.max(dr)
}

/// White-relative KBN guidance bonus, 0 unless exactly K+B+N vs bare K.
fn kbn_guide(b: &Board) -> i32 {
    let mut wb = 0;
    let mut wn = 0;
    let mut wother = 0;
    let mut bb = 0;
    let mut bn = 0;
    let mut bother = 0;
    let mut wb_sq = 0u8;
    let mut bb_sq = 0u8;
    for s in 0..64usize {
        let pc = b.sq[s];
        if pc == EMPTY || type_of(pc) == 5 {
            continue;
        }
        let c = color_of(pc);
        match type_of(pc) {
            2 => {
                if c == WHITE {
                    wb += 1;
                    wb_sq = s as u8;
                } else {
                    bb += 1;
                    bb_sq = s as u8;
                }
            }
            1 => {
                if c == WHITE {
                    wn += 1;
                } else {
                    bn += 1;
                }
            }
            _ => {
                if c == WHITE {
                    wother += 1;
                } else {
                    bother += 1;
                }
            }
        }
    }
    let white_attacks = wb == 1 && wn == 1 && wother == 0 && bb == 0 && bn == 0 && bother == 0;
    let black_attacks = bb == 1 && bn == 1 && bother == 0 && wb == 0 && wn == 0 && wother == 0;
    if !white_attacks && !black_attacks {
        return 0;
    }
    let (att_king, def_king, bsq, sign) = if white_attacks {
        (b.king[WHITE as usize], b.king[BLACK as usize], wb_sq, 1)
    } else {
        (b.king[BLACK as usize], b.king[WHITE as usize], bb_sq, -1)
    };
    // Mating corner matches the bishop's square color: a1/h8 are dark (0),
    // h1/a8 are light (1). Square indices: a1=0, h1=7, a8=56, h8=63.
    let bcol = (file_of(bsq) + rank_of(bsq)) & 1;
    let corners = if bcol == 0 { [0u8, 63u8] } else { [7u8, 56u8] };
    // Steer towards the nearer matching corner (no corner-switch confusion).
    let target = if cheby(def_king, corners[0]) <= cheby(def_king, corners[1]) {
        corners[0]
    } else {
        corners[1]
    };
    let corner_term = (7 - cheby(def_king, target)) * KBN_CORNER_W;
    let king_term = (7 - cheby(att_king, def_king)) * KBN_KING_W;
    sign * (corner_term + king_term)
}

pub fn evaluate(b: &Board) -> i32 {
    if let Some((p, t)) = override_params() {
        evaluate_with(b, p, t)
    } else {
        evaluate_with(b, &STANDARD, tables())
    }
}

// Count squares a piece attacks (own pieces block, enemy squares count).
fn mobility_of(b: &Board, s: usize, ptype: u8, color: u8) -> i32 {
    let f = file_of(s as u8);
    let r = rank_of(s as u8);
    let mut n = 0;
    if ptype == 1 {
        const KN: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
        for (df, dr) in KN {
            let ff = f + df;
            let rr = r + dr;
            if (0..8).contains(&ff) && (0..8).contains(&rr) {
                let q = b.sq[sq_at(ff, rr) as usize];
                if q == EMPTY || color_of(q) != color {
                    n += 1;
                }
            }
        }
        return n;
    }
    let dirs: &[(i8, i8)] = match ptype {
        2 => &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
        3 => &[(1, 0), (-1, 0), (0, 1), (0, -1)],
        4 => &[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)],
        _ => return 0,
    };
    for (df, dr) in dirs.iter() {
        let mut ff = f + df;
        let mut rr = r + dr;
        while (0..8).contains(&ff) && (0..8).contains(&rr) {
            let q = b.sq[sq_at(ff, rr) as usize];
            if q == EMPTY {
                n += 1;
            } else {
                if color_of(q) != color {
                    n += 1;
                }
                break;
            }
            ff += df;
            rr += dr;
        }
    }
    n
}

pub fn evaluate_with(b: &Board, p: &EvalParams, t: &Tables) -> i32 {
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut phase = 0i32;

    // pawn file occupancy for structure + rook files
    let mut white_pawn_file = [0u8; 8];
    let mut black_pawn_file = [0u8; 8];
    // pawn bitboards-lite: presence per file/rank for passed-pawn test
    let mut wp_at = [[false; 8]; 8];
    let mut bp_at = [[false; 8]; 8];

    let mut bishops = [0i32; 2];
    let mut king_sq = [0u8; 2];

    for s in 0..64usize {
        let pc = b.sq[s];
        if pc == EMPTY {
            continue;
        }
        let c = color_of(pc);
        let pt = type_of(pc) as usize;
        let f = (s & 7) as usize;
        let r = (s >> 3) as usize;
        let tr = if c == WHITE { r } else { 7 - r };
        let sign = if c == WHITE { 1 } else { -1 };
        mg += sign * (PIECE_VALUE[pt] + t.mg[pt][tr][f]);
        eg += sign * (PIECE_VALUE[pt] + t.eg[pt][tr][f]);
        phase += PHASE_W[pt];
        if pt == 0 {
            if c == WHITE {
                white_pawn_file[f] += 1;
                wp_at[f][r] = true;
            } else {
                black_pawn_file[f] += 1;
                bp_at[f][r] = true;
            }
        }
        if pt == 2 {
            bishops[c as usize] += 1;
        }
        if pt == 5 {
            king_sq[c as usize] = s as u8;
        }
        if p.mob[pt] != 0 {
            let m = mobility_of(b, s, pt as u8, c);
            // mobility matters less with little material left; scale by nothing fancy
            mg += sign * m * p.mob[pt] / 2;
            eg += sign * m * p.mob[pt] / 2;
        }
    }

    // --- pawn structure (own weights) ---
    let mut struct_mg = 0i32;
    let mut struct_eg = 0i32;
    for f in 0..8 {
        // doubled pawns
        if white_pawn_file[f] > 1 {
            struct_mg -= p.doubled_mg * (white_pawn_file[f] - 1) as i32;
            struct_eg -= p.doubled_eg * (white_pawn_file[f] - 1) as i32;
        }
        if black_pawn_file[f] > 1 {
            struct_mg += p.doubled_mg * (black_pawn_file[f] - 1) as i32;
            struct_eg += p.doubled_eg * (black_pawn_file[f] - 1) as i32;
        }
        // isolated pawns
        let w_adj = (f > 0 && white_pawn_file[f - 1] > 0) || (f < 7 && white_pawn_file[f + 1] > 0);
        let b_adj = (f > 0 && black_pawn_file[f - 1] > 0) || (f < 7 && black_pawn_file[f + 1] > 0);
        if white_pawn_file[f] > 0 && !w_adj {
            struct_mg -= p.isolated_mg;
            struct_eg -= p.isolated_eg;
        }
        if black_pawn_file[f] > 0 && !b_adj {
            struct_mg += p.isolated_mg;
            struct_eg += p.isolated_eg;
        }
    }
    // passed pawns
    for f in 0..8 {
        for r in 0..8 {
            if wp_at[f][r] {
                let mut blocked = false;
                for ff in f.saturating_sub(1)..=(f + 1).min(7) {
                    for rr in (r + 1)..8 {
                        if bp_at[ff][rr] {
                            blocked = true;
                            break;
                        }
                    }
                    if blocked {
                        break;
                    }
                }
                if !blocked {
                    let adv = r as i32; // higher = closer to promotion
                    struct_mg += p.passed_base_mg + p.passed_adv_mg * adv;
                    struct_eg += p.passed_base_eg + p.passed_adv_eg * adv;
                }
            }
            if bp_at[f][r] {
                let mut blocked = false;
                for ff in f.saturating_sub(1)..=(f + 1).min(7) {
                    for rr in 0..r {
                        if wp_at[ff][rr] {
                            blocked = true;
                            break;
                        }
                    }
                    if blocked {
                        break;
                    }
                }
                if !blocked {
                    let adv = (7 - r) as i32;
                    struct_mg -= p.passed_base_mg + p.passed_adv_mg * adv;
                    struct_eg -= p.passed_base_eg + p.passed_adv_eg * adv;
                }
            }
        }
    }

    // --- bishop pair, rooks on open files, king shield (own weights) ---
    let mut extra_mg = 0i32;
    let mut extra_eg = 0i32;
    if bishops[0] >= 2 {
        extra_mg += p.bishop_pair_mg;
        extra_eg += p.bishop_pair_eg;
    }
    if bishops[1] >= 2 {
        extra_mg -= p.bishop_pair_mg;
        extra_eg -= p.bishop_pair_eg;
    }
    for s in 0..64usize {
        let pc = b.sq[s];
        if pc == EMPTY || type_of(pc) != 3 {
            continue;
        }
        let c = color_of(pc);
        let f = s & 7;
        let own = if c == WHITE { white_pawn_file[f] } else { black_pawn_file[f] };
        let foe = if c == WHITE { black_pawn_file[f] } else { white_pawn_file[f] };
        let sign = if c == WHITE { 1 } else { -1 };
        if own == 0 {
            if foe == 0 {
                extra_mg += sign * p.rook_open;
            } else {
                extra_mg += sign * p.rook_half;
            }
        }
    }
    // king pawn shield (middlegame only): three squares ahead of the king
    for c in [WHITE, BLACK] {
        let k = king_sq[c as usize];
        let kf = file_of(k);
        let kr = rank_of(k);
        // only when king is on its first two ranks
        let home = if c == WHITE { kr <= 1 } else { kr >= 6 };
        if !home {
            continue;
        }
        let ahead: i8 = if c == WHITE { 1 } else { -1 };
        let mut shield = 0;
        for df in -1..=1 {
            let ff = kf + df;
            let rr = kr + ahead;
            if (0..8).contains(&ff) && (0..8).contains(&rr) {
                let q = b.sq[sq_at(ff, rr) as usize];
                if q != EMPTY && color_of(q) == c && type_of(q) == 0 {
                    shield += 1;
                }
            }
        }
        let missing = 3 - shield;
        if c == WHITE {
            extra_mg -= p.shield * missing;
        } else {
            extra_mg += p.shield * missing;
        }
    }

    mg += struct_mg + extra_mg;
    eg += struct_eg + extra_eg;

    // taper
    let ph = phase.min(MAX_PHASE);
    (mg * ph + eg * (MAX_PHASE - ph)) / MAX_PHASE
        + if b.side == WHITE { p.tempo } else { -p.tempo } // own tempo bonus
        + kbn_guide(b) // K+B+N mating steer (0 unless exact KBN vs bare K)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kbn_guide_white_attacks() {
        // Weiss KBN vs blanker Koenig: Führung positiv und klein (<=154).
        let b = Board::from_fen("8/8/3B4/8/4K3/8/8/5k1N w - - 0 1").unwrap();
        let g = kbn_guide(&b);
        assert_eq!(g, 64, "guide = {g}"); // (7-5)*12 + (7-3)*10
        assert!(g > 0 && g <= 7 * KBN_CORNER_W + 7 * KBN_KING_W);
        // Material dominiert weiter: klar gewonnen, kein Matt-Phantom.
        assert!(evaluate(&b) > 400, "eval = {}", evaluate(&b));
    }

    #[test]
    fn kbn_guide_black_mirrors_white() {
        // Gleiche Geometrie in Schwarz: exakt negiert (kein Farb-Bias).
        let w = Board::from_fen("8/8/3B4/8/4K3/8/8/5k1N w - - 0 1").unwrap();
        let b = Board::from_fen("8/8/3b4/8/4k3/8/8/5K1n w - - 0 1").unwrap();
        assert_eq!(kbn_guide(&b), -kbn_guide(&w));
        assert!(evaluate(&b) < -400);
    }

    #[test]
    fn kbn_guide_cornered_defender() {
        // Verteidiger schon in der passenden Ecke (dunkler Laeufer, h8):
        // Eck-Term maximal (7-0)*12.
        let b = Board::from_fen("7k/8/2K5/8/3B4/8/8/4N3 w - - 0 1").unwrap();
        assert_eq!(kbn_guide(&b), 84 + 20, "guide = {}", kbn_guide(&b));
    }

    #[test]
    fn kbn_guide_off_unless_exact() {
        // Startpos, K+S vs K (ungenuegend, kein KBN), KBN vs K+Mehrfigur,
        // KBB vs K: überall 0 — die Führung feuert nur im exakten Fall.
        assert_eq!(kbn_guide(&Board::startpos()), 0);
        for fen in [
            "8/8/4k3/8/8/3N4/8/4K3 w - - 0 1",     // K+S vs K
            "8/8/3b4/8/4k3/8/7p/5K1N w - - 0 1",   // KBN vs K+Bauer
            "7k/8/8/4B3/8/8/5B1K/8 w - - 0 1",     // KBB vs K
            "8/8/3b4/8/4k3/8/8/5K2 w - - 0 1",     // nur KL vs K
        ] {
            let b = Board::from_fen(fen).unwrap();
            assert_eq!(kbn_guide(&b), 0, "{fen}");
        }
    }
}
