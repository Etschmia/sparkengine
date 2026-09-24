# Experimente: Voigtsbach-Blunder (Funken)

Sitzungsprotokoll — Zweck: Bei Abbruch der Sitzung ist hier der exakte Stand
(Plan, laufende Versuche, Zwischenergebnisse, offene Fragen) nachlesbar.
Messungen landen zusätzlich knapp in `MESSERGEBNISSE.md`, Unvermessene
bleiben hier.

**Regeln:** Keine Codeänderung ohne Messung (Buch + feste Knotenzahl,
9.1-Verfahren). Varianten nur als separate Binaries in `/tmp`
(`cargo build --release` nach Patch, danach Revert, nie committen).
Ausgangspunkt: `MESSERGEBNISSE.md` 9.5/9.6 (55 Blunder, 38 Partien).

**PERSPEKTIVEN-KONVENTIONEN (22.09.2026, verifiziert — hier festgehalten,
weil der Fehler schon einmal passiert ist):**
- Blunder-JSON (`eval_before/after_cp`, `loss_cp`): **Eigen-Sicht** (immer
  Voigtsbach). Beweis: Nxe4 (Voigtsbach schwarz): JSON −114 → −355;
  lokaler SF 17.1: Root +123, nach Nxe4 +358 (Weiß-Sicht) = Eigen −123/−358.
  Passt exakt; Weiß-Sicht-Lesart (−114 → −355 = Verbesserung!) absurd.
- Bridge-PGN `[%eval]`: **Weiß-Sicht** (Beweis: Qc8 +10,41 bei Matt gegen
  Schwarz; Bxh3 −8,12 bei Schwarz-Gewinn → Partie 0-1).
- Funken-UCI `info score cp`: **Seite-am-Zug-Sicht** (`score_to_uci` ohne
  Konvertierung, `src/uci.rs:213`). In FEN-Proben (am Zug immer Voigtsbach)
  = Eigen-Sicht — direkt mit JSON vergleichbar. In PGN [%eval] Weiß-Sicht.
- FEHLER vom 22.09. (korrigiert): JSON als Weiß-Sicht gelesen → Wahnmaß und
  „Vorzeichenflip" (Bxh3/h4) für Schwarz-Partien falsch. Korrekt: Median 272
  (statt 244), p90 468 (statt 660); Bxh3/h4 waren Zustimmung, kein Flip.
  Nxe4-H2H war KEIN „feines Gefälle": Bc7 −53 vs. Wahrheit −123 (Lücke 70),
  Nxe4 −22 vs. Wahrheit −358 (Lücke 336!) — echte Fehleinschätzung, E2-KERN.

## E1 Matt-Erkennung (`b7` statt `Ke5`, Xa1961ka)

- Stellung: `8/5k2/1Pp3p1/7p/3K3P/8/5PP1/8 w - - 0 42`, SF: Matt → +8,97.
- Spiel-Sicht: Tiefe 16, +14,14, 86 s Rest, 5,0 s Bedenkzeit. Auch Tiefe 16
  sieht kein Matt. Probe Tiefe 12: dito.
- Hypothese: Schach-Verlängerung (1 Ply) reicht nicht / Mattdistanz-Pruning
  oder Aspiration schneidet Mattäste; Quieszenz ohne Matt-Erkennung bei
  ruhigen Knoten.
- Status: OFFEN.

## E2 Hängende Figuren (`Nxe4`, `Nb5`, `Qxd3`-Variante)

- `Nxe4`: d15, +0,11 bei 11,2 s / 234 s Rest — reine Eval-Blindheit oder
  ruhige Tiefe zu flach ( SF −114 → −355 … Notation Weiß-Sicht, s. 9.6).
- `Nb5`: d12, +1,78 (SF danach 0) — Überschätzung.
- `cxd3` statt `Qxd3`: Läufer hängt wird gesehen, falsche Figur nimmt.
- Kandidaten: Quieszenz-Tiefe/Delta-Pruning, SEE fehlt ganz, ruhige
  LMR-Reduktionen.
- Status: OFFEN.

## E3 Rettung weggeschnitten (`Ng3`, `Qc8`, `Kc4`-Typ)

- `Ng3`: d12, −3,58 — wusste um −3,5 und spielte es trotzdem. Rettung `Qxe4`
  nicht gefunden. Kandidaten: Nullzug, LMR, Futility/RFP, Aspiration.
- `Qc8`: d14, +10,41 — Verlust erkannt, `Qxf5` nicht gefunden.
- Methode: je Pruning einzeln abschalten, `go nodes` fix auf der Stellung,
  prüfen ob Rettung auftaucht und zu welchen Knotenkosten.
- Status: OFFEN.

## Statik-Audit (22.09., Temp-Binary `/tmp/opencode/funken-eval`, Revert ok)

Methode: `evaluate()` (Weiß-Sicht) vs. SF 17.1 d20–24 auf Wurzeln, Blättern
und entlang SF-PV. Alle Gaps Weiß-Sicht-cp (Funken-statisch vs. SF):

| Fall | Wurzel stat. | Wurzel SF | Blatt stat. | Blatt SF | Deutung |
|---|---|---|---|---|---|
| Nb5 | +104 | +272 | +77 | 0 | Aufbau −168 diffus + Zug-Optimismus |
| Nxe4 | −6 | +114 | +215 | +344 | f5!-Ressource: stat −129 + QS-Rest |
| Qxf7 | +124 | +663 | +618 | 0 | ruhiges Qb4: stat −618, Tiefen-Domäne |
| Ke2 | +60 | 0 | — | — | Statik OK → Pruning (Nullzug) |
| Rd6 | −214 | +431 | — | — | Angriff −645, Suche holt 220 bis d14 |

- Nb5-PV-Walk (d3/Re1/Be3-Plan, SF +270…+351): Funken-Statik flach +93…+137,
  bricht ply8–10 auf +24/+8/+37 ein (…Rxb2-Bauernraub ohne Fallenblick).
  Kein einzelner Term — Aufbau + Falle brauchen Tiefe + Skala.
- Rd6 (`2kr1b2/...`, Schwarz am Zug): Material sagt Schwarz +230 (Läuferpaar
  inkl.), SF +431 Weiß (Qa7-Invasion). Größte statische Lücke (−645).
  King-Shield-Vorzeichen geprüft (Z.343–347): korrekt (Weiß-relativ).
  Kein Term-Bug gefunden — Angriffs-Skala fehlt strukturell (Mobilität Q×0,5,
  PST ±12: nichts trägt Hunderte).
- FAZIT Audit: `evaluate()` ehrlich aber flach (Material + Mikro-Terme, keine
  Angriffs-/Ungleichgewichts-Skala). Kein Hand-Patch trägt das — Hebel ist
  systematisches Tuning (SPSA, s. KONZEPT) + billigere Tiefe. Kein Bug.

## Systematisches Tuning (SPSA/Texel, ab 22.09.)

Motivation: Statik-Audit (s.o.) — `evaluate()` ehrlich aber flach, kein
Hand-Patch trägt. Alle Daten selbst erzeugt (keine fremden Labels).

- T1: Eval-Params als Struct (39 Knöpfe: PST-Regelkoeffizienten, MOB,
  Struktur, Läuferpaar, Linien, Schild, Tempo). Material (PIECE_VALUE)
  und PHASE_W bewusst FIX (Suche teilt sich PIECE_VALUE für MVV-LVA/Delta —
  kein Nebeneffekt-Risiko). Verhalten identisch (Tests + Bench).
- T2: Daten per Selbstspiel (engine_match.py, feste Knoten) → FEN+Resultat,
  nur ruhige Stellungen (kein Schach, ab Zug 12). Kleine Menge zuerst.
- T3: Tuner als Dev-Subkommando (perft/bench-Präzedenz, std-only):
  Tables pro Kandidat einmal bauen, Koordinaten-Abstieg auf Texel-MSE.
- T4: Holdout-Fehler + Ablation tuned vs. Basis (9.1). Erst dann übernehmen.
- Pruning-Experimente (Nullzug-R etc.) danach — nicht vermischen.

## Ablauf-Log

- 22.09.2026: E1–E3 angelegt. Werkzeug verifiziert (`go nodes` in
  `src/uci.rs:187`, `engine_match.py -n`, `openings.epd` vorhanden).
- 22.09.2026: Suchkern gelesen (`src/search.rs`, 1006 Zeilen). Mechanik-Notizen:
  RFP (Z.480, `static-90*depth >= beta`, Tiefe ≤ 4) und Nullzug (Z.485,
  `static >= beta`, R=2–3) geben in Gewinnstellungen schnell „gut genug"
  zurück — der Mattast wird nie bestätigt (klassischer Mechanismus für
  „findet Gewinnzug, aber nicht Matt"). Mattdistanz-Pruning nur Lower-bound
  (Z.417, harmlos). QS nur Schlagzüge + Umwandlungen, Delta +200 (Z.339):
  ruhige Drohungen (z.B. f5 gegen hängenden Läufer) sind hinter dem
  Nominalhorizont unsichtbar — E2-Mechanismus. Aspiration ±25/±120 (Z.786).
- 22.09.2026: Baseline-Mikro (`/tmp/opencode/micro_nodes.py`, frischer Prozess,
  `go nodes` 200k/1M/5M, 8 Stellungen, Binary `/tmp/opencode/funken-base`):
  b7/Ng3/Nb5/Ke2 frisch reproduziert (b7 selbst bei 5M/d15 kein Matt);
  Nxe4 erst bei 5M/d14 (bei 200k/1M: Na6); Qc8 frisch = g6g8 (Spielzug Qc8
  nur mit Spielzustand — zustandsabhängig); Qxf7 frisch = Rxa1 schon bei
  200k (Spielzug Qxf7 bei d16 — stark zustandsabhängig, TT/Historie-Verdacht
  9.3); Rc3 frisch = Rxa3/Rd3 (SF: Ree8). Rohdaten: `/tmp/opencode/micro_*.json`.
- 22.09.2026: E1-Varianten V1 (ohne RFP), V2 (ohne Nullzug), V3 (ohne beides)
  auf b7-Stellung bei 1M/5M Knoten — läuft.
- 22.09.2026: V1–V3-Ergebnisse (Vollmatrix 8 Stellungen × 200k/1M/5M,
  Rohdaten `/tmp/opencode/micro_*.json`, Binaries `/tmp/opencode/funken-*`):
  - b7: KEINE Variante findet Matt (V1/V2/V3 bei 5M: b7, +1250–1380).
    Weder RFP noch Nullzug ist der Matt-Verstecker. Nächster Kandidat: LMR.
  - Ke2: V2 und V3 finden bei 5M `d1d8` (= SF-Bestzug Qd8+, +7/+0 statt
    +30/+25 bei Ke2) — der NULLZUG hat die Rettung weggeschnitten (V1 ohne
    RFP: kein Effekt). Kosten: erst bei 5M sichtbar (1M noch Ke2).
  - Qc8: V1 (ohne RFP) spielt bei 1M/5M `c5c8` = SPIEL-Blunder (−896/−1239)!
    Mit Nullzug aus (V2) bzw. beidem aus (V3) wieder g6g8. Lehre: RFP trägt
    hier den guten Zug; Pruning-Wechselwirkungen nicht-monoton — Vorsicht
    bei Einzel-Ablationen.
  - Nxe4: V1 spielt f6e4 schon bei 200k (Basis erst bei 5M: dort Na6) —
    RFP bremste den Fehlzug bei kleinen Budgets.
  - Qxf7: V3 bei 200k `e7f7` (+540, Blunder), ab 1M Rxa1 — Flachsuch-Artefakt
    im frischen Zustand (Spiel: d16 trotzdem Qxf7 → zustandsabhängig).
  - Ng3/Nb5/Rc3: alle Varianten wie Basis (Ng3, Nb5, Rxa3/Rd3 statt Ree8).
- 22.09.2026: E1-Fortsetzung: V4 ohne LMR auf b7 + Tiefensuche Basis mit
  großem Budget (findet Basis das Matt je?). PV von Ke5 inspizieren.
- 22.09.2026: V4 (ohne LMR): b7 weiter b6b7 (5M, +1373) — auch LMR ist es
  nicht. Basis mit 20M (d16, +1405): weiter kein Matt. E1 ist STRUKTURELL,
  kein Budget-Problem.
- 22.09.2026: UCI-Befund: einzelnes `searchmoves` antwortet SOFORT ohne Suche
  (`src/uci.rs:477-480`) — Solo-Scores erfordern Paare. Neues Werkzeug
  `/tmp/opencode/h2h.py` (Rettung vs. Spielzug, 1M/5M/20M):
  - b7: Ke5 gewinnt NIE (≤20M/d17). Matt strukturell unsichtbar.
  - Ng3: Rettung gewinnt erst bei 20M (−658; Ng3 kollabiert tiefer). → Tiefe,
    kein Bug. Richtung: billigere Tiefe (Pruning-Tuning).
  - Nxe4: 1M Bc7 (−53 eigen), ab 5M Nxe4 (−22 eigen). Wahrheit (SF):
    Bc7 −123, Nxe4 −358 (eigen). Lücken: 70 vs. **336** — keine „feine"
    Präferenz, sondern echte 336cp-Fehleinschätzung der Bxe4-f5-Abwicklung
    (Statik-Anteil 129, Rest QS). E2-KERN, repriorisiert (s. Konventionen).
  - Nb5 stabil +190 (SF 0): Eval-Optimismus ~190 cp, ebenfalls Tuning-Gebiet.
  - Ke2: Rettung gewinnt bei 20M (0,00). Mit V2 schon bei 5M. → Nullzug
    versteckt Qd8+. Kandidat V5: reduzierter Nullzug (R=1/2).
  - Qc8: 1M Qc8 (−1219), ab 5M Qxf5 (−1742; Qc8 tiefer als Matt entlarvt).
    → Tiefe.
- 22.09.2026: Nächste: V5 (Nullzug R=1/2) auf Ke2-Matrix; E1: Ke5-Absolutscore
  (Paar gegen klar schlechten Zug) + Schachverlängerung 2.
- 22.09.2026: V5 (Nullzug R=1/2): Ke2 → d1d8 bei 5M (+6, d14) wie V2, sonst
  wie Basis. Aber: Spiel-Budget (~2M) liegt unter der 5M-Schwelle — V5 fixt
  den Spielfall voraussichtlich NICHT. Mechanismus verstanden, kein
  billiger Fix. V4-Nachlese: Ke2 ohne LMR flippt mit Budget (200k d1d8,
  1M Ke2, 5M d1d8) — Horizont-Oszillation, kein Fix-Kandidat (Ng3 dort
  schlechter: Kg1 −525 bei d10).
- 22.09.2026: E1-PIVOT: Stockfish (lokal, reine Messreferenz) sagt
  Ke5 = Matt in 8 (d17 erzwungen wie d30 Vollsuche). Funken-H2H: Ke5 gewinnt
  nie ≤20M/d17. Mechanismus: Matt nie bewiesen + b7 (+14) höher bewertet —
  kein Bug, zwei Gewinnwege. Praktisch: Partie wurde GEWONNEN (1-0),
  loss 98993 ist Metrik-Artefakt. E1-b7 → DEPRIORISIERT (Kosmetik).
  Echter Matt-Fall bleibt Qc8 (erlaubt Matt, Partie verloren — war aber mit
  −785 schon verloren; auch dort begrenzter Hebel).
- 22.09.2026: Höchster Praxiswert: JdclEX3n-Remis (1/2) aus +640 (Bh7 + Qxf7).
  Historien-Test (moves-Liste, frische TT): KEIN Effekt — beide d16 Rxa1.
  Befund aus `src/uci.rs:495`: TT persistiert über die Session (alter Tisch
  wandert in den Worker). Neue Hypothese: TT-Verschmutzung (9.3).
- 22.09.2026: SESSIONSSIMULATION (`/tmp/opencode/session_sim.py`: ein Prozess,
  30×500k Füllsuchen entlang der Partie, dann 5M): Entscheidung **e7f7
  (+3,72)** = SPIEL-Blunder (frisch: Rxa1 +3,82; Spiel: +4,68). PV zeigt
  Dame-Ge pendel (a3b4/d2e2/…) mit überhöhtem Score — TT-Signatur.
  9.3-Mechanismus als Ursache BESTÄTIGT (Prinzip; welcher Eintrag genau: offen).
- 22.09.2026: Nächste: V6 (kein TT-Store nach Repetitions-Cutoff, 9.3-Plan)
  in Sessionssimulation — falls Rxa1, Fix-Kandidat für Ablation.
- 22.09.2026: V6 gebaut (rep_taint-Feld, Save/Restore pro Frame, Pfad-
  Propagation; nur Repetition, nicht 50-Züge/Material). Tests 14/14 grün,
  0 Warnungen. Ergebnis: Sessionssimulation weiter **e7f7 (+465, d16)** —
  Repetitions-Taint ist NICHT der (alleinige) Mechanismus.
- 22.09.2026: TT-Clear (`ucinewgame`) vor der Entscheidung → **Rxa1 (+382)**.
  Ursache zu 100 % TT-INHALT (nicht Historie/Binary/Uhr). V6 deckt den
  Mechanismus nicht ab → Restkandidaten: alte Bounds (Lower/Upper aus
  früheren Fenstern), reine Order-Pfadabhängigkeit bei festen Knoten,
  Halbzuguhren-Blindheit (Hash ohne halfmove). Frisch: Qxf7 wird niedrig
  überschätzt (+540 bei 200k), ab 1M Rxa1 — TT konserviert die
  Überschätzung über 5M hinaus.
- 22.09.2026: Nächste (billig,mechanisch): Bh7-Session (gleiche Fills,
  Entscheidung Ply 55); H2H Qxf7/Rxa1 frisch @20M (wahre Lücke?) und in
  Session @5M (flippt TT auch das Paar? → Bounds vs. Ordnung).
- 22.09.2026: Bh7-Session → **g6h7 (+634)** = Spielzug (Spiel: +468/d14).
  Frisch-H2H Qxf7/Rxa1 @20M → Rxa1 (+389). Stand: Session flippt Qxf7, Bh7
  (beide JdclEX3n, Remis aus +640).
- 22.09.2026: SESSION12 (`/tmp/opencode/session12.py`, 12 Fälle, Fills 500k,
  Entscheidung 5M): Session reproduziert **8/12 Spielzüge** (b7, Ng3, Nxe4,
  Nb5, Ke2, Qc8, Qxf7, Bh7). Entscheidend: Qc8 (frisch g6g8 → Session Qc8),
  Qxf7 (frisch Rxa1 → Session Qxf7), Bh7 — drei Session-Flips zu Blundern.
  Rest (Rc3/Rg5/Bf4/Rg4): Session = frisch ≠ Spiel (dort: Uhr/Dynamik oder
  Binary-Differenz — offen, nicht TT). Rohdaten `/tmp/opencode/session12.json`.
  Erster session12-Lauf war Off-by-one (Stellung NACH dem Zug) — verworfen,
  rerun mit Assert `moves[k]==played`.
- 22.09.2026: Deutung: Überlappende Teilbäume über Züge → alte Scores/Bounds
  (andere Fenster/Historie) + Order-Pfadabhängigkeit. V6 (Rep-Taint) greift
  nicht → Bounds/Ordnung. Nächste: V8 Generationen-Aging (Scores nur aus
  laufender Generation, Zug immer) → Session12-Retest; dann Ablation.
- 22.09.2026: V8 implementiert (gen-Feld, cur-Generation, new_search() in
  setup_limits, Cutoffs nur bei gen==cur, alte Züge nur Ordnung, freies
  Überschreiben verfallener Einträge). Tests 14/14 grün, 0 Warnungen.
  Session12 mit V8: Qc8 → g6g8 (FLIP GEFIXT), Bh7 → Qxd7 +450 (FIXIERT, gut),
  Qxf7 bleibt e7f7 (+464). V8 heilt Bounds-Flips, nicht Ordnungs-Flips.
- 22.09.2026: Session-H2H (Paar Qxf7/Rxa1 in Session, Basis): e7f7 +372 —
  auch das erzwungene Paar flippt → Ordnungs-/Bounds-Effekt IM Teilbaum,
  keine Root-Ordnung unter 30 Zügen nötig. Frisch-H2H @20M: Rxa1 +389.
- 22.09.2026: ABLATION läuft: Basis vs. V8, 40 Partien, Buch openings.epd,
  -n 200000, `~/engine-arena/ablation-ttage-200k/` (run_series.sh). Frage:
  Kostet Aging Stärke? Falls V8 ≈ Basis: übernehmen (Bounds-Flips geheilt,
  Ordnung bleibt). Qxf7-Chaos (nahe Alternativen + unglückliche Ordnung)
  bleibt akzeptiert — nur Voll-Clear (V7) hülfe, zu unbekannten Kosten.
- 22.09.2026: ABLATION Basis vs. V8: **14,0/40 (35 %) aus V8-Sicht, Elo −108,
  95-%-CI −230…−7, LOS 1,8 %**. Klare Regression — V8 TOT. Lehre:
  zügeübergreifende TT-Scores/Bounds sind ~100 Elo wert (effektiv +1–2 Plies);
  Flips sind ihr seltener Preis. (`apply_v8v9.py` reproduziert V8/V9.)
- 22.09.2026: V9 (alte EXACT weiter cutfähig, nur Bounds altern): Session12 →
  Qc8 GEFIXT (g6g8), Bh7 ZURÜCK zum Blunder (g6h7 — alte EXACT!), Qxf7 weiter
  Blunder. V9 heilt nur Qc8 (ohnehin verlorene Partie, −785 vorher) → keine
  Ablation wert (Kosten-Nutzen). Session-H2H Qxf7/Rxa1 (Basis): e7f7 +372 —
  auch das Paar flippt → Effekt sitzt IM Teilbaum (Ordnungs-geimpft).
- 22.09.2026: PIVOT-ENTSCHEIDUNG (Kosten-Nutzen): TT-Route beendet. Bewiesener
  Flip-Schaden: ½ Punkt (JdclEX3n-Remis) + Qc8 (verloren ohnehin) aus 38
  Partien — gegen ~+100 Elo TT-Nutzen. Weiter an E2/E3-Qualität (frisch
  reproduzierbar, TT-unabhängig): Ng3 (Tiefe: Rettung erst 20M), Nxe4
  (336cp-Fehleinschätzung Bxe4-f5: Statik 129 + QS, E2-KERN), Nb5
  (Wurzel −168 + Zug-Optimismus ~80), Ke2 (Nullzug versteckt Qd8+ bis 5M),
  sac-Überschätzung (Qxf7-nach +618 statisch vs. 0 — ruhiges Qb4 unsichtbar,
  Tiefen-Domäne). Nächste: SF-PV entlanggehen (Nb5-Wurzel: wo divergiert
  Funken-Statik?).
- 22.09.2026: T1 DONE: EvalParams-Struct (39 Knöpfe, Material/PHASE_W fix),
  `evaluate_with(b, p, t)`, `build_tables(p)`. Verifikation: 14/14 Tests,
  0 Warnungen, Bench bit-identisch (52148/+8, 312538/-31, 38731/+19).
  Bau-Zwischenfall (Edit fraß mobility-Signatur) bemerkt und repariert.
- 22.09.2026: T2 läuft: 100 Selbstspiel-Partien (-n 50000, Buch) als
  Tuning-Daten (v1, klein).
- 22.09.2026: T2 DONE: 100 Selbstspiele (-n 50000, Buch) → 3140 Positionen
  (train 2642, holdout 498; Filter ply>=24, kein Schach, dedup).
  Verteilung schief (Holdout Schwarz-lastig — v1, klein, notiert).
- 22.09.2026: T3a DONE: Tuner `funken texel-tune` (Koordinaten-Abstieg,
  Sigmoid 1/(1+10^(-s/400)), 39 Knöpfe) + FUNKEN_PARAMS-Ladung (Warnung bei
  Fehler, sonst STANDARD). Tests 14/14, 0 Warnungen, Bench identisch.
  MSE STANDARD: train 0,10023 / hold 0,08136. Override-Nachweis: tempo 50
  ändert Bench (52148→1119304 Knoten — Eval-Sensitivität belegt).
- 22.09.2026: T3b läuft: 15 Sweeps auf v1-Daten.
- 22.09.2026: T3b v1-ERGEBNIS: Train 0,10023 → 0,07152, Holdout 0,08136 →
  0,12170 (monoton schlechter). LEHRBUCH-OVERFIT: 2642 rauschige Positionen
  (50k-Selbstspiel) für 39 Knöpfe zu wenig. Richtungen wild (knight_base_mg
  12→−33 u.a.) — kein Wert, nur Signal. KEINE Übernahme (Holdout schlechter
  ab Sweep 1). Extractor v2: Stellungen nach Schlag/Umwandlung raus.
- 22.09.2026: T2v2 läuft über Nacht: 1000 Partien (-n 50000, Buch),
  `~/engine-arena/texel-gen2-50k/`, PID 3687052, Log /tmp/opencode/texel_gen2.log.
  Danach: Extractor v2 → ~30k Positionen → Tuning mit Early-Stopp (Holdout).
  Resume: `tail -2 /tmp/opencode/texel_gen2.log`, bei „Serie beendet":
  `python3 tools/texel_data.py ~/engine-arena/texel-gen2-50k <train> <hold>`.
- 22.09.2026: v2-Datensatz zu klein (1000 Spiele → nur 2507 Positionen:
  64k Duplikate — deterministische Engine + 40 Buchstellungen = ~80
  distinkte Partien). Fix v3: 1500 Spiele aus 6-Halbzug-Zufallsopenings
  (Seed 20260922), tools/texel_gen.py. Stolpern: doppelter Generator
  (fehlendes mkdir + Timeout-Kill) bereinigt, läuft (PPID 1, setsid).
- 23.09.2026: T3b v3-ERGEBNIS: Train 0,08119 → 0,07656, Holdout 0,07703 →
  0,07428 (25 Sweeps, monoton gemeinsam, kein Overfit). 33/39 Params bewegt
  (u.a. Mobilität hoch, Läuferpaar-eg 40→−19, Turm-7.Reihe 10→54).
- 23.09.2026: T4 ABLATION Basis vs. Tuned (Wrapper FUNKEN_PARAMS, 40 Partien,
  Buch, -n 200000, `../engine-arena/ablation-tuned-200k/`): **20,0/40
  (50,0 %)** — exakt pari, Elo ±0. Texel-MSE (−3,6 % Holdout) übersetzt sich
  NICHT in Spielstärke (Suche wäscht Statik-Differenzen; schwache Labels).
  KEINE Übernahme (Gewinn unbelegt). Tuning-Kapitel: Pipeline steht,
  v3 neutral; nächste Runde bräuchte stärkere Daten (200k) — zurückgestellt.
  Pruning-Experimente (vereinbart: zum Schluss) als Nächstes.
- 23.09.2026: PRUNING-MIKROS (8 Stellungen × 200k/1M/5M, frisch): P1 ohne
  Futility = Basis überall (Ke2 weiter Ke2). P2 Nullzug mit Marge +150 =
  wie V2/V5 (Qd8+ erst 5M, nicht Blitz-Reichweite); Qxf7-200k schon Rxa1.
  P3 ohne Aspiration = neutral bis schlechter (Qc8-5M: Blunder c5c8 —
  Aspiration trug dort mit, wie RFP). Voller Switch-Sweep (RFP/Null/LMR/
  Futility/Aspiration): KEINE Variante holt Rettungen in Spiel-Budgets
  (≤2M). Keine Ablation (nichts zu übernehmen). Pruning-Kapitel zu (billig).
  Resthebel (Projekte): Such-Effizienz global, stärkere Tuning-Daten,
  Zeitmanagement.
- 23.09.2026: GATE 1 LMR-MIKROS (ROADMAP-Empfehlung 1): drei milde Varianten
  als /tmp-Binaries (danach revertiert, Baum sauber): red1only (kein red=2),
  late (red1 erst legal>5, red2 erst legal>12), deep (red1 erst Tiefe≥4,
  red2 erst Tiefe≥7). Matrix 8 Stellungen × 500k/1M/2M, frische Prozesse,
  Werkzeug `/tmp/opencode/micro_lmr.py`, Rohdaten
  `/tmp/opencode/micro_lmr.json`. ERGEBNIS: **keine Rettung** (a4e4/c5f5/d1d8)
  in keiner Variante bei ≤2M — alle spielen wie Basis (Ng3→e2g3, Ke2→f3e2,
  Qc8→g6g8; Nxe4→f6e4 erst bei 2M wie Basis). Nur Score-/Tiefen-Diffs
  (±1 Ply). GATE 1 NEGATIV — keine Ablation, LMR-Kapitel zu.
  Nebenbefund: erster Mikro-Lauf mit unrevertiertem Binary (deep als „Basis")
  verworfen und nach Rebuild (Bench-Referenz verifiziert) wiederholt.
- 23.09.2026: GATE 2 LABEL-FILTER läuft (ROADMAP-Empfehlung 2): v3-Daten
  (texel3: 78512 train + 8493 hold) filtern auf tiefenstabile Such-Eval
  (persistente UCI-Session, `go nodes` 10k vs. 50k, behalten wenn beide cp
  und |Δ|≤50; mates raus; ucinewgame alle 200 Positionen gegen TT-Carryover).
  Werkzeug `/tmp/opencode/label_filter.py`. Pilot (500 Zeilen): 89 % kept,
  5 % mate, 6 % instabil.   Volläufe als Hintergrundjobs
  (PIDs in Logs, `tail /tmp/opencode/filter_{train,hold}.log`):
  → danach `funken texel-tune` auf gefilterten Daten (25 Sweeps) + Holdout.
- 23.09.2026: GATE 2 ERGEBNIS (Filter fertig: train 68193/78512 = 86,9 %,
  hold 7407/8493 = 87,2 %; Rest mates ~5 % + instabil ~8 %):
  `funken texel-tune` auf gefilterten Daten (25 Sweeps, Early-Stopp Geduld 3):
  STANDARD train 0,08658/hold 0,08224 → best (Sweep 1) train 0,08509 /
  hold **0,08205 (−0,2 %)**. Stopp nach Sweep 4 (Holdout-Plateau).
  Zum Vergleich v3 ungefiltert: −3,6 % Holdout (das selbst pari spielte).
  Nebenbefund: STANDARD-MSE auf gefilterten Daten *schlechter* als auf
  ungefilterten (0,08658 vs. 0,08119) — der Filter entfernte gerade gut
  passende Labels; stabile Positionen, kein stärkeres Signal.
  GATE 2 NEGATIV (kein Holdout-Plus → keine Ablation per ROADMAP-Regel).
  Tuning-Kapitel damit zu (v1 Overfit, v3 pari, v3-gefiltert flach).
  Rohdaten: `/tmp/opencode/texel3_filt50_{train,hold}.tsv`,
  `/tmp/opencode/params_filt50.txt`, Log `/tmp/opencode/tune_filt50.log`.
- 23.09.2026: GEN4 BEAUFTRAGT (Tobias): 200.000 Tuning-Positionen, -n 200000
  (4× v3-Knoten, stärkere Labels), 2 parallele Jobs auf 2 Kernen.
  Piloten: 6 Partien -n 100000 in 44 s, 6 Partien -n 200000 in 82 s
  (13,7 s/Partie, 106 Halbzüge/Partie wie v3). Hochrechnung: 3800 Partien
  (2×1900, Seeds 20260923/24) ≈ 7,2 h, Ertrag ~220k Positionen (v3-Rate
  58/Spiel, 55 % Extrakt).   Dirs `~/engine-arena/texel-gen4-200k/{jobA,jobB}/`,
  Logs `/tmp/opencode/texel_gen4_{A,B}.log` (PIDs 260129/260127, Start
  21:18 CEST, beide verifiziert produzierend). KEIN QS/SEE, KEIN Rollout.
- 23.09.2026, 21:40: GEN4-SETUP-PRÜFUNG (Auftrag Tobias, Befund-1-Verdacht):
  BESTANDEN, Lauf bleibt. Belege: beide Jobs mit korrekten Args
  (1900 Partien, -n 200000, eigene Dirs/Seeds); `funken-base` md5-identisch
  mit frischem Release-Build (Code seit Build unverändert — nur Doku/Tools
  committet); Gen4 nutzt direktes Binary ohne Wrapper/Env (Befund-1-Klasse
  ausgeschlossen); PGN-Stichprobe: SearchLimit 200000, Zufallsopenings,
  plausible Ergebnisse (54/72/37 nach 163 Partien). Fehlalarm „Hänger":
  mtimes 21:39 waren frisch (nicht 3 h alt) — 174 Partien in 22 Min
  (≈15 s/Partie wie pilotiert), ETA weiter ~04:30. Lehre: vor Hänger-Urteil
  `date` prüfen.
- 24.09.2026, 07:34: GEN4 FERTIG: beide Jobs 1900/1900 (Prozesse beendet).
  Extrakt aus `/merged/` (a_/b_-Präfix, ein Lauf): 3800 Partien →
  **231660 Positionen (train 208160, hold 23500)**, Ziel ≥200k übertroffen.
  Plausibel: 109,3 Halbzüge/Partie, Extrakt 56 %, Weiß-Score 56 %,
  Dup 13k. Details `MESSERGEBNISSE.md` 9.9. Retune NICHT beauftragt.
- 24.09.2026: „LEG LOS" (Tobias): Retune Gen4 → −4,0 % Holdout (echt);
  M1 Tuned-Repeat **+56 Elo (CI +18…+95, LOS 99,8 %, signifikant)**;
  M2 Nullzug-Marge −21 n.s. (offen); M3 LMR-late −3 pari (offen).
  Precheck-Bug (Glob nur 49/100 Paare) gefunden + gefixt, M1/M2 nachträglich
  verifiziert (100 Paare, 0 identisch). Details `MESSERGEBNISSE.md` 9.10.
- 24.09.2026: ÜBERNAHME (Tobias): params_v3 als STANDARD (39/39-Check,
  Bench-Beweis, `438e01e`), M4-Bestätigung **+63 (CI +28…+100,
  LOS 100 %)** — Details 9.11. Danach Gen4-Match (M5). KEIN QS/SEE, KEIN Rollout.
  Nach Abschluss: PGNs in ein Dir (Präfix a_/b_), EIN texel_data.py-Lauf
  (globaler Dedup + Holdout), zählen, MESSERGEBNISSE. Retune danach ist
  NICHT beauftragt (Folgeentscheidung).
- 23.09.2026: BEFUND-KORREKTUR (Tobias, verifiziert): Tuning-Ablation vom
  Morgen (angeblich 20/40 pari) ist UNGÜLTIG — 20/20 Paare identisch
  (ttage/nonull je 0/20). Ursache verifiziert: Match 08:06–08:19 mit
  funken-base-Build 22.09. 16:50, tune.rs-Commit („FUNKEN_PARAMS") erst
  22.09. 20:22 → Variable still ignoriert, Basis vs. Basis. Wirkbeleg heute:
  gleiches Binary mit/ohne Params → Bench 52148 vs. 264705 Knoten.
  Folgen: „MSE→Stärke-Lücke", „Tuning-Kapitel zu", Gate-2-Begründung und
  ROADMAP-Stand („Kapitel zu") sind OFFEN, nicht gemessen. Gate-Schlüsse
  (Pruning-Sweep, LMR-Mikros) gelten nur als Mikro-Diagnose („dreht die 8
  Stellungen nicht"), nicht als Stärke-Befund. Neue Regel: Precheck
  (`tools/precheck.sh`, 4/4 Tests grün: PRE-Abbruch bei identischen
  Kandidaten, PRE-OK Basis-vs-Tuned, POST-UNGÜLTIG tuned, POST-OK ttage),
  Mindestgröße 200, nicht-signifikant = offen. Gen4 läuft unberührt weiter;
  ausstehende Matches (Tuned-Repeat, Nullzug-Marge, LMR-late) starten nach
  Gen4 (Kerne belegt). HINWEIS: POST-Checks schrieben je eine Zeile in
  `PRECHECK.log` der fremden Matchdirs (tuned/ttage) — nur Logs, kein
  PGN angerührt.
