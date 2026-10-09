# Funken-Roadmap: von Blunder-Analyse zu Stärke (Stand 30.09.2026, abends)

Dies ist das Wiedereinstiegs-Dokument für eine spätere Sitzung: als Prompt
einlesen, dann bei **Empfehlung** fortfahren. Details in `EXPERIMENTE.md`
(Ablauf-Log), Messungen in `MESSERGEBNISSE.md` 9.9–9.18.

## Empfehlung (nächster Schritt)

Stand 30.09. abends — Ernte seit 24.09. (Details `MESSERGEBNISSE.md`
9.13–9.18):

- **Gen5-Tuning verworfen:** 229k Positionen mit Gen4-Engine, Holdout nur
  −1,3 % (Gen4: −4,0 %), M6 −30 n.s. (offen). Texel-Route vorerst
  ausgereizt — keine weitere Iteration ohne neues Signal.
- **QS/SEE-Trilogie vermessen, kein Kandidat:** Diagnose (9.15) belegte
  QS-Blindheit als Mechanismus (36/45 Widerlegungen ruhig, Statik-Gap nur
  108 cp, 32/45 nie gefunden bis 5M). Aber: M7 QS-Checks −10 n.s.
  (kostet einen Ply), M8 SEE +5 pari (kostenlos), M9 Kombi −7 n.s.
  (alle offen). Kapitel zu — Engine **unverändert Gen4** (`959b13e`).
- Offene Hebel danach: Zeitmanagement (gezielte Verlängerung bei
  Eval-Sprüngen; schwächster Hebel — Blunder waren keine Zeitnot),
  oder Pause bis deutlich mehr Gen4-Partien/neue Muster.
- **Eigen-Buch v1 (30.09., M10 +70, LOS 100 %, signifikant):**
  Bot-Übergabe bei Tobias (Binary bauen, `book/funken.book` nach
  `/opt/funken/`, Config, Neustart). Engine-Messung bleibt buchfrei.

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

- QS mit Schachgeboten / SEE: **vermessen 30.09.** (M7 −10 n.s., M8 +5
  pari, M9 −7 n.s. — alle offen, kein Kandidat). Kapitel zu, siehe 9.15–9.18.
  Wiederaufnahme nur mit neuer Idee (z. B. tiefere Checks-Stufen), nicht
  mit denselben Varianten.
- Tuning: Gen5 verworfen (M6 −30 n.s., Holdout nur −1,3 %) — ausgereizt
  ohne neues Signal (stärkere Daten, andere Zielfunktion).
- Zeitmanagement: nur gezielte Verlängerung bei Eval-Sprüngen; Blunder waren
  keine Zeitnot (außer 60+0 Martuni). Schwächster Hebel hier.
  **Vermessen 08.10. (M11 −2 pari, offen, kein Kandidat):** V_TM1 (einmalig
  +1 Basis bei Drop >80 cp, Tiefe ≥6, nur Uhr) misst exakt pari
  (100,5:99,5, CI −32…+35, Tiefe 12:12, 0 Flags). Kapitel vorerst zu —
  Wiederaufnahme nur mit neuer Idee (Schwelle/Mehrfach/Bestmove-Trigger),
  nicht mit denselben Parametern.
- Remis-Vorausschau (06.10., MESSERGEBNISSE 9.20/§6): 50-Züge nur als
  eingetreten gewertet (keine Annäherungs-Dämpfung), kein Remis-Angebot/Claim
  (UCI/Bridge aus) — plus Nebenbefund K+L+S-Konvertierung bei kleinen Knoten
  (h1=N korrekt, danach nicht konvertiert). Je Feature eigene Entscheidung
  mit Messung.
  **08.10.:** WB-Fehlalarm gefixt+committet (`9ec7c2c`, 9.22 — leere Ecke
  maskierte Matt in 4); V_FD1-Dämpfung als Kandidat verworfen (9.23 —
  Sonden höchstens marginal, kein Match per Konstruktion pari).
  **09.10.:** KBN-Führung V_KBN als Bugfix übernommen+committet (9.25 —
  24/24 Tests, Bench/Perft unverändert, 5/20 vs 2/20 Matts bei 1M,
  Tendenz n.s., kein Elo-Plus behauptet).
  Offen: wurzelnäherer Dämpfungsansatz (nur mit neuer Idee),
  Remis-Claim (Protokoll-Thema).

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
