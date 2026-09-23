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

Alt-gegen-Neu nach Wiederholungs-Fix (18.09.2026, `~/engine-arena/engine_match.py`,
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
  Ob daraus je ein Partiepatzer wurde, ist ungeklärt (siehe Abschnitt 9.3).
- Ablationen aller übrigen ererbten Bausteine (LMR, Futility/Reverse-Futility,
  Aspiration, Killer/History, Delta-Pruning, Schachverlängerung,
  TT-Ersetzungsregel, Zeitformel) — nur Nullzug ist vermessen (Abschnitt 9.2,
  dort als offen eingestuft: n = 40 trägt keinen Stärke-Befund).
- Wiederholung der Tuning-Ablation (Basis vs. params_v3, UNGÜLTIG s. 9.8):
  ≥ 200 Partien, > 20 Eröffnungen (Zufallsopenings), Precheck — ausstehend,
  eingeplant nach Gen4-Langlauf (Kerne belegt bis ~04:30).
- Varianten-Matches Nullzug-Marge (+150) und LMR-late vs. Basis (je ≥ 200,
  Precheck) — ausstehend, gleiche Einplanung. Die 8 Blunder-Stellungen bleiben
  Diagnose, nicht Übernahme-Kriterium.
- Mess-Regel seit 23.09. (Befund-Korrektur): Precheck (`tools/precheck.sh`)
  vor/nach jedem Match Pflicht; Mindestgröße 200 Partien; keine Variante wird
  allein wegen „Gate negativ" oder „n = 40 pari" verworfen — nicht signifikant
  heißt „offen", nicht „tot"; Übernahme nur mit gemessenem Plus.
- Patzer-Repro: Stellung + Uhrstand + TT-Zustand des nächsten Einzug-Patzers
  sichern (Abschnitt 9.3, nächste Schritte).

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

## 9. Kanon und Messung (20.09.2026, Auftrag `PROMPT_kanon_und_messung.md`)

Stand: Code unverändert `a31601d` (Binary `cmp`-identisch mit
`../engine-arena/sparkengine-a31601d`, kein neues Freeze-Binary nötig).
Antwort auf beide Denkanstöße, erste Ablation mit neuem Verfahren
(Buch + feste Knotenzahl), Patzer-Befund. Alle Zahlen auf der Test-VM.

### 9.1 Was unsere Zahlen hergeben (Denkanstoß 1)

- Keine Messung in Abschnitt 3–5 kann einen Nulleffekt von einem Gewinn
  unterscheiden: 4 Partien vs. SF-Stufen und 2 Partien alt-gegen-neu sind
  Kleinstsamples ohne Fehlerbalken. Das stand schon vorher so da („keine
  Elo-Behauptung") und bleibt so.
- Neue Regel ab jetzt: Eval- und Suchänderungen werden nur noch mit
  Eröffnungsbuch (`openings.epd`) und fester Knotenzahl (`-n`) gemessen —
  also Effekt der Änderung statt Zufall der Zeiteinteilung. Uhr-Partien nur
  noch separat für die Zeiteinteilung selbst.
- Rechnung für „2 Kerne und eine Nacht": Bei `-n 200000` dauert eine Partie
  ~25 s (eine Partie gleichzeitig, vgl. 9.2) → ~1100 Partien in 8 Stunden.
  Für ±20 Elo bei p ≈ 0,5 braucht man je nach Remisanteil ~400–800 Partien
  (1,96·sd/√n ≤ 0,035 mit sd ≈ 0,35–0,5). Eine Nacht reicht also für
  10–20-Elo-Auflösung — mit Uhr-Partien (5+0, ~10 min/Partie) wären es
  ~50 Partien/Nacht (±~100 Elo) und damit aussichtslos. Genau dafür ist das
  neue Verfahren da.

### 9.2 Erste Ablation: Nullzug-Pruning (gemessen)

Frage aus Denkanstoß 2: Verdient der Nullzug (R = 2, ab Tiefe > 6 R = 3,
nur mit Nicht-Bauern-Material, `src/search.rs`) seinen Platz?
Aufbau: Voll (`/tmp/funken-full` = `a31601d`) gegen Ohne-Nullzug
(`/tmp/funken-nonull`: dieselbe Bedingung mit `false && …`, nach der Messung
revertiert, nicht committet). 40 Partien, `BOOK=openings.epd`
(20 Stellungen × beide Farbverteilungen), `-n 200000`, eine Partie
gleichzeitig. PGNs: `../engine-arena/ablation-nonull-200k/` (nicht im Repo).

  40 Dateien, 40 Partien: funken-full gegen funken-nonull
    Ergebnis   22 : 18   (+12 =20 −8 aus Sicht von funken-full)
    Score      55,0 % — Elo +35, 95-%-Bereich −42 … +115
    LOS 81 % — Gleichstand liegt im Bereich: belegt keinen Unterschied.
    Tiefe (Median der Partiemediane): full 10, nonull 9.
    Enden: 20× normal, 20× dreifache Wiederholung.

Einordnung: Selbst eine bekannte starke Heuristik ist mit 40 Partien nicht
nachweisbar — der Balken ist ±~80 Elo breit. Das ist kein Versagen des
Nullzugs, sondern die erwartete Auflösung (vgl. 9.1): Für ±20 bräuchte es
~400–800 Partien. Auffällig: 50 % Remis (Selbstspiel aus ausgeglichenen
Buchstellungen drückt Differenzen) und nur 1 Halbzug Tiefenabstand bei
gleicher Knotenzahl. Status seit Befund-Korrektur 23.09.: **offen, nicht
stärke-belegt** (n = 40 ist kein Beleg in irgendeine Richtung; Regel seitdem:
kein Verwerfen wegen „n = 40 pari", Wiederholung ≥ 200 Partien mit Precheck).
Der Baustein bleibt drin — nicht weil die Messung ihn bestätigt (das tut sie
nicht), sondern weil sie ihn nicht widerlegt und die Buchhaltung (mehr Tiefe
pro Knoten) für ihn spricht. Zweitmessung mit ~800 Partien steht aus
(Abschnitt 6).

### 9.3 Die unerklärten Patzer (Punkt 1: verfolgt, Ursache offen)

Serie-1-Befund bestätigt (PGNs in `../engine-arena/`): Runde 1
(Halbzug 34, Schwarz): `34…a4??` mit Ansage +3,1/12 bei Tiefe 12 — danach
−8 (Turmverlust in einem Zug, ~11 Bauern). Runde 4 (Halbzug 37, Weiß):
`37.Tb1??` +0,81/13 — danach −4,2 (Qualitäts-/Turmverlust, ~5 Bauern),
extern nicht reproduzierbar → Timing- oder TT-zustandsabhängig.
Serie 3 (10 Nicht-Siege = 6 Niederlagen + 4 Remis, PGNs in
`../engine-arena/match-2026-09-19-b/`): 5× langsames Untergehen über 20–60
Züge ohne Sprünge > 1,5 Bauern (r02, r05, r10, r13, r15), 1× Einzug-Patzer
(r09: `61…Bg5`, −0,57 → −3,83 bei Tiefe 12–13), 4× Remis (r04, r08, r11,
r12 — in r04/r11/r12 Funken-Eval +1 bis +2 gegen Gegner-~0: mögliche
Konvertierungsschwäche oder reine Eval-Differenz, offen).

Zur Verdachtsfrage TT + Wiederholung: Der Mechanismus existiert
(`negamax_impl` speichert Eltern-Scores, in deren Teilbaum ein
Remis-Cutoff steckt, historienblind in der TT; Quieszenz ebenso) — eine
falsche 0 statt ±300+ kann daraus folgen, ein Figurenverlust als Zug also
grundsätzlich ja. Aber: kein Beweis, dass das die Serie-1-Patzer waren
(kein Repro, keine Knoten-Logs). Zur Fensterabbruch-Frage: Der ID-Treiber
verwirft abgebrochene Iterationen (Aspiration wie Zeit) und gibt den Zug
der letzten vollendeten Iteration zurück — kein Rückfall auf den
erstgenerierten Zug (Ausnahme nur: Abbruch vor Abschluss von Tiefe 1, dann
`depth_completed = 0`). Der Wurzel-Rückfall ist seit dem Fix (`a31601d`)
geschlossen.

Folge: Keine Codeänderung, kein erfundener Regressionstest — ohne Repro
wäre beides Schönfärberei. Nächste Schritte, fest vereinbart: Beim nächsten
Einzug-Patzer Stellung + Uhrstand + `go nodes`-Repro sichern; als
Experiment mit Messung (9.1-Verfahren): TT-Einträge aus
repetitionsbeeinflussten Knoten nicht speichern bzw. Historie in den
TT-Schlüssel aufnehmen, Kosten/Nutzen per Ablation.

### 9.4 Was an Funken gemessen ist, was geerbt (Denkanstoß 2)

Gemessen (eigene Tests/Messungen): exakte Dreifach-Schwelle (≥ 3),
Wurzel-sucht-immer bei Wiederholung, TT-an-Wurzel nur Ordnung (je
Regressionstest, Abschnitt 2); Nullzug-Ablation ohne Nachweis (9.2).
Alles andere ist geerbt und unvermessen — übernommene Zahlen ohne eigene
Messung dahinter: Aspiration ±25 ab Tiefe 4 (Nachsuche ±120, Halbierung),
Nullzug-R = 2–3 (Schwelle Tiefe > 6), LMR (red 1 ab Tiefe ≥ 3 und legal > 3,
red 2 ab Tiefe ≥ 6 und legal > 8, nur ruhige Züge), Futility-Margen 150/250
(Tiefe ≤ 2), Reverse-Futility 90×Tiefe (Tiefe ≤ 4), Delta-Schwelle
Gewinn + 200, 2 Killer/Ply (850k/840k), History Tiefe² (Cap 1 Mio.),
Schachverlängerung 1, Mattdistanz-Pruning, TT-Ersetzung Tiefe+2-Toleranz,
Zeitformel Rest/25 + Ink/2 (hart 4×), Tempo +8 sowie sämtliche
Eval-Gewichte (Material, MOB_W, Struktur-, Läuferpaar-, Linien-,
Schild-Terme, alle PST-Regelkoeffizienten).

Gestalt-Hinweis: Die Überlieferung wuchs an Bitboard-Engines mit Millionen
Knoten/s; Funken ist Mailbox, ein Kern, ~1 Mnps, nur std. Pruning, das dort
getunt wurde, beschneidet hier relativ mehr — die Nullzug-Ablation (+35,
n. s., statt überlieferter ~100+) passt zu diesem Vorbehalt, beweist ihn
aber nicht (Remislastigkeit als Alternative, 9.2).

Eigene Entscheidung gegen den Kanon, weiter verteidigt: exaktes
Dreifach-Remis auch im Baum (statt 2. Wiederholung = Remis) und
Wurzel-sucht-immer — Begründung: Korrektheit vor ein paar Knoten, Kosten
vernachlässigbar. Weitere Gegenpositionen gibt es derzeit keine.

### 9.5 Voigtsbach-Blunderstichprobe (22.09.2026, gemessen)

Anlass: 38 Lichess-Partien (Account Voigtsbach, alle vs. Bots, 19,5/35 = 56 %),
55 Blunder per Stockfish 17.1 Tiefe 17 (`~/voigtsbach_analysen`). Verfahren:
12 FENs (`fen_before`, je Top-Loss pro Motiv + Matt-Fälle + 3 ohne Motiv-Tag),
Funken aktueller Stand, frische TT, `go depth 12`, Vergleich mit gespieltem
Zug und SF-Bestzug (Treiber `/tmp/opencode/probe_fen.py`, Rohdaten
`/tmp/opencode/fen_probe.json` — beide temporär, nicht im Repo).

- **5/12 reproduziert (systematisch, kein Zeitproblem):** Funken spielt den
  Blunder auch bei Tiefe 12: `b7` statt `Ke5` (Matt nicht gesehen, +1232 statt
  Matt; Endspiel), `Ng3` statt `Qxe4` (−358 statt SF −613 vor dem Zug),
  `Nxe4` statt `Bc7` (+13 statt −114; Eröffnung, Springer hängt),
  `Nb5` statt `d3` (+178, SF nach dem Zug 0), `Ke2` statt `Qd8+` (+30, SF
  danach −213). In allen fünf Fällen liegt Funkens Root-Score 40–260 cp
  neben SF *vor* dem Zug — Such- oder Eval-Blindheit, nicht nur Tiefe.
- **2/12 bei Tiefe 12 behoben (Tiefenproblem im Blitz plausibel):** `Rxa1`
  statt `Qxf7`, `Nd3` statt `Rg4` — im Spiel (180+2 bzw. 60+2) vermutlich zu
  flach gesucht. Teilerfolg: `cxd3` statt SF-`Qxd3` (hängender Läufer wird
  gesehen, aber mit der falschen Figur genommen).
- **5/12 dritte Züge:** `g6g8` statt Matt-erlaubendem `Qc8` (−735, hält die
  verlorene Stellung statt Matt), `h7h6` statt `Rh3` (+55 statt −15),
  `e7e6` statt `Bf4` (+84 statt −26) — Schadensbegrenzung, nicht Bestzug;
  `Ng5` statt `Rg5`, `Qxd3`-Variante s. oben.
- Hypothesen (unvermessen, als Experimente mit 9.1-Verfahren zu prüfen):
  Hängende-Figuren-Blindheit (Eval oder ruhige-Tiefe/SEE), Matt-Erkennung
  (Verlängerung), Eröffnungs-Flachsuche bei Blitz-Bedenkzeit.
- Ausstehend: PGNs für Uhrstand, TT-Zustand und Partiekontext (Abschnitt 6);
  dann `go nodes`-Repro je Fall wie in 9.3 vereinbart.

### 9.6 Voigtsbach-Blunder × Funkens Spiel-Sicht (22.09.2026, gemessen)

Die PGNs (`~/voigtsbach_analysen/pgn/`, 38 Partien) enthalten Funkens eigene
PV mit `[%eval x,d]` und `[%clk]` je eigenem Zug. Join mit der Blunder-JSON:
2066 eigene Züge, 2016 mit Eval, alle 55 Blunder zugeordnet (Treiber
`/tmp/opencode/join_blunders.py`, Daten `/tmp/opencode/blunder_funken_view.json`,
beide temporär). Bridge-Eval ist Weiß-Sicht in Bauern; die Blunder-JSON ist
Eigen-Sicht (immer Voigtsbach — verifiziert per Kreuzcheck mit lokalem SF
17.1 an Nxe4: JSON −114 → −355 = SF-Weiß-Sicht +123 → +358). Alle Vergleiche
unten in Eigen-Sicht (PGN-Werte ggf. konvertiert). Gegner-Elo ~1900–2210,
Funken ~1980–2210.

- **Zeitnot als Hauptursache widerlegt:** Tiefe am Blunder Median 13
  (Spanne 10–17) vs. 14 über alle Züge (p10 12); Restzeit meist 30–400 s bei
  2–19 s Bedenkzeit. Nur die beiden Martuni-Blunder (60+0, ohne Inkrement:
  `Rb1`/`Kxg7` bei 14/7 s Rest) sind echte Zeitnot. Blunder passieren bei
  normaler Tiefe und Bedenkzeit.
- **Wahnmaß** |Funken-Erwartung − SF-nachher| (Eigen-Sicht): Median 272 cp,
  p90 468 cp. Bei der Hälfte der Blunder lag Funken ≥ 2,7 Bauern neben der
  Realität. Größte Delusionen: `Rg5` 779, `Kc4` 769, `Ng3` 687, `Rd6` 606,
  `b7` 517, `Qxf7`/`Bh7` je 468, `Rc3` 440, `Rd3` 431, `Rhc1` 420.
  (Korrektur 22.09.: JSON war fälschlich als Weiß-Sicht gelesen — Bxh3/h4-
  „Flips" waren Artefakte; Details in `EXPERIMENTE.md`, Konventionen.)
- **Kleine Lücke (41–77 cp):** `d5`, `Rg4`, `b3`, `Bxh4`, `Bf4`, `Rh3` —
  Funken bewertete die Stellung korrekt und wählte unter Übeln
  (Horizont/Pruning, nicht Eval-Blindheit).
- **Die 5 Repro-Fälle aus 9.5 mit Spiel-Sicht** (PGN-Werte Weiß-Sicht; in
  Eigen-Sicht: b7 +1414, Ng3 −358, Nxe4 −11, Nb5 +178, Ke2 +27, Qc8 −1041):
  `b7` d16 +14,14 (Matt auch
  bei Tiefe 16 nicht gesehen, 86 s Rest — Matt-Erkennung bestätigt schwach);
  `Ng3` d12 −3,58 (wusste um −3,5 und spielte es trotzdem — Rettung `Qxe4`
  weggeprunt/Horizont); `Nxe4` d15 +0,11 (11 s Bedenkzeit, 234 s Rest —
  reine Eval-Blindheit); `Nb5` d12 +1,78 (Überschätzung); `Ke2` d15 +0,27
  (gegnerisches `Qd8+` nicht gesehen). `Qc8` d14 +10,41: Verlust grob
  erkannt, einzige Rettung `Qxf5` nicht gefunden.
- **Nuance zu 9.5:** `Qxf7` fiel im Spiel bei Tiefe 16 (tiefer als die
  Probe mit Tiefe 12, die `Rxa1` fand) — zustandsabhängig (TT/Historie),
  Tiefe allein erklärt es nicht (vgl. 9.3-Verdacht TT + Wiederholung).
- **Tiefe 64** (95× im Rohtext) nur bei Matt-Ansagen (50) und 0,00 (45) —
  Suche läuft in entschiedenen/toten Stellungen bis Max-Tiefe, harmlos,
  nie an einem Blunder (dort max. 17).
- Nächste Schritte (unvermessen): Statik-Audit entlang SF-PVn (wo divergiert
  `evaluate()`? Nb5-Wurzel −168, Nxe4-Blatt −129, Qxf7-nach −618);
  QS-Diagnose ruhiger Widerlegungen (Qb4-Typ); Nullzug-Kosten für Qd8+-Typ.
  Stand in `EXPERIMENTE.md`.

### 9.7 Session-Effekt und TT-Aging-Ablation (22.09.2026, gemessen)

Frage aus 9.3/9.6: Warum spielt Funken im Spiel Züge (Qxf7, Bh7, Qc8), die
die frische Suche nicht spielt? Methode: Sitzungssimulation (ein Prozess,
500k-Füllsuchen entlang der Partie, 5M-Entscheidung; Skripte
`/tmp/opencode/session_sim.py`, `/tmp/opencode/session12.py`, temporär).

- Session12 (12 Fälle): Session reproduziert 8/12 Spielzüge. Drei
  Session-Flips zu Blundern: Qc8 (frisch g6g8), Qxf7 (frisch Rxa1), Bh7.
  Historie (moves-Liste, frische TT) allein ändert nichts; TT-Clear
  (`ucinewgame`) vor der Entscheidung stellt frisches Ergebnis her —
  Ursache ist TT-Inhalt über Züge (überlappende Teilbäume: alte
  Scores/Bounds aus anderen Fenstern/Historien + Order-Pfadabhängigkeit).
- V6 (kein TT-Store nach Repetitions-Cutoff): ändert nichts — Repetitions-
  Taint ist nicht der Mechanismus.
- Ablation Basis vs. V8 (Generationen-Aging: Scores/Bounds nur laufende
  Generation, Zug immer; Patch reproduzierbar via
  `/tmp/opencode/apply_v8v9.py`): 40 Partien, Buch, `-n 200000`,
  PGNs `../engine-arena/ablation-ttage-200k/`. Ergebnis aus V8-Sicht
  **14,0 : 40 (35,0 %), Elo −108, 95-%-Bereich −230…−7, LOS 1,8 %**.
  Klare Regression: zügeübergreifende TT ist ~100 Elo wert. V8 verworfen.
- V9 (nur Bounds altern, alte EXACT weiter cutfähig): heilt nur Qc8 (ohnehin
  verlorene Partie), Bh7 fällt zurück, Qxf7 bleibt — Kosten-Nutzen negativ,
  keine Ablation, verworfen.
- Einordnung: Bewiesener Flip-Schaden ½ Punkt (JdclEX3n-Remis aus +640) aus
  38 Partien gegen ~+100 Elo TT-Nutzen — TT-Route beendet. Weiter an frisch
  reproduzierbaren Schwächen (E2/E3): Details in `EXPERIMENTE.md`.

### 9.8 Texel-Tuning v1/v3 (22.–23.09.2026, gemessen)

Eval-Params als Struct (39 Knöpfe, T1; Tests 14/14, Bench bit-identisch).
Tuner `funken texel-tune` (Koordinaten-Abstieg, Sigmoid 1/(1+10^(−s/400))),
Gewichte per `FUNKEN_PARAMS` ladbar. Daten: eigene Selbstspiele (keine
fremden Labels), Filter ply ≥ 24, kein Schach, nicht nach Schlag/Umwandlung.

- v1 (100 Spiele, 2642 Positionen): Train 0,100 → 0,072, Holdout 0,081 →
  0,122 — Lehrbuch-Overfit, keine Übernahme.
- v3 (1500 Spiele aus 6-Halbzug-Zufallsopenings, 87k Positionen, Holdout
  balanciert): Train 0,08119 → 0,07656, Holdout 0,07703 → 0,07428 (25 Sweeps,
  gemeinsam, Early-Stopp nicht ausgelöst). 33/39 Params bewegt.
- Ablation Basis vs. Tuned (Wrapper, 40 Partien, Buch, `-n 200000`, PGNs
  `../engine-arena/ablation-tuned-200k/`): **UNGÜLTIG (Befund 23.09.,
  verifiziert): 20/20 Partiepaare Zug für Zug identisch** (gegenüber 0/20
  in ttage/nonull). Ursache: `funken-tuned.sh` setzt `FUNKEN_PARAMS`, startet
  aber `funken-base` — zum Match-Zeitpunkt (23.09. 08:06–08:19) ein Build vom
  22.09. 16:50, also VOR dem tune.rs-Commit (22.09. 20:22); die Variable wurde
  stillschweigend ignoriert → Basis gegen Basis, 50 % garantiert. Beleg heute:
  gleiches Binary mit/ohne `FUNKEN_PARAMS` → Bench 52148 vs. 264705 Knoten
  (Startpos). Folge: „MSE übersetzt sich nicht in Spielstärke" ist NICHT
  gemessen; Wiederholung (≥200 Partien, mit Precheck) steht aus. Festes
  Werkzeug seitdem: `tools/precheck.sh` (Bench-Divergenz + md5 vor, Paar-Check
  nach jedem Match).
- Label-Filter-Retune (23.09.2026, ROADMAP-Gate 2): v3-Daten gefiltert auf
  tiefenstabile Such-Eval (10k vs. 50k Knoten, |Δ| ≤ 50 cp, mates raus;
  Werkzeug `/tmp/opencode/label_filter.py`, persistent, ucinewgame/200):
  train 68193/78512 (86,9 %), hold 7407/8493 (87,2 %). Retune (25 Sweeps,
  Early-Stopp): STANDARD 0,08658/0,08224 → best 0,08509/**0,08205**
  (Holdout **−0,2 %**, Stopp nach Sweep 4). Kein Holdout-Plus → keine
  Ablation (Regel). Nebenbefund: STANDARD liegt auf gefilterten Daten
  schlechter als auf ungefilterten (0,08658 vs. 0,08119) — stabile
  Positionen tragen kein stärkeres Signal. Status seit Befund-Korrektur
  23.09.: Die Gate-2-Begründung („schwache Labels") ist offen — sie stützte
  sich auf die UNGÜLTIGE Tuning-Ablation (s. oben). Das Tuning-Urteil als
  Ganzes ist offen bis zur wiederholten Ablation (≥200, Precheck).
- LMR-Mikros (23.09.2026, ROADMAP-Gate 1): red1only/late/deep als
  /tmp-Binaries (revertiert), 8 Stellungen × 500k/1M/2M
  (`/tmp/opencode/micro_lmr.py`, Rohdaten `micro_lmr.json`): **keine
  Rettung** (Qxe4/Qxf5/Qd8+) bei ≤ 2M in keiner Variante — alle wie Basis.
  Status seit Befund-Korrektur 23.09.: **nur Mikro-Diagnose, kein
  Stärke-Befund** — „dreht die 8 Stellungen nicht" ≠ „kein Nutzen"
  (+20–40 Elo wären so unsichtbar). Echte Messung (LMR-late, Nullzug-Marge
  vs. Basis, ≥200 Partien, Precheck) steht aus.
