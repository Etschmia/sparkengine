// Eigen-Eroeffnungsbuch (Bot-only, default aus).
//
// Eigene Implementierung, nur std: Textdatei mit Stellungen (4-Felder-FEN)
// und Funkens eigenen Tiefenanalyse-Zuegen (UCI). Auswahl uniform zufaellig
// unter den gespeicherten Zuegen (alle per Konstruktion nah beieinander).
// Treffer werden in der UCI-Schleife vor der Suche geprueft und sofort als
// `bestmove` zurueckgegeben — die Engine-Messung (Buch aus, Default) bleibt
// unberuehrt.

use crate::chess::{sq_at, Board, Move};

/// Ein Buchzug als Felder + Umwandlung (Flags wie EP/Rochade werden beim
/// Abgleich gegen die legalen Zuege der Stellung verifiziert).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BookMove {
    pub from: u8,
    pub to: u8,
    pub promo: u8,
}

pub struct Book {
    entries: Vec<(u64, Vec<BookMove>)>,
}

impl Book {
    /// Laedt eine Buchdatei. Format pro Zeile:
    /// `<placement> <side> <castling> <ep> ; <uci>[,<uci>...]`
    /// `#`-Zeilen und Leerzeilen werden ignoriert. Fehlerhafte Zeilen werden
    /// uebersprungen (gezaehlt, kein Abbruch).
    pub fn load(path: &str) -> Result<(Book, usize), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("Buch {path} unlesbar: {e}"))?;
        let mut entries: Vec<(u64, Vec<BookMove>)> = Vec::new();
        let mut skipped = 0usize;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(2, ';');
            let fen4 = match parts.next() {
                Some(f) => f.trim(),
                None => {
                    skipped += 1;
                    continue;
                }
            };
            let moves = match parts.next() {
                Some(m) => m.trim(),
                None => {
                    skipped += 1;
                    continue;
                }
            };
            let board = match Board::from_fen(&format!("{fen4} 0 1")) {
                Ok(b) => b,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let mut bms = Vec::new();
            for uci in moves.split(',') {
                if let Some(bm) = parse_book_uci(uci.trim()) {
                    bms.push(bm);
                }
            }
            if bms.is_empty() {
                skipped += 1;
                continue;
            }
            entries.push((board.hash, bms));
        }
        Ok((Book { entries }, skipped))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Buchzug fuer die Stellung: Treffer per Hash, verifiziert gegen die
    /// legalen Zuege (Flags!) und optional gegen die `searchmoves`-Liste.
    /// Auswahl uniform per `rng`. `None` = kein Treffer (normale Suche).
    pub fn pick(
        &self,
        board: &Board,
        legal: &[Move],
        allow: Option<&[Move]>,
        rng: &mut SimpleRng,
    ) -> Option<Move> {
        let mut cands: Vec<Move> = Vec::with_capacity(3);
        for (hash, bms) in self.entries.iter() {
            if *hash != board.hash {
                continue;
            }
            for bm in bms {
                if let Some(m) = legal
                    .iter()
                    .find(|m| m.from == bm.from && m.to == bm.to && m.promo == bm.promo)
                {
                    if let Some(a) = allow {
                        if !a.iter().any(|x| x.from == m.from && x.to == m.to && x.promo == m.promo) {
                            continue;
                        }
                    }
                    cands.push(*m);
                }
            }
        }
        if cands.is_empty() {
            return None;
        }
        Some(cands[rng.next_usize(cands.len())])
    }
}

fn parse_book_uci(s: &str) -> Option<BookMove> {
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
    Some(BookMove { from: sq_at(ff, rf), to: sq_at(tf, rt), promo })
}

/// Minimaler Zufall (xorshift64*, std-only) fuer die Buch-Auswahl.
/// Abgedichtet gegen n = 0 (gibt 0 zurueck).
pub struct SimpleRng(pub u64);

impl SimpleRng {
    pub fn seed() -> SimpleRng {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ (d.as_secs() << 32))
            .unwrap_or(0x9E3779B97F4A7C15);
        // xorshift braucht Zustand != 0.
        SimpleRng(if nanos == 0 { 0x9E3779B97F4A7C15 } else { nanos })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_usize(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next() % (n as u64)) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Board;

    fn write_tmp(name: &str, content: &str) -> String {
        let p = std::env::temp_dir().join(name);
        std::fs::write(&p, content).unwrap();
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn book_load_skips_junk() {
        let p = write_tmp(
            "funken_booktest1.txt",
            "# Kommentar\n\nrnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - ; e2e4,d2d4\nungueltig ohne semikolon\n",
        );
        let (book, skipped) = Book::load(&p).unwrap();
        assert_eq!(book.len(), 1);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn book_pick_hits_startpos_and_respects_allow() {
        let p = write_tmp(
            "funken_booktest2.txt",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - ; e2e4\n",
        );
        let (book, _) = Book::load(&p).unwrap();
        let mut b = Board::startpos();
        let mut legal = Vec::new();
        b.gen_legal(&mut legal);
        let mut rng = SimpleRng(12345);
        let m = book.pick(&b, &legal, None, &mut rng).unwrap();
        assert_eq!(m.to_uci(), "e2e4");
        // Allow-Liste ohne e2e4 -> kein Treffer.
        let allow: Vec<Move> = legal.iter().filter(|m| m.to_uci() == "d2d4").cloned().collect();
        assert!(book.pick(&b, &legal, Some(&allow), &mut rng).is_none());
    }

    #[test]
    fn book_pick_verifies_legality() {
        // Buchzug d7d5 ist in der Startpos illegal -> kein Treffer.
        let p = write_tmp(
            "funken_booktest3.txt",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - ; d7d5\n",
        );
        let (book, _) = Book::load(&p).unwrap();
        let mut b = Board::startpos();
        let mut legal = Vec::new();
        b.gen_legal(&mut legal);
        let mut rng = SimpleRng(7);
        assert!(book.pick(&b, &legal, None, &mut rng).is_none());
    }

    #[test]
    fn rng_never_panics_on_zero() {
        let mut rng = SimpleRng(1);
        assert_eq!(rng.next_usize(0), 0);
    }
}
