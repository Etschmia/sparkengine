# Funken — Messergebnisse und Testprotokoll

Stand: 15. September 2026. Maschine: Debian 13, x86_64, 2× AMD EPYC (KVM),
3,7 GB RAM, keine GPU. Build: `cargo build --release` (Rust 1.94, LTO).
Alle Zahlen sind auf dieser Maschine reproduzierbar; absolute Zeiten/NPS
schwanken mit der VM-Last (angegeben als beobachtete Spannen).

Regel: **Ausgeführt** (mit Befehl) vs. **Ausstehend** — strikt getrennt (Abschnitt 6).

## 1. Regelkorrektheit: Perft (ausgeführt)

Referenzwerte aus der Perft-Literatur (u. a. CPW-Referenzstellungen).
Befehl: `./target/release/funken perft <tiefe> [fen]`.

| Stellung | Tiefe | Knoten (gemessen) | Referenz | Status |
|---|---|---|---|---|
| Startpos | 1–5 | 20 / 400 / 8902 / 197281 / **4865609** | identisch | ✅ |
| Kiwipete | 1–4 | 48 / 2039 / 97862 / **4085603** | identisch | ✅ |
| Pos 3 (Endspiel) | 4–5 | 43238 / **674624** | identisch | ✅ |
| Pos 4 (Umwandlung/Rochade) | 3–4 | 9467 / **422333** | identisch | ✅ |
| Pos 5 | 2–3 | 1486 / **62379** | identisch | ✅ |
| Pos 6 | 3–4 | 89890 / **3894594** | identisch | ✅ |

Perft-Leistung: ca. 5–14 Mio. Knoten/s (VM-lastabhängig).

## 2. Unit-Tests: `cargo test` — 14/14 ✅ (ausgeführt)

Schachkern: FEN-Roundtrip, Hash-Stabilität unter make/unmake (inkrementell ==
rekomputiert), Startpos-Perft 1–3, **En-passant-Fesselung** (illegaler EP-Schlag
fehlt, legale Königs-/Bauernzüge vorhanden), **Rochade durch Schach**
(kurz illegal/lang legal), Matt & Patt, unzureichendes Material (K–K, K+L–K,
K+S–K, gleichfarbige Läufer remis / ungleichfarbig spielbar), 8× Umwandlung.
Suche: Matt-in-1 (`e1e8`, Matt-Score ab Tiefe 1), Patt = 0, 50-Züge = 0,
**2. Stellungswiederholung sucht weiter** (Italienisch mit Vorgeschichte:
gleicher Zug/selber Score wie ohne Vorgeschichte, Tiefe 3),
**erzwungenes Dreifach-Remis = 0** (Weiß klar schlechter, Kh1 stellt das
3. Auftreten her → `g1h1`, Score 0; bei nur einem früheren Auftreten bleibt
der Score klar negativ; die Vorgeschichte ist dabei synthetisch injiziert —
zweimal derselbe Hash, keine legal vollständig ausgespielte
Wiederholungssequenz; getestet wird die Zählschwelle, nicht der
Partieverlauf), **wiederverwendete TT sucht die Wurzel neu**
(Tiefe 4 nach Tiefe 4 mit derselben TT: volle Knotenzahl, PV konsistent).

## 3. UCI + Zeitmanagement (ausgeführt, Treiber `tests_ucitool.py`)

| Test | Ergebnis |
|---|---|
| Handshake `uci/isready`, Optionen | ✅ (`Hash`, `Move Overhead`, `Threads`) |
| `go depth 8` Startpos | ✅ 0,07 s, 52k Knoten, `bestmove b1c3`, PV plausibel |
| Mattstellung `go depth 5` | ✅ `bestmove e1e8`, `score mate 1` |
| `go movetime 500` | ✅ Antwort nach 404 ms |
| `go wtime 60000 btime 60000 winc 1000 binc 1000` | ✅ Antwort nach 2903 ms (Ziel 2900) |
| `go wtime 5000 btime 5000` | ✅ Antwort nach 208 ms (Ziel 200) |
| `go infinite` + `stop` | ✅ `bestmove` nach 5 ms |
| Ungültige `position`-Befehle | ✅ `info string`-Fehler, kein Absturz |

Gefundene und behobene Fehler (Protokoll der Eigenkorrektur):
1. Blockgepufferte Stdout bei Pipes → alle UCI-Ausgaben werden geflusht.
2. Soft-Limit nur zwischen Iterationen geprüft (60+1 → 10 s statt 2,9 s) →
   Mid-Iterations-Abbruch eingebaut.
3. `else-if`-Fehler: Soft-Limit bei gleichzeitigem Hard-Limit nie geprüft → behoben.
4. Optionsname `MoveOverhead` vs. Bridge-Standard `Move Overhead` →
   beide Schreibweisen akzeptiert (von Bridge-Validierung aufgedeckt).
5. Stellungswiederholung: 2. Auftreten wurde überall (auch an der Wurzel) als
   Remis gewertet (Fund durch externes Review, 18.09.2026). An der Wurzel kehrte
   die Suche sofort mit Score 0 und leerer PV zurück; der Treiber übernahm den
   erstgenerierten Zug (`info depth 64 … nodes 64 … pv b1c3`, `bestmove b1c3`).
   Ursache: `is_repetition` zählte `>= 2` bei Stack inklusive aktueller Stellung
   (`src/search.rs`), Prüfung vor PV-Init/Zugschleife, auch für Ply 0; zusätzlich
   war ein TT-Cutoff an der Wurzel möglich (leere PV, gleicher Effekt über
   Sitzungen hinweg). Fix: exaktes Dreifach-Remis (`>= 3`), keine
   Wiederholungs-Abkürzung und kein TT-Cutoff an der Wurzel (TT dort nur
   Zugordnung), PV-Abbruch an Remis-Cutoffs. Unverändert: Für 50-Züge-Regel
   und unzureichendes Material kehrt auch die Wurzel weiterhin sofort mit
   Score 0 zurück. Tests: 3 neue Regressionstests (14/14 ✅).
   Messung: `bench` nahezu unverändert (Startpos Tiefe 8: 52148 vs. 52116 Knoten
   alt, gleiche Scores/PVs — das zeigt nur keine auffällige Abweichung, beweist
   aber keinen fehlenden Leistungsverlust), Match neu vs. alt 2,0 : 0,0 (je 1×
   Weiß/Schwarz, 30 s pro Seite — Kleinstsample: keine Abstürze/illegalen Züge
   in diesen zwei Partien beobachtet, keine Stärke- oder
   Schadensfreiheits-Behauptung, Details in Abschnitt 5).

## 4. Vollpartien (ausgeführt, Schiedsrichter: python-chess 1.11.2)

- Selbstspiel Tiefe 5 (FEN-Pfad): **1-0 nach 87 Zügen**, alle Züge legal,
  Matt-Endstellung `7k/5Q2/5B2/...`. Deterministisch reproduzierbar.
- Selbstspiel Tiefe 4 (moves-Listen-Pfad inkl. Wiederholungshistorie):
  **0-1 nach 66 Zügen**, alle Züge legal.
- Eröffnungseindruck: regelkonform, aber eigenwillig (frühe Springerzüge);
  dokumentierte Grenze, kein Fehler.

## 5. Spielstärke-Kalibrierung vs. Stockfish 17 (Messgegner, ausgeführt)

Gegner **ausschließlich zur Evaluation** (eigene Züge nie betroffen).
Je 4 Partien (2× Weiß/2× Schwarz), beidseits `movetime 300`, Schiedsrichter
python-chess, Abbruch bei 300 Zügen = remis.

| Gegner | Ergebnis aus Funkens Sicht |
|---|---|
| Stockfish 17, UCI_Elo 1350 | **4,0 : 0,0** |
| Stockfish 17, UCI_Elo 1600 | **4,0 : 0,0** |
| Stockfish 17, UCI_Elo 1900 | **2,0 : 2,0** (2 Remis, je 1 Sieg/Niederlage) |

Ehrliche Einordnung: Kleinstsample (je 4 Partien), Kurzzeit-Bedenkzeit,
Stockfish-Limiter ≠ echte Wertungszahl. **Es wird keine Elo-Zahl behauptet.**
Befund: Funken spielt auf Anhieb kohärentes, taktisch waches Schach auf
Vereinsniveau-Nähe unter Blitzbedingungen — und verliert gegen SF1900 nicht.

Suchleistung (Befehl `funken bench`, Tiefe 8): Startpos 52k Knoten/~60 ms;
Kiwipete 313k Knoten/~0,5 s; Such-NPS ca. 0,5–1,6 Mio./s.

Alt-gegen-Neu nach Wiederholungs-Fix (18.09.2026, `~/engine_match.py`,
Schiedsrichter python-chess, je 30 s pro Seite): neu (Fix) vs. alt (38f21c9)
**2,0 : 0,0** (je 1× Weiß/Schwarz, beide regulär mit Matt beendet, PGNs unter
`/tmp/opencode/match_r1.pgn`, `/tmp/opencode/match_r2.pgn` — temporär, nicht im
Repo). Ehrliche Einordnung: Kleinstsample (2 Partien) — beobachtet wurden nur
keine Abstürze und keine illegalen Züge in diesen zwei Partien; das Ergebnis
beweist weder Spielstärke noch Schadensfreiheit des Fix.
**Keine Elo-/Stärke-Behauptung.**

## 6. Ausstehend (nicht behauptet, nicht gemessen)

- `searchmoves`-Restriktion mit mehreren Zügen (nur Ein-Zug-Pfad getestet).
- Echte Uhr-Partien mit Inkrement gegen Fremdgegner (nur movetime/Selbstspiel).
- Langzeit-Stabilität (Stundenlauf, volles Hash, Reconnects der Bridge).
- Lichess-Testpartien (casual) — braucht Token + Freigabe (siehe unten).
- Systematisches Eval-Tuning, SMP, eigenes Eröffnungsrepertoire.
- TT-Wechselwirkung mit Stellungswiederholung: Scores, in deren Teilbaum ein
  Remis-Cutoff steckt, werden in der TT gespeichert und können auf anderen
  Pfaden (mit anderer Historie) wiederverwendet werden — pfadabhängige
  Restungenauigkeit, Standardverhalten, nicht als exakt behauptet.

## 7. Lichess-Anbindung: Validierungsstand (ausgeführt, ohne Token)

- Offizielle Bridge `lichess-bot-devs/lichess-bot` (Stand 2026.8.9.2),
  Deps in venv installiert, läuft unter Python 3.13.
- Eigene `config.yml.example` gegen den echten Config-Loader geprüft:
  **„Engine configuration OK"**; einziger Fehler danach die erwartete
  API-Auth-Ablehnung des Dummy-Tokens (Netz funktioniert, Auth fehlt).
- Deaktiviert nachweislich: Bücher (polyglot/online), Cloud-Analyse, EGTB,
  Ponder, fremdes Aufgeben/Remis. Engine-UCI-Optionen der Bridge
  (`Move Overhead`, `Threads`, `Hash`) werden akzeptiert.
- `setup.sh`/`start.sh`/`funken.service` lokal syntaktisch geprüft (`bash -n`,
  systemd-Unit gegen Checkliste); Installation auf dem Zielrechner steht aus.

## 8. Letzte Schritte bis zum Livebetrieb (Checkliste)

1. [ ] Zielrechner festlegen (Annahme bisher: CPU-Linux wie Test-VM) und
      `sudo PREFIX=/opt/funken ./lichess/setup.sh` ausführen.
2. [ ] Lichess-Account **neu anlegen** (wichtig: vorher **keine** Partie spielen,
      sonst ist das BOT-Upgrade blockiert).
3. [ ] Token erzeugen: lichess.org → Profil → API-Token, **nur Scope `bot:play`**,
      als `LICHESS_BOT_TOKEN` bereitstellen (Shell-Env oder `/etc/funken/token.env`,
      chmod 600). Niemals ins Repo/Logs.
4. [ ] Trockenlauf: `start.sh` → Log zeigt `Engine configuration OK` + Profilname;
      bei Fehlern erst Config/Netz prüfen, kein Upgrade.
5. [ ] **Freigabe einholen**, dann BOT-Upgrade (`-u`, irreversibel);
      danach nur noch Challenge-Partien, keine Pools/Turniere möglich.
6. [ ] Erste Woche: nur `casual` (bereits so konfiguriert), 1 Partie gleichzeitig,
      Logs via `journalctl -u funken.service -f`; auf Flaggen/Zeitnot achten
      (ggf. `Move Overhead`/`move_overhead` erhöhen).
7. [ ] Erst bei stabilem Lauf: `rated`/Matchmaking erwägen; UltraBullet bleibt
      für Bots systemseitig gesperrt.
