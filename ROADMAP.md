# Funken-Roadmap: von Blunder-Analyse zu Stärke (Stand 23.09.2026, abends)

Dies ist das Wiedereinstiegs-Dokument für eine spätere Sitzung: als Prompt
einlesen, dann bei **Empfehlung** fortfahren. Details in `EXPERIMENTE.md`
(Ablauf-Log), Messungen in `MESSERGEBNISSE.md` 9.5–9.8.

## Empfehlung (nächster Schritt)

Stand 24.09. mittags — Ernte (Details `MESSERGEBNISSE.md` 9.9/9.10):

- **Tuned (params_v3) +56 Elo, LOS 99,8 % (M1, 200 Partien).** Übernahme-
  Kriterium erfüllt. Vorschlag: `FUNKEN_PARAMS`-Default ins Binary
  übernehmen (= Tuned als neue Basis) + per Folge-Match absichern; dann
  Gen4-Modell (params_gen4, −4,0 % Holdout) gegen die neue Basis messen.
  Entscheidung (Rollout/Bot) bei Tobias — nichts ohne Auftrag übernehmen.
- Nullzug-Marge (−21 n.s.) und LMR-late (−3 pari): offen, keine Übernahme.
- Danach erst: QS/SEE-Entscheidung, Zeitmanagement.

Laufend: nichts (alle Jobs beendet). Kein Langlauf ohne expliziten Auftrag.

## Stand (Lesestoff für den Einstieg, 10 Minuten)

- Engine-Verhalten **unverändert** (kein Rollout nötig, Bot spielt wie bisher).
  Alle Varianten waren `/tmp`-Binaries; `search.rs` stets revertiert.
- 55 Blunder/38 Partien (SF 17.1/T17), alle mit Funkens Spiel-Sicht zugeordnet.
  Zeitnot widerlegt (Median Tiefe 13); Wahnmaß Median 272 cp (Eigen-Sicht!).
- Session-Effekt bewiesen (TT-Inhalt flippt Qxf7/Bh7/Qc8), aber TT-Aging kostet
  −108 Elo (Ablation 14/40) — TT-Route beendet, ~+100 Elo TT-Nutzen behalten.
- Texel-Tuning: Pipeline steht (`src/tune.rs`, `tools/texel_{gen,data}.py`,
  `FUNKEN_PARAMS`), v3 neutral (20/40, 50 %) — schwache Labels.
- Pruning-Sweep (RFP/Null/LMR/Futility/Aspiration): kein Schalter holt
  Rettungen in Spiel-Budgets — OFFEN (nur Mikro-Diagnose, kein Stärke-Befund;
  echte Matches ausstehend). LMR-Mikros (23.09.: red1only/late/deep, ≤2M):
  ebenfalls keine Rettung — OFFEN (Match LMR-late ausstehend).
- Label-Filter-Retune (23.09.): 87 % tiefenstabil, Retune −0,2 % Holdout.
  OFFEN: Die Deutung („schwache Labels") stützte sich auf die UNGÜLTIGE
  Tuning-Ablation; Urteil erst nach Tuned-Repeat (≥200, Precheck).
  Gen4 (200k Positionen @200k Knoten) läuft beauftragt.
- Perspektiven (Fehlerquelle!): Blunder-JSON = Eigen-Sicht, PGN-[%eval] =
  Weiß-Sicht, UCI-`info score` = Seite-am-Zug. Details `EXPERIMENTE.md` oben.

## Offene Hebel (nach den Gates)

- QS mit Schachgeboten / SEE (ruhige Ressourcen Qb4/f5!): Feature, 1–2 Wochen
  inkl. Korrektheitsrisiko. Erst nach LMR-Gate.
- Tuning mit 200k-Daten (~10 h Maschine) + Label-Filter.
- Zeitmanagement: nur gezielte Verlängerung bei Eval-Sprüngen; Blunder waren
  keine Zeitnot (außer 60+0 Martuni). Schwächster Hebel hier.

## Mess-Regeln (nicht verhandelbar)

Buch (`~/engine-arena/openings.epd`) + feste Knoten (`-n`), nie Uhr (außer für
Zeitmanagement selbst). n=40 → ±80 Elo (nur Riesen-Effekte, kein
Verwerfungs-Kriterium); n=400–800 → ±20 (eine Nacht). Kein Elo erfinden,
keine Übernahme ohne Messung. Seit 23.09. zusätzlich: **Precheck**
(`tools/precheck.sh`) vor/nach jedem Match Pflicht (Bench-Divergenz + md5,
danach Paar-Check; >10 % identische Paare = ungültig); **Mindestgröße 200
Partien** (>20 Eröffnungen, Zufallsopenings) für Stärke-Aussagen; **nicht
signifikant = offen, nicht tot** — keine Variante wird allein wegen „Gate
negativ" oder „n=40 pari" verworfen.

## Resume-Anleitung

- Repo `/home/librechat/sparkengine`, sauberer Baum erwartet
  (`git status --short` prüfen). Toolchain: `cargo test` (14 Tests),
  `cargo build --release` (0 Warnungen), `./target/release/funken bench`
  (Referenz: 52148/+8, 312538/−31, 38731/+19), `perft 5` = 4865609.
- Daten (dauerhaft): `~/engine-arena/texel-gen{,2,3}-*/` (PGNs),
  `~/engine-arena/ablation-*/` (Matches). FLÜCHTIG: `/tmp/opencode/`
  (Mikro-Skripte, TSVs, Varianten-Binaries) — bei Verlust: TSVs via
  `tools/texel_data.py` neu extrahieren, Varianten via dokumentierte
  Einzeiler aus `EXPERIMENTE.md` neu bauen.
- Hintergrundjobs: keine aktiv (Stand Commit). Neue Langläufe per
  `setsid nohup … & disown` + PID in `EXPERIMENTE.md`, weil Tool-Timeouts
  Prozessgruppen töten können.
- Blunder-Daten: `/home/librechat/voigtsbach_analysen/` (aktualisiert sich
  selbst alle 30 Min; PGNs + `analysen/voigtsbach-blunders.json`).
