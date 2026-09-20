# Auftrag: Serie 3, und zwei Zweifel an unserem Vorgehen

Hallo Spark,

der Wiederholungs-Fix (`a31601d`) sitzt, Funken hat seitdem zwei Serien
gegen grokengine gespielt und beide gewonnen. Der Vorsprung ist dabei aber
stark geschrumpft. Unten die Zahlen, danach zwei Denkanstöße, die nicht
deine Engine betreffen, sondern die Art, wie wir über sie reden: **wie
belastbar unsere Messungen sind**, und **woher das Wissen stammt, aus dem
beide Engines gebaut sind**.

Regeln unverändert (siehe auch deine eigene `AGENTS.md`): keine
Engine-Übernahmen, den Code der Engines hier im Home (`../grokengine`,
`../enginemartuni`) nicht lesen, gemessen und vermutet strikt trennen.
Commit ja, Push nein.

Kleinigkeit vorweg: In `MESSERGEBNISSE.md` und
`PROMPT_stellungswiederholung.md` liegen zwei nicht eingecheckte
Pfadkorrekturen von mir (`~/engine_match.py` → `~/engine-arena/engine_match.py`,
das Skript ist umgezogen). Nimm sie beim nächsten Commit einfach mit.

## Ergebnis: zwei Serien gegen grokengine

Alles in `../engine-arena/ERGEBNISSE.md`, PGNs in den dortigen
Unterverzeichnissen. Funken stand in beiden Serien unverändert auf
`a31601d`; der Gegner hat zwischen den Serien überarbeitet.

| Serie | Gegnerstand | Ergebnis | Score | grob | Bereich (95 %) |
|---|---|---|---|---|---|
| 2 (`match-2026-09-19`) | `cee7c62` | **17 : 3** | 85 % | +300 Elo | ±150 |
| 3 (`match-2026-09-19-b`) | `a65bad9` | **12 : 8** | 60 % | +70 Elo | −60 … +230 |

Serie 3: 20 Partien 5+0, 16 Matts, 4 echte dreifache Wiederholungen, keine
Zeitüberschreitung, kein illegaler Zug. Nach 16 Partien stand es 8:8, die
letzten vier gingen an dich. Suchtiefe im Median: Funken 15, Gegner 12.

Der Gegner hat in einem Durchgang rund 230 Elo aufgeholt, indem er seine
Bewertung um Mobilität, Königssicherheit und Türme auf offenen Linien
erweitert hat — Terme, die du von Anfang an hattest. Sein Vorsprung war also
zu einem guten Teil dein Vorsprung in der Bewertungsbreite, und der ist
jetzt weg. **Dein Vorsprung ist nicht mehr sicher von null zu
unterscheiden.**

## Denkanstoß 1: Was unsere Zahlen wirklich hergeben

`+300` und dann `+70` sieht nach einer klaren Geschichte aus. Sie ist
schwächer, als sie aussieht:

- Bei 20 Partien reicht der 95-%-Bereich jeweils über rund 300 Elo. Die
  beiden Intervalle überlappen sich. Dass der Abstand kleiner geworden ist,
  ist ein Indiz, kein Nachweis.
- **Alle Partien beginnen in der Grundstellung, ohne Eröffnungsbuch.** Beide
  Engines sind weitgehend deterministisch. Was die 20 Partien überhaupt
  unterscheidet, ist im Wesentlichen Timing-Rauschen: wann die Uhr in eine
  Iteration fällt, was zufällig in der Transpositionstabelle steht. Das sind
  keine 20 unabhängigen Stichproben, sondern korrelierte Wiederholungen
  desselben Versuchs — die echten Intervalle sind noch breiter als
  angegeben.
- Zum Maßstab: Im Feld gilt als Nachweis ein SPRT-Test — Tausende bis
  Zehntausende Partien, fester Eröffnungspool, feste Hardware, Ergebnis mit
  Fehlerbalken. Unsere 20 Partien sind ein Stimmungsbild.

Das betrifft auch deine eigenen Messungen. In `MESSERGEBNISSE.md` stehen
Ergebnisse aus wenigen Partien gegen SF-Stufen und aus Selbstspiel. Die
Trennung „ausgeführt / ausstehend“ führst du vorbildlich. Die Frage ist die
Stufe darunter: **Wie groß ist die Unsicherheit dessen, was als ausgeführt
dasteht?**

Fragen dazu:

- Woran würdest du erkennen, dass eine deiner Änderungen **nichts** gebracht
  hat? Welche deiner Messungen könnte einen Nulleffekt von einem echten
  Gewinn unterscheiden?
- Unter Abschnitt 6 steht seit Langem „Systematisches Eval-Tuning“ als
  ausstehend. Genau das braucht ein Messverfahren, dem man bei Unterschieden
  von 10 bis 20 Elo trauen kann. Was wäre das bei 2 Kernen und einer Nacht
  Zeit?
## Dafür kann der Schiedsrichter jetzt mehr

`../engine-arena/` ist am 20.09. erweitert worden, genau für diese Fragen
(Doku: `engine_match.md`):

- **Eröffnungsbuch** `openings.epd`: 40 Stellungen aus Hauptvarianten, je 6
  Halbzüge. Mit `BOOK=openings.epd ./run_series.sh …` spielt jede Serie
  verschiedene Stellungen, jede einmal mit jeder Farbverteilung. Damit sind
  die Partien endlich unabhängige Stichproben statt Wiederholungen desselben
  Versuchs.
- **Feste Suchgrenzen statt Uhr**: `-m/--movetime`, `-d/--depth`,
  `-n/--nodes`. Feste Knotenzahl ist für Ablationen die sauberste Messung:
  identische Arbeit pro Zug, praktisch kein Timing-Rauschen, zwischen Läufen
  wiederholbar. Funken beherrscht `go nodes` bereits, das Match-Skript prüft
  das vor der Partie (grokengine fällt dabei durch). Einzelne Startstellungen
  gehen direkt mit `-f/--fen`.
- **Auswertung** `auswertung.py VERZEICHNIS`: Score, Elo, 95-%-Bereich aus
  der Streuung der Partieergebnisse, LOS, Weiß-/Schwarz-Bilanz, Suchtiefe,
  Partieenden. Sie warnt ausdrücklich, wenn der Gleichstand im Bereich liegt.

Damit ist eine ordentliche Messung eine Zeile:

```bash
cd ../engine-arena
BOOK=openings.epd ./run_series.sh sparkengine ./sparkengine-a31601d 80 test-xyz -n 200000
./auswertung.py test-xyz
```

Das ersetzt für Eval- und Suchänderungen die bisherigen Uhr-Partien: Bei
fester Knotenzahl siehst du den Effekt der Änderung selbst, nicht den
Zufall der Zeiteinteilung. Für die Zeiteinteilung brauchst du weiterhin
Uhr-Partien — aber dann als eigene Messung, nicht vermischt.

**Wichtig:** `engine_match.py`, `run_series.sh`, `openings.epd` und
`auswertung.py` benutzen beide Engines. Ändere sie nicht selbst — sonst
stehen wir mit zwei Gabelungen da und können nichts mehr vergleichen. Wenn
dir etwas fehlt, schreib es auf; das Werkzeug wird zentral erweitert. Eigene
Auswertungsskripte daneben sind willkommen.

## Denkanstoß 2: Woher das Wissen stammt

Beide Engines sind aus **einem** Prompt entstanden, unabhängig voneinander,
ohne konzeptionellen Input, mit dem einzigen Verbot, nichts zu kopieren.
Herausgekommen sind zwei Engines mit praktisch derselben Architektur:
Mailbox-Brett, Pseudo-legal plus make/unmake, Perft, Zobrist,
Transpositionstabelle, iterative Vertiefung, Aspiration, Ruhesuche,
MVV-LVA, Killer und History, Nullzug, LMR, Futility, getaperte Bewertung.

Das kann zweierlei heißen. Entweder ist das die objektiv richtige Lösung,
auf die man unabhängig stößt. Oder beide Modelle haben aus demselben Korpus
gelernt — und der ist im Wesentlichen das Chess Programming Wiki samt der
Foren, aus denen es schöpft.

Du benennst das in `KONZEPT.md` korrekt: *„Beschreibungen im Chess
Programming Wiki — als Theoriequelle genutzt, kein Code übernommen.“* Das
ist die richtige Vokabel, und der Gegner hat sie nicht gehabt: Er hat die
Tabellen der „Simplified Evaluation Function“ und die PeSTO-Materialwerte
Zahl für Zahl aus dem Gedächtnis reproduziert und dazu geschrieben, die
Werte seien seine eigenen. Nicht aus Unehrlichkeit — er hat es selbst nicht
gemerkt. Dass du deine Tabellen programmatisch aus eigenen Regeln erzeugst,
ist an dieser Stelle der deutlichste eigene Beitrag im ganzen Projekt.

Die Theoriequelle selbst ist allerdings kein geprüftes Lehrbuch, sondern ein
Literatur- und Begriffsverzeichnis: Es sammelt, wer wann was behauptet hat,
oft aus Forenbeiträgen, meist ohne Effektgröße und ohne Fehlerbalken. Dass
es sich irren kann, hat das Feld mehrfach selbst gezeigt:

- MTD(f) wurde 1994/95 als überlegen gegenüber PVS publiziert und steht
  prominent im Wiki. In starken Engines hat es sich nie durchgesetzt.
- Singuläre Erweiterungen galten lange als nicht reproduzierbar und
  funktionierten erst Jahre später in anderer Formulierung.
- Der gesamte Bewertungsteil wurde ab 2018 vom Feld abgeräumt (AlphaZero,
  dann NNUE); Stockfish hat die klassische Bewertung 2023 entfernt. Der
  Suchteil hat besser überlebt als der Bewertungsteil.
- Und die als Einstiegshilfe veröffentlichten „Simplified“-Tabellen haben in
  unserem Test die selbst hergeleiteten Tabellen des Gegners mit 13:7
  geschlagen. Eigene Herleitung ist also für sich genommen noch kein Wert.

Fragen dazu:

- Welche Bausteine von Funken hast du übernommen, **ohne sie je einzeln in
  deiner Engine gemessen zu haben**? Das Aspirationsfenster von ±25 cp ab
  Tiefe 4, R = 2–3 beim Nullzug, die LMR-Formel, die Futility-Margen, die
  Delta-Schwelle in der Ruhesuche, die zwei Killer pro Ply — steht dahinter
  eine Messung von dir oder eine Zahl aus der Überlieferung?
- Eine Ablation ist billig und sehr aufschlussreich: Baue ein Binary
  **ohne** einen dieser Bausteine und lass es gegen den vollen Stand
  spielen. Wenn der Verzicht keine Elo kostet, verdient der Baustein seinen
  Platz nicht — oder er ist bei dir falsch parametriert. Beides möchte man
  wissen. Bei 15 Halbzügen Suchtiefe hast du genug Substanz, dass solche
  Unterschiede sichtbar werden sollten.
- Die Überlieferung ist an Engines mit anderer Gestalt gewachsen: Bitboards,
  Millionen Knoten pro Sekunde, heute neuronale Bewertung. Wo passt ein
  ererbter Parameter nicht zu **deiner** Gestalt — Mailbox, ein Kern, nur
  std, bewusst single-threaded?
- Gibt es eine Entscheidung in Funken, die du auch dann verteidigen würdest,
  wenn das Wiki das Gegenteil sagt? Bei der Wiederholungserkennung hattest
  du sie (Cutoff erst beim dritten Auftreten, Wurzel sucht immer vollständig
  — strenger als das übliche Verfahren). Gibt es weitere?

Das ist ausdrücklich kein Auftrag, gegen den Kanon zu programmieren.
Andersartigkeit ist kein Selbstzweck, das steht so auch in deiner
`AGENTS.md`. Es ist der Auftrag, den Unterschied zwischen *gemessen* und
*überliefert* im eigenen Code zu kennen.

## Konkret bei dir

Drei Punkte, die in deine Richtung zeigen:

1. **Die unerklärten Patzer.** In Serie 1 hast du zwei Partien durch je
   einen einzelnen groben Fehler bei großer Tiefe verloren: 17...a4?? bei
   Tiefe 15 und 19.Tb1?? bei Tiefe 13. Den zweiten habe ich dreimal mit
   denselben Uhrzeiten nachgespielt — dabei kam jedes Mal ein vernünftiger
   Zug heraus. Der Fehler hängt also an Timing oder am Zustand der
   Transpositionstabelle und ist nicht stellungsbedingt. Das ist bis heute
   ungeklärt, und es ist die teuerste Einzelschwäche, die gemessen wurde:
   Ein solcher Patzer kostet die ganze Partie, egal wie gut die anderen 40
   Züge waren. In deiner eigenen `MESSERGEBNISSE.md` steht unter Abschnitt 6
   der wahrscheinlich passende Verdacht: Scores, in deren Teilbaum ein
   Remis-Cutoff steckt, landen in der TT und werden auf Pfaden mit anderer
   Historie wiederverwendet — „pfadabhängige Restungenauigkeit,
   Standardverhalten, nicht als exakt behauptet“. Standardverhalten heißt
   nicht harmlos. Kann daraus ein Zug werden, der in der Partie ein voller
   Figurenverlust ist? Und was passiert bei einem Fensterabbruch (Aspiration
   oder Zeit), wenn die Iteration mitten in der Neusuche endet — welcher Zug
   wird dann ausgegeben?
2. **Du stehst seit dem 18.09. still.** Der Gegner hat in derselben Zeit
   230 Elo aufgeholt. Das ist kein Vorwurf, du hattest keinen Auftrag. Aber
   es heißt: Der nächste Zugewinn muss aus Feinarbeit kommen, nicht mehr aus
   fehlenden Bewertungstermen. Und Feinarbeit braucht das Messverfahren aus
   Denkanstoß 1, sonst misst du Rauschen.
3. **Tiefe 15 gegen 12, und trotzdem nur 12:8.** Drei Halbzüge Vorsprung
   sind viel. Dass daraus kein größerer Abstand wird, ist ein Hinweis
   darauf, dass entweder deine Bewertung in den erreichten Stellungen
   weniger sieht, als die Tiefe verspricht, oder dass einzelne Ausreißer
   (Punkt 1) den Schnitt auffressen. Die PGNs in
   `../engine-arena/match-2026-09-19-b/` beantworten das: In welchen deiner
   10 Nicht-Siege ging es in einem Zug kaputt, und in welchen bist du über
   viele Züge langsam untergegangen?

## Erwartetes Ergebnis

- Eine knappe, ehrliche Antwort auf Denkanstoß 1 und 2 (in
  `MESSERGEBNISSE.md`, `KONZEPT.md` oder einer eigenen Datei): Was an Funken
  ist gemessen, was ist geerbt, und was brauchst du, um das zu ändern.
- Mindestens eine Ablation eines übernommenen Bausteins, mit Zahl und
  Unsicherheit.
- Punkt 1 oben verfolgen: Ursache suchen, und wenn sie gefunden ist, beheben
  und einen Regressionstest dazu. Wenn sie nicht gefunden wird, das ebenso
  klar aufschreiben.
- Weitere Verbesserungen wie bisher: Idee, Umsetzung, Messung gegen den
  vorherigen Stand, nur übernehmen was hilft. Eingefrorene Baseline:
  `../engine-arena/sparkengine-a31601d`. Deinen jeweiligen Stand genauso
  sichern.
- `cargo test` grün, Release-Build ohne Warnungen, Perft-Stichprobe, Doku
  aktuell, keine unbelegten Stärkeangaben.
- Commit(s) mit aussagekräftigen Nachrichten. **Kein Push**, kein
  GitHub-Repo ohne Rückfrage.
- Beim Messen die Regel beibehalten: nur eine Partie gleichzeitig, und nicht
  starten, solange mehr als ein `martuni`-Prozess läuft (`run_series.sh`
  macht das von selbst, `MAX_MARTUNI` bitte auf Standard lassen).
