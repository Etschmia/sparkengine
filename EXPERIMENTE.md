# Experimente: Voigtsbach-Blunder (Funken)

Sitzungsprotokoll — Zweck: Bei Abbruch der Sitzung ist hier der exakte Stand
(Plan, laufende Versuche, Zwischenergebnisse, offene Fragen) nachlesbar.
Messungen landen zusätzlich knapp in `MESSERGEBNISSE.md`, Unvermessene
bleiben hier.

**Regeln:** Keine Codeänderung ohne Messung (Buch + feste Knotenzahl,
9.1-Verfahren). Varianten nur als separate Binaries in `/tmp`
(`cargo build --release` nach Patch, danach Revert, nie committen).
Ausgangspunkt: `MESSERGEBNISSE.md` 9.5/9.6 (55 Blunder, 38 Partien).

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
  - Nxe4: 1M Bc7 (−53), ab 5M Nxe4 (−22). Präferenz-Flip mit Tiefe um ~30 cp;
    SF: Nxe4 +355, Bc7 noch besser. Feines Eval-Gefälle, DEPRIORISIERT
    (Eval-Tuning-Territorium, kein Hand-Patch).
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
  (Gradienten-Flip, Eval-Tuning-Gebiet), Nb5 (stabiler Optimismus ~190 cp),
  Ke2 (Nullzug versteckt Qd8+ bis 5M), sac-Überschätzung frisch (Qxf7 +540
  bei 200k → QS/SEE-Kandidat). Nächste: Statik-Audit Nb5/Nxe4 (evaluate()
  vs. Suche?), QS-Diagnose Opferlinien.
