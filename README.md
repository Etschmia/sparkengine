# Funken 1.0 — eigenständige UCI-Schachengine

**Funken** ist eine von Grund auf selbst implementierte Schachengine (kein Clone,
kein Port, kein Wrapper; keine fremden Netze oder Labels; Tabellen/Bücher als
reine Daten erlaubt, derzeit nicht eingebunden).
Autorin des Entwurfs und der Implementierung: **Muse Spark** (KI).
Getestet und validiert auf Debian Linux (x86_64, CPU ohne GPU).

- Sprache: **Rust** (stabil, single-threaded, keine Laufzeitabhängigkeiten)
- Protokoll: **UCI** (‪+ `perft`- und `bench`-Unterkommandos zur Selbstprüfung)
- Spielbetrieb: Zugentscheidung aus eigener Suche/Bewertung (derzeit ohne Buch/Tablebase);
  Lichess-Anbindung nur über die offizielle Bridge **lichess-bot**
  (Fremdquellen im Beispiel derzeit aus, optional)

## Schnellstart (Linux)

```bash
cargo build --release
./target/release/funken            # UCI-Modus (z. B. unter cutechess, Arena via Wine entfällt, lichess-bot)
./target/release/funken perft 5    # Regel-Selbsttest mit Referenzwerten
./target/release/funken bench      # Mini-Benchmark (feste Tiefen/Stellungen)
```

UCI-Optionen: `Hash` (1–1024 MB, Default 64), `Move Overhead` (ms, Default 100),
`Threads` (fix 1 — Engine ist single-threaded; wird akzeptiert, aber gemeldet).

Zeitsteuerung: `wtime/btime/winc/binc`, `movestogo`, `movetime`, `depth`,
`nodes`, `infinite` + `stop`. Formel: Grundzeit − Overhead, weich/hart getrennt
(siehe `KONZEPT.md`).

## Repo-Übersicht

| Pfad | Inhalt |
|---|---|
| `src/chess.rs` | Schachkern: Brett, Zugerzeugung, make/unmake, FEN, Zobrist, Perft |
| `src/eval.rs` | Eigene Tapered-Eval (Material + generierte PST + Struktur + Mobilität) |
| `src/search.rs` | Iterative Vertiefung, Alpha-Beta, TT, Quieszenz, Pruning/Reduktionen, Zeit |
| `src/uci.rs` | UCI-Schleife (Worker-Thread, `stop`-fähig, geflushte Ausgabe) |
| `src/main.rs` | CLI-Dispatch: UCI / `perft` / `bench` |
| `tests_*.py` | Testtreiber (UCI, Selbstspiel, Match) — Werkzeuge, kein Spielbetrieb |
| `lichess/` | Bot-Anbindung: `config.yml.example`, `setup.sh`, `start.sh`, `funken.service` |
| `KONZEPT.md` | Entwurfsentscheidungen, eigene vs. übernommene Verfahren, Grenzen |
| `MESSERGEBNISSE.md` | Alle Messungen (ausgeführt vs. ausstehend, klar getrennt) |

## Tests (lokal, reproduzierbar)

```bash
cargo test                       # 14 Unit-Tests (Perft, Sonderfälle, Matt, Remis)
./target/release/funken perft 5  # 4865609 (Startpos, Referenz)
python3 tests_ucitool.py ./target/release/funken    # UCI + Zeitverhalten
python3 tests_selfplay.py ./target/release/funken 5 2  # Selbstspiel, python-chess als Schiedsrichter
```

Details und alle Zahlen: `MESSERGEBNISSE.md`.

## Lichess-Betrieb (vorbereitet, noch nicht live)

Annahme: gewöhnlicher CPU-Rechner ohne GPU, Debian/Ubuntu, Python ≥ 3.10.
Die Bridge-Entscheidung und alle Schritte stehen in `lichess/` bzw. unten.

**Letzte Schritte bis zum Livebetrieb** (ausführlich in `MESSERGEBNISSE.md`):

1. `sudo PREFIX=/opt/funken ./lichess/setup.sh` auf dem Zielrechner ausführen.
2. Lichess-Account anlegen (noch **keine** Partie spielen) und Token mit Scope
   `bot:play` erzeugen.
3. Token **nur** als Umgebungsvariable/`token.env` bereitstellen (nie ins Repo).
4. Trockenlauf prüfen: Bridge startet Engine (`Engine configuration OK` im Log),
   scheitert nur an der API-Auth, bis das echte Token gesetzt ist.
5. Nach Freigabe: BOT-Upgrade (`python lichess-bot.py -u`, **irreversibel**),
   zuerst `casual`-Partien, Logs beobachten (`journalctl -u funken`).
6. Erst bei stabilem Betrieb ggf. `rated`/Matchmaking erwägen.

## Lizenz

MIT (siehe `LICENSE`). Eigenständige Implementierung; Inspirationen aus
Fachliteratur sind in `KONZEPT.md` offengelegt.
