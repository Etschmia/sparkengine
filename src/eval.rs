// Funken evaluation: tapered mid/endgame score, white-relative, centipawns.
//
// All weights and tables below are my own choices, built from first
// principles (centralisation, advancement, basic pawn-structure heuristics).
// No copied piece-square tables and no external tuning data. The tables are
// generated programmatically from small rules so the construction is
// transparent and reproducible.

use crate::chess::*;

// Material values are in chess::PIECE_VALUE.

// Phase weights for tapering (own choice): N=1, B=1, R=2, Q=4 per side.
const PHASE_W: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

// --- programmatic piece-square tables --------------------------------------
// Tables are indexed [rank][file] from White's perspective (rank 0 = rank 1).
// Black mirrors them with rank ^ 7.

fn build_pawn(mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let adv = r as i32; // 0..7
            let center = 3 - ((3 - f as i32).abs().min((4 - f as i32).abs()));
            mg[r][f] = adv * adv + center * 2 - 6;
            eg[r][f] = adv * adv * 2 + center * 2 - 6;
        }
    }
}

fn build_knight(mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            // chebyshev distance to the d4/e4/d5/e5 block
            let df = if f < 3 { 3 - f as i32 } else if f > 4 { f as i32 - 4 } else { 0 };
            let dr = if r < 3 { 3 - r as i32 } else if r > 4 { r as i32 - 4 } else { 0 };
            let d = df.max(dr);
            mg[r][f] = 12 - 9 * d;
            eg[r][f] = 8 - 6 * d;
        }
    }
}

fn build_bishop(mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            mg[r][f] = 8 - 3 * (df + dr);
            eg[r][f] = 6 - 2 * (df + dr);
        }
    }
}

fn build_rook(mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let seventh = if r == 6 { 10 } else { 0 };
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            mg[r][f] = seventh + 4 - df;
            eg[r][f] = 4 - df;
        }
    }
}

fn build_queen(mg: &mut [[i32; 8]; 8], eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            mg[r][f] = 4 - (df + dr);
            eg[r][f] = 2 - (df + dr) / 2;
        }
    }
}

fn build_king_mg(mg: &mut [[i32; 8]; 8]) {
    // Encourage a castled king: bonus near g1/c1 (and mirrored).
    for r in 0..8 {
        for f in 0..8 {
            let dg: i32 = (f as i32 - 6).abs() + (r as i32 - 0).abs();
            let dc: i32 = (f as i32 - 2).abs() + (r as i32 - 0).abs();
            let d = dg.min(dc);
            mg[r][f] = 10 - 7 * d;
        }
    }
}

fn build_king_eg(eg: &mut [[i32; 8]; 8]) {
    for r in 0..8 {
        for f in 0..8 {
            let df = (f as i32 - 3).abs().min((f as i32 - 4).abs());
            let dr = (r as i32 - 3).abs().min((r as i32 - 4).abs());
            eg[r][f] = 12 - 5 * (df + dr);
        }
    }
}

pub struct Tables {
    pub mg: [[[i32; 8]; 8]; 6],
    pub eg: [[[i32; 8]; 8]; 6],
}

static TABLES: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();

pub fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let mut t = Tables { mg: [[[0; 8]; 8]; 6], eg: [[[0; 8]; 8]; 6] };
        build_pawn(&mut t.mg[0], &mut t.eg[0]);
        build_knight(&mut t.mg[1], &mut t.eg[1]);
        build_bishop(&mut t.mg[2], &mut t.eg[2]);
        build_rook(&mut t.mg[3], &mut t.eg[3]);
        build_queen(&mut t.mg[4], &mut t.eg[4]);
        build_king_mg(&mut t.mg[5]);
        build_king_eg(&mut t.eg[5]);
        t
    })
}

// Mobility weights per piece type (own choice), multiplied by attack-square count.
const MOB_W: [i32; 6] = [0, 4, 4, 2, 1, 0];

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

pub fn evaluate(b: &Board) -> i32 {
    let t = tables();
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
        let p = b.sq[s];
        if p == EMPTY {
            continue;
        }
        let c = color_of(p);
        let pt = type_of(p) as usize;
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
        if MOB_W[pt] != 0 {
            let m = mobility_of(b, s, pt as u8, c);
            // mobility matters less with little material left; scale by nothing fancy
            mg += sign * m * MOB_W[pt] / 2;
            eg += sign * m * MOB_W[pt] / 2;
        }
    }

    // --- pawn structure (own weights) ---
    let mut struct_mg = 0i32;
    let mut struct_eg = 0i32;
    for f in 0..8 {
        // doubled pawns
        if white_pawn_file[f] > 1 {
            struct_mg -= 12 * (white_pawn_file[f] - 1) as i32;
            struct_eg -= 15 * (white_pawn_file[f] - 1) as i32;
        }
        if black_pawn_file[f] > 1 {
            struct_mg += 12 * (black_pawn_file[f] - 1) as i32;
            struct_eg += 15 * (black_pawn_file[f] - 1) as i32;
        }
        // isolated pawns
        let w_adj = (f > 0 && white_pawn_file[f - 1] > 0) || (f < 7 && white_pawn_file[f + 1] > 0);
        let b_adj = (f > 0 && black_pawn_file[f - 1] > 0) || (f < 7 && black_pawn_file[f + 1] > 0);
        if white_pawn_file[f] > 0 && !w_adj {
            struct_mg -= 10;
            struct_eg -= 12;
        }
        if black_pawn_file[f] > 0 && !b_adj {
            struct_mg += 10;
            struct_eg += 12;
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
                    struct_mg += 12 + 6 * adv;
                    struct_eg += 18 + 12 * adv;
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
                    struct_mg -= 12 + 6 * adv;
                    struct_eg -= 18 + 12 * adv;
                }
            }
        }
    }

    // --- bishop pair, rooks on open files, king shield (own weights) ---
    let mut extra_mg = 0i32;
    let mut extra_eg = 0i32;
    if bishops[0] >= 2 {
        extra_mg += 30;
        extra_eg += 40;
    }
    if bishops[1] >= 2 {
        extra_mg -= 30;
        extra_eg -= 40;
    }
    for s in 0..64usize {
        let p = b.sq[s];
        if p == EMPTY || type_of(p) != 3 {
            continue;
        }
        let c = color_of(p);
        let f = s & 7;
        let own = if c == WHITE { white_pawn_file[f] } else { black_pawn_file[f] };
        let foe = if c == WHITE { black_pawn_file[f] } else { white_pawn_file[f] };
        let sign = if c == WHITE { 1 } else { -1 };
        if own == 0 {
            if foe == 0 {
                extra_mg += sign * 15;
            } else {
                extra_mg += sign * 8;
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
            extra_mg -= 12 * missing;
        } else {
            extra_mg += 12 * missing;
        }
    }

    mg += struct_mg + extra_mg;
    eg += struct_eg + extra_eg;

    // taper
    let ph = phase.min(MAX_PHASE);
    (mg * ph + eg * (MAX_PHASE - ph)) / MAX_PHASE
        + if b.side == WHITE { 8 } else { -8 } // own tempo bonus
}
