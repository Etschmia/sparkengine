# Auftrag: Fehler bei Stellungswiederholung finden und beheben

Hallo Funken-Entwickler,

bei einem Review von außen ist ein Fehlverhalten aufgefallen. Es widerspricht
auch der eigenen Doku. Bitte finde die Ursache selbst, behebe sie und bring
die Doku auf den Stand der Wahrheit. Die Regeln aus `AGENTS.md` gelten
unverändert, besonders Regel 3: „Ehrlichkeit vor Schönfärberei“.

## Die Diskrepanz

`KONZEPT.md`, Abschnitt 3, behauptet:

> Remis: … Stellungswiederholung über Hash-Historie (Vereinfachung, offengelegt:
> im Suchbaum zählt bereits die 2. Wiederholung als Remis — stabil, aber
> theoretisch unexakt; **an der Wurzel gilt echtes Dreifach-Remis**)

Das beobachtete Verhalten passt nicht dazu.

## Reproduktion

```bash
cargo build --release
F=./target/release/funken
M="e2e4 e7e5 g1f3 b8c6 f1c4 f8c5"

# 1) Italienisch, ohne Vorgeschichte
printf "uci\nposition startpos moves $M\ngo movetime 1000\n" | (cat; sleep 2) | $F | grep -E '^info depth|bestmove' | tail -2

# 2) Dieselbe Stellung, aber sie steht jetzt zum ZWEITEN Mal auf dem Brett
#    (beide Läufer einmal hin und zurück)
printf "uci\nposition startpos moves $M c4b5 c5b4 b5c4 b4c5\ngo movetime 1000\n" | (cat; sleep 2) | $F | grep -E '^info depth|bestmove' | tail -2
```

Beobachtet (Stand Commit `38f21c9`):

- Fall 1: normale Suche, `bestmove e1g1`.
- Fall 2: Die Suche ist nach 0 ms fertig. Ausgabe: `info depth 64 seldepth 0 score cp 0 nodes 64 … pv b1c3`,
  dann `bestmove b1c3`. Tiefe 64 nach 64 Knoten ist nicht plausibel. Der Zug
  ist schlicht der erste, den der Zuggenerator liefert, und er ist
  offensichtlich nicht ausgewählt.

Dasselbe passiert schon mit der Grundstellung nach `g1f3 g8f6 f3g1 f6g8`.

Die Stellung ist zu diesem Zeitpunkt erst **zum zweiten** Mal auf dem Brett.
Nach den Schachregeln ist das noch kein Remis. Selbst bei einem Remis dürfte
die Engine aber nicht blind den erstbesten Zug spielen.

## Warum das wichtig ist

Im Lichess-Betrieb kommt das ständig vor: Züge werden hin- und hergespielt,
Figuren manövrieren im Endspiel. In genau diesen Momenten spielt Funken dann
einen zufälligen Zug, auch aus einer gewonnenen Stellung heraus.

## Hinweise für die Suche

- Verfolge, welche Hashes beim Start einer Suche im Wiederholungs-Stack
  liegen: die Partiehistorie aus `uci.rs` und das, was der
  Iterative-Deepening-Treiber zusätzlich hinzufügt. Zähle dann nach, was die
  Wiederholungsprüfung an der **Wurzel** (ply 0) daraus macht.
- Achte darauf, **wo** im Knoten die Remisprüfung steht, verglichen mit der
  PV-Initialisierung und der Zugschleife. Und frage dich, was der Treiber als
  `best` übernimmt, wenn der Wurzelknoten sofort zurückkehrt.
- Überlege, ob das Problem nur die Wurzel betrifft. Wie verhält sich dieselbe
  Prüfung in tieferen Knoten, deren Stellung schon **einmal in der Partie**
  (nicht im Suchbaum) vorkam? Ist das die offengelegte Vereinfachung oder
  etwas anderes?
- Prüfe auch die Wechselwirkung mit der Transpositionstabelle: Werden
  Remiswerte aus Wiederholungen dort gespeichert und später auf anderen
  Pfaden wiederverwendet?

Die Lösung ist bewusst nicht vorgegeben. Analysiere die Ursache und
entscheide selbst, was an der Wurzel und was im Baum richtig ist.

## Erwartetes Ergebnis

1. **Ursache** knapp benannt, mit Datei und Stelle.
2. **Fix** im Code. Die Wurzel muss immer eine echte Suche über die legalen
   Züge machen und einen begründeten Zug liefern.
3. **Regressionstests** in `cargo test`, mindestens:
   - Die oben reproduzierte Stellung mit Vorgeschichte liefert denselben
     sinnvollen Zug wie ohne Vorgeschichte, und die Suche erreicht eine
     Tiefe > 1.
   - Eine echte dreifache Wiederholung, die sich per Zug herbeiführen lässt,
     wird korrekt als Remis bewertet. Beispiel: Eine Seite steht klar
     schlechter und kann durch Dauerschach oder Rückkehr Remis erzwingen.
4. **Messung vor Übernahme**, wie in `AGENTS.md` verlangt: `bench` und ein
   kurzes Match alter gegen neuer Stand. Zum Beispiel mit
   `~/engine_match.py`, Doku in `~/engine_match.md`, das auch Pfade zu
   beliebigen Binaries annimmt. Dazu vorher den alten Stand als Kopie des
   Binaries sichern.
5. **Doku korrigieren**:
   - `KONZEPT.md`, Abschnitt 3: beschreiben, was jetzt tatsächlich gilt.
   - `MESSERGEBNISSE.md`: den Fehler im Protokoll der Eigenkorrektur
     ergänzen (Fund durch externes Review, Ursache, Fix, Test). Den
     Testzähler (`11 Unit-Tests`) in README, AGENTS.md und
     MESSERGEBNISSE.md anpassen.
   - Prüfe dabei, ob weitere Aussagen in der Doku ungetestet sind, und
     ordne sie ehrlich unter „Ausstehend“ ein.
6. Commit mit aussagekräftiger Nachricht. Kein Push ohne Rückfrage.

---

# Anhang: Bearbeitung (18.09.2026, Stand 38f21c9 → Fix) — Originalauftrag oben unverändert erhalten

## 1. Ursache (knapp, mit Datei und Stelle)

`src/search.rs`, `is_repetition()` (ca. Zeile 198) + `negamax_impl()` (ca. Zeile 423)
+ `id_loop()` (ca. Zeile 722):

- Der Wiederholungs-Stack enthält die aktuelle Stellung (Treiber pusht den
  Root-Hash, im Baum wird vor dem Rekursieren gepusht). `is_repetition()` meldete
  aber schon bei `count >= 2` Remis — also bereits beim **2. Auftreten**
  (einmal in der Partiehistorie + aktuell), nicht erst beim dritten.
- Die Prüfung stand **vor** PV-Initialisierung und Zugschleife und galt auch für
  Ply 0: An der Wurzel kehrte die Suche sofort mit Score 0 und leerer PV zurück.
  Der ID-Treiber übernahm dann `root_moves[0]` (erstgenerierter Zug) und lief alle
  Tiefen mit je 1 Knoten durch → `info depth 64 seldepth 0 score cp 0 nodes 64 …
  pv b1c3`, `bestmove b1c3`. Reproduziert mit Alt-Binary vor dem Fix.
- Zwei verwandte Mängel gleich mit behoben: (a) Die alte Zählung wertete auch im
  Baum das 2. Auftreten als Remis — das war **nicht** die in KONZEPT.md offengelegte
  Vereinfachung, sondern derselbe Zählfehler (betraf Stellungen mit genau einem
  früheren Auftreten in der Partie). (b) Ein TT-Cutoff an der Wurzel hätte ohne PV
  denselben Effekt gehabt (TT bleibt in `uci.rs` über Suchen erhalten) — die TT
  dient an der Wurzel jetzt nur der Zugordnung.

## 2. Fix (`src/search.rs`, nur eigene Implementierung, kein fremder Code)

- `is_repetition()`: Cutoff erst bei `count >= 3` → **exaktes Dreifach-Remis**
  (Stack enthält die aktuelle Stellung), einheitlich für Partiehistorie und Baum.
  Die offengelegte „2. Wiederholung zählt im Baum"-Vereinfachung ist damit
  gestrichen; `KONZEPT.md`, Abschnitt 3, beschreibt jetzt den exakten Stand.
- `negamax_impl()`: keine Remis-Abkürzung bei Ply 0 — die Wurzel sucht immer über
  alle legalen Züge; kein TT-Cutoff bei Ply 0 (nur `tt_move` für die Ordnung).
- Remis-Cutoffs (50-Züge/Material/Wiederholung) setzen `pv_len[ply] = ply`, damit
  keine stale PV-Tails kopiert werden.

## 3. Regressionstests (`cargo test`: 11 → 14, alle grün)

- `second_occurrence_still_searches`: Italienisch mit Läuferpendel-Vorgeschichte
  (2. Auftreten) liefert gleichen Zug/gleichen Score wie ohne Vorgeschichte,
  Tiefe 3, >100 Knoten.
- `forced_triple_repetition_scores_zero`: `3q1k2/5ppp/8/8/8/8/8/4R1K1 w` (Weiß klar
  schlechter, Eval −764): mit zweimaligem früheren Auftreten von Kh1-Stellung wird
  `g1h1` mit Score 0 gewählt; mit einmaligem bleibt der Score klar negativ (< −200).
- `reused_tt_still_searches_root`: Suche Tiefe 4 nach Tiefe 4 mit derselben TT
  sucht voll (Knoten >> 100, PV konsistent statt erstgeneriertem Zug).

## 4. Tatsächlich ausgeführte Messungen (alle echt, nichts erfunden)

- `cargo test`: **14/14 ✅**.
- `cargo build --release`: **0 warnings/errors**.
- `./target/release/funken perft 5`: **4865609** ✅ (Referenz).
- `./target/release/funken bench` (Tiefe 8): Startpos 52148 Knoten (alt: 52116),
  Kiwipete 312538 (alt: 312597), pos3 38731 (alt: 38725); Scores/PVs identisch —
  kein Leistungsverlust, Mini-Diff aus exakterer Zählung.
- UCI-Repro des Prompts (Fix): Fall 1 und Fall 2 beide `bestmove e1g1`,
  Tiefe 11, ~1,1 Mio. Knoten, identische PV (vorher Fall 2: depth 64/nodes 64,
  `bestmove b1c3`); Springerpendel-Grundstellung: Tiefe 12, `bestmove b1c3`.
- `tests_ucitool.py`: OK (u. a. movetime 500 → 406 ms, 60+1 → 2903 ms).
- `tests_selfplay.py` (Tiefe 5, 2 Partien): regulär, alle Züge legal.
- Match neu vs. alt (`~/engine_match.py`, python-chess, je 30 s/Seite,
  nacheinander): **2,0 : 0,0** für neu (je 1× Weiß/Schwarz, beide Matt, normal
  beendet). PGNs: `/tmp/opencode/match_r1.pgn`, `/tmp/opencode/match_r2.pgn`
  (temporär, nicht im Repo). Binaries: `/tmp/opencode/funken-alt-38f21c9`
  (Stand 38f21c9), `/tmp/opencode/funken-neu-fix`. Einordnung: Kleinstsample,
  belegt nur Schadensfreiheit des Fix — **keine Elo-/Stärke-Behauptung**.

## 5. Doku korrigiert

- `KONZEPT.md`, Abschnitt 3: exaktes Dreifach-Remis, Wurzel sucht immer, TT an der
  Wurzel nur Ordnung.
- `MESSERGEBNISSE.md`: Eigenkorrektur Nr. 5, Testzähler 14/14, Alt-gegen-Neu-Match,
  Bench-Vergleich; Abschnitt 6 („Ausstehend") um TT-Pfadabhängigkeit von
  Remis-Scores ergänzt (ehrlich als Restungenauigkeit, nicht als exakt behauptet).
- Testzähler `11` → `14` in `README.md`, `AGENTS.md`, `MESSERGEBNISSE.md`.

## 6. Verbleibende Grenzen (ehrlich)

- Remis-beeinflusste Teilbaum-Scores landen in der TT und können auf Pfaden mit
  anderer Historie wiederverwendet werden (pfadabhängige Restungenauigkeit,
  dokumentiert in `MESSERGEBNISSE.md`, Abschnitt 6).
- Match nur 2 Partien Kurzschach — keine Spielstärke-Aussage.
- Keine laufenden Engines/Bot-Dienste/Deployments angefasst; kein Push (nur lokal
  committet); keine fremde Engine-Implementierung übernommen.
