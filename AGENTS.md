# AGENTS.md — Hinweise für KI-Agenten in diesem Repo

Dies ist **Funken**, eine eigenständig entwickelte UCI-Schachengine in Rust
(Binary `funken`), plus vorbereiteter Lichess-Bot-Anbindung. Das Projekt ist
ein Experiment in eigenständiger KI-Entwicklung: Der schachliche Kern
(Zuggenerierung, Suche, Bewertung, Zugentscheidung) ist **eigene Implementierung**.

## Harte Regeln (nicht verhandelbar)

1. **Keine Engine-Übernahmen:** Keinen Code bestehender Engines klonen, portieren
   oder übersetzen; keine Wrapper um Stockfish & Co.; keine fremden
   Bewertungsnetze, Gewichtstabellen oder Trainingslabels. Etablierte Verfahren
   aus Lehrbuchliteratur selbst implementieren ist ok — als Bauvorlage dienender
   Engine-Quellcode ist es nicht.
2. **Keine Geheimnisse:** Keine Tokens, API-Keys oder Zugangsdaten in Code,
   Config-Beispiele, Logs oder Commits. Lichess-Token läuft ausschließlich über
   die Env-Var `LICHESS_BOT_TOKEN` (vgl. `lichess/`).
3. **Ehrlichkeit vor Schönfärberei:** Keine Elo-Zahl erfinden, keine Spielstärke
   ohne Messung versprechen. Gemessen vs. ausstehend strikt trennen
   (`MESSERGEBNISSE.md`, Abschnitt 6 pflegen). Schlägt ein Ansatz fehl: Ursache
   analysieren, selbst ändern, knapp protokollieren.
4. **Spielbetrieb = eigene Berechnung:** Externe Engines/Cloud/Bücher/Tablebases
   nur als klar getrennte Testreferenz (z. B. `tests_match.py`, python-chess als
   Schiedsrichter), niemals als Zugquelle im laufenden Betrieb.

## Bauen, Testen, Messen

```bash
cargo build --release          # Binary: target/release/funken
cargo test                     # 11 Unit-Tests (müssen alle grün sein)
./target/release/funken perft 5           # Referenz: 4865609
./target/release/funken bench             # Mini-Benchmark
python3 tests_ucitool.py ./target/release/funken       # UCI + Zeitverhalten
python3 tests_selfplay.py ./target/release/funken 5 2  # Selbstspiel (Referee: python-chess)
```

Vor jedem Commit: `cargo test`, Release-Build ohne Warnungen
(`cargo build --release` → 0 warnings/errors), Perft-Stichprobe.

## Architektur (kurz)

- `src/chess.rs` — Mailbox-Board `[u8;64]` (a1=0..h8=63), Pseudo-legal + volle
  Legalitätsprüfung via make/unmake, inkrementeller Zobrist-Hash, FEN, Perft.
- `src/eval.rs` — eigene Tapered-Eval (Weiß-sicht, cp); Tabellen werden
  programmatisch aus Zentralisierungs-/Vormarsch-Regeln erzeugt, nicht kopiert.
- `src/search.rs` — iterative Vertiefung, Alpha-Beta, TT, Quieszenz, Nullzug,
  LMR, Killer/History, Zeitlimits (weich/hart, Poll alle 2048 Knoten).
- `src/uci.rs` — UCI-Schleife, Suche im Worker-Thread; **jede Ausgabezeile wird
  geflusht** (Pipes sind blockgepuffert — nie `println!` ohne Flush verwenden).
- `src/main.rs` — Dispatch: UCI (default), `perft <tiefe> [fen]`, `bench`.
- `lichess/` — Bridge-Anbindung (offizielles `lichess-bot`), Beispiel-Config mit
  allen Fremdquellen deaktiviert, `setup.sh`/`start.sh`/`funken.service`.

## Konventionen

- Engine-Code: Rust, nur std (keine Dependencies in `Cargo.toml` — bitte so lassen).
- Single-threaded ist eine bewusste Entscheidung (s. `KONZEPT.md`); `Threads`
  bleibt fix 1, bis SMP sauber implementiert + getestet ist.
- Neue Heuristiken/Gewichte: erst als Experiment mit Messung (Bench + Match vs.
  Stand), dann übernehmen oder verwerfen — Andersartigkeit ist kein Selbstzweck.
- Doku aktuell halten: `KONZEPT.md` (Warum), `MESSERGEBNISSE.md` (Was gemessen),
  `README.md` (Wie). Deutsch, knapp, faktenbasiert.
