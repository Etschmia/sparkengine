# Betrieb: Blunder-Analyse der Voigtsbach-Partien

Analysiert die Partien, die **Voigtsbach** mit Funken auf Lichess gespielt
hat, und schiebt die Ergebnisse zu dem Host, auf dem sparkengine entwickelt
wird, damit dort an den gefundenen Blundern gearbeitet werden kann.

Läuft auf dem Analyse-Host **SYR-PE-BUTDEV**, nicht auf dem Martuni-Server.

## Abgrenzung zur Martuni-Analyse

Beides läuft auf demselben Rechner und benutzt dasselbe Analyseskript, ist
aber strikt getrennt. Diese Trennung ist keine Kosmetik:

| | Martuni | Voigtsbach |
|---|---|---|
| Quelle | `martuni.de:grok_bot_schleuse/outbox` | lokal `lichess-bot/game_records` |
| Transport | Schleuse, beidseitig | einseitiger Push |
| Ziel | `grok_bot_schleuse/inbox` | eigenes Verzeichnis |
| Zustandsdatei | `enginemartuni/analyse-*.json` | `voigtsbach-blunders.json` |
| Dienst | `martuni-schleuse-worker.service` | `voigtsbach-analyse.timer` |

**Warum das strikt bleiben muss:** `schleuse_sync.py` auf dem Martuni-Server
merged **jede** Datei aus `grok_bot_schleuse/inbox/*.json` ungeprüft in
Martunis Zustandsdateien. Eine Voigtsbach-Analyse, die dort landet,
verfälscht Martunis Blunder/Partie-Kennzahlen — und ist hinterher kaum wieder
herauszutrennen. Deshalb: **nie** in die Schleuse schreiben, **nie** das
Namensmuster `analyse-*` benutzen, **nie** nach `enginemartuni/` ablegen.

Randfall: Voigtsbach und Martuni spielen gelegentlich gegeneinander. Solche
Partien liegen in beiden Reihen, aus je anderer Perspektive analysiert. Das
ist korrekt und keine Doppelzählung — aber nur, solange die Ausgabedateien
getrennt bleiben.

## Vorrang: diese Analyse ist die unwichtigste Last auf dem Host

Drei Verbraucher teilen sich 4 Kerne:

| Priorität | Was | Warum |
|---|---|---|
| 1 | `lichess-bot-voigtsbach` | Funken spielt unter Zeitdruck, verliert sonst nach Zeit |
| 2 | `martuni-schleuse-worker` (Nice 10) | Zusage an den Martuni-Betrieb |
| 3 | `voigtsbach-analyse` (Nice 15) | kann beliebig warten |

Zusätzlich zur Nice-Priorität **hält sich das Skript aktiv zurück**: Es prüft
vor jedem Lauf die Martuni-Warteschlange und bricht ab (rc=0), solange dort
PGNs warten oder lokal gerade eine Schleusen-Analyse rechnet. Der Timer darf
deshalb bedenkenlos alle 30 Minuten feuern.

```
SPARK_ANALYSE_QUEUE_THRESHOLD=0   # nur laufen, wenn Martunis outbox leer ist
```

## Konfiguration

`/var/www/but2/botdir/voigtsbach-analyse/analyse.env`, alle Werte als
`SPARK_ANALYSE_*` überschreibbar. Parameter bewusst identisch zu Martunis
Reihe (`--depth 17 --threads 2 --hash 256 --min-movetime 0`), damit Funkens
und Martunis Kennzahlen vergleichbar bleiben.

Analyse-Engine ist **Stockfish 17.1**. Fairy-Stockfish wird nicht gebraucht —
Funken spielt nur `standard`.

`--player Voigtsbach` ist zwingend: Die Vorgabe von `analyze_blunders.py`
steht auf `Martuni` — das Skript stammt aus jenem Projekt. Würde der Default
hier unbemerkt greifen, analysierte der Lauf die Züge des **Gegners** statt
Funkens eigene, und die Zahlen sähen lange plausibel aus. Das Skript bricht
deshalb mit rc=2 ab, wenn `SPARK_ANALYSE_PLAYER` leer ist oder auf `Martuni`
steht, statt eine falsche Reihe zu produzieren.

## Übertragung

```
SPARK_ANALYSE_PUBLISH=1
```

Ziel: `martuni.de:/home/librechat/voigtsbach_analysen/`, freigegeben am
21.09.2026. **Der Geltungsbereich ist genau dieses Verzeichnis und dieser
Zweck.** Schleuse, `enginemartuni/` und `lichess-bot/` auf jenem Host sind
davon nicht gedeckt — dort wird weiterhin nichts geschrieben.

Auf `0` gesetzt analysiert der Dienst nur lokal und lädt nichts hoch; es wird
dann nicht einmal ein `mkdir` abgesetzt. Das ist der Zustand, in dem der
Dienst ausgeliefert wurde, bevor die Zustimmung vorlag.

Übertragen wird atomar per `.tmp` + `mv`:

```
<REMOTE_DIR>/analysen/voigtsbach-blunders.json
<REMOTE_DIR>/games.json         (Metadaten ALLER Partien seit Stichtag, auch unanalysierter)
<REMOTE_DIR>/status.json
<REMOTE_DIR>/LIESMICH.md        (einmalig, Orientierung für Spark)
```

`games.json` speist einen Statusbefehl auf dem Zielhost. Alle Felder stammen
aus den PGN-Headern, es gibt keine API-Abfragen. Bewusst **alle** Partien,
nicht nur die analysierten — sonst wären „gesamt" und „davon analysiert"
zwangsläufig gleich und die Anzeige nutzlos.

Die lokale `games.json` ist der Nachweis des **zuletzt veröffentlichten**
Standes, nicht des zuletzt berechneten: Sie wird erst nach erfolgreichem
Upload geschrieben. Ein Trockenlauf (`--no-publish`) legt sie deshalb nicht
an — täte er es, hielte der nächste echte Lauf den Remote-Stand für aktuell
und überspränge den Upload.

## Stichtag-Schnitt bei Engine-Wechsel

**Zweck:** Funken wird auf martuni.de weiterentwickelt. Ausgewertet werden
dort nur die Lichess-Partien, die **seit der letzten Engine-Änderung**
gespielt wurden. Nur diese sagen etwas über den aktuellen Stand aus.
Voigtsbachs Zähler auf Martuni (gespielt / analysiert) soll deshalb bei jedem
Engine-Wechsel wieder bei 0 anfangen.

**Mechanik:** `SPARK_ANALYSE_SINCE` in `analyse.env` (ISO, UTC). Partien, deren
`UTCDate`/`UTCTime` vor dem Stichtag liegt, fallen aus **allem**, was
hochgeladen wird: PGN-Sync, `games.json`, `status.json` und Blunder-JSON.
`status.json` trägt den Wert als `published_since`. Lokal bleibt alles
vollständig, also `game_records/` und die kumulative
`voigtsbach-blunders.json`. Nichts wird neu analysiert.

**Nicht** `game_records/` leeren oder verschieben: `angstgegner.py` und
`analyze_opponents.py` in `lichess-bot/` lesen dort die Gegnerhistorie.
Auch die lokale Blunder-JSON nicht rotieren, der Filter reicht.

**Warum Martuni allein nicht archivieren kann:** Der Upload vergleicht, was
*auf Martuni* in `pgn/` liegt (nur oberste Ebene), und kopiert alles
Fehlende nach. Die drei JSON-Dateien werden jedes Mal komplett ersetzt.
Ohne Stichtag hier lädt der nächste Lauf also den ganzen Altbestand wieder
hoch.

### Ablauf (zuletzt 24.09.2026, Gen4)

1. Engine bauen, Bot neu starten (graceful, siehe `BETRIEB-voigtsbach.md`
   im lichess-bot). Stichtag ist der **echte Neustart-Zeitpunkt** in UTC:

   ```bash
   date -u -d "$(systemctl show lichess-bot-voigtsbach.service \
       -p ActiveEnterTimestamp --value)" +%FT%TZ
   ```

   Nicht die Zeit der letzten hochgeladenen Partie nehmen: Eine Partie kann
   nach dem Neustart begonnen haben und schon oben liegen. Am 24.09. war das
   `6bLSKbSO` (Start 12:37:14Z, Neustart 12:34:47Z), also schon Gen4.
2. `sudo systemctl stop voigtsbach-analyse.timer`
3. In `analyse.env` `SPARK_ANALYSE_SINCE=<Stichtag>` setzen und den
   Kommentar darüber anpassen. Offline prüfen, was künftig rausgeht:

   ```bash
   set -a && . ../voigtsbach-analyse/analyse.env && set +a
   python3 -c "import sys; sys.path.insert(0,'tools'); import analyse_voigtsbach as a; \
       print([p.name for p in a.publishable_pgns()])"
   ```
4. Den Abschnitt „Stichtag“ in `voigtsbach-analyse/LIESMICH.md` anpassen.
5. Martuni melden: Stichtag, Zahlen alt/neu. Martuni verschiebt den
   **gesamten** Inhalt von `~/voigtsbach_analysen/` in ein Archiv, z. B.
   `archiv-vor-<engine>-<datum>/`. `LIESMICH.md` gehört dazu, denn es wird
   nur hochgeladen, wenn es fehlt.
6. Nach Martunis OK: `sudo systemctl start voigtsbach-analyse.timer` und
   `sudo systemctl start voigtsbach-analyse.service` für einen Sofortlauf.
   Prüfen: `status.json` auf Martuni zeigt nur die Partien ab Stichtag und
   `published_since`.

Das lokale Log schreibt weiter die lokale Gesamtzahl („N analysed“). Die
hochgeladenen Zahlen stehen in der `Published …`-Zeile.

## Varianten-Wächter

Der Lauf analysiert das ganze Verzeichnis mit **einer** Engine, und das ist
Stockfish. Richtig ist das nur, solange Funken ausschließlich Standardschach
spielt (die Bridge nimmt `variants: [standard]` an). Taucht je eine Variante
in `game_records/` auf, bricht das Skript mit `rc=3` ab, statt sie von
Stockfish als normales Schach rechnen zu lassen — dieselbe Begründung wie
beim `--player`-Wächter: Das Ergebnis wäre nicht offensichtlich falsch,
sondern plausibel falsch.

Wenn Varianten dazukommen sollen, braucht es hier dieselbe Weiche wie im
Schleusen-Worker (`detect_engine`) und eine getrennte Ausgabedatei.

## Betrieb

```bash
systemctl list-timers voigtsbach-analyse.timer
journalctl -u voigtsbach-analyse.service -n 20
tail -20 /var/www/but2/botdir/voigtsbach-analyse/analyse_voigtsbach.log

# Sofort laufen lassen, auch wenn Martuni noch Arbeit hat:
sudo systemctl stop voigtsbach-analyse.timer   # sonst feuert der Timer dazwischen
cd /var/www/but2/botdir/sparkengine
set -a && . ../voigtsbach-analyse/analyse.env && set +a
../enginemartuni/.venv/bin/python3 tools/analyse_voigtsbach.py --force --no-publish
```

`--force` übergeht die Warteschlangen-Sperre, `--no-publish` unterdrückt den
Upload. Beides nur für Tests — im Dauerbetrieb soll die Sperre greifen.

## Ergebnisdatei

`voigtsbach-blunders.json` ist **kumulativ**: `analyze_blunders.py --output`
lädt den vorhandenen Stand, überspringt bereits analysierte PGNs und schreibt
die Vereinigung. Die Datei wächst also und wird nicht ersetzt. Löschen
erzwingt eine Neuanalyse aller Partien.

Bewusst **eine** Datei mit stabilem Namen, kein Datum darin. Martunis Reihe
trägt zwar Datumslabel (`analyse-06.10.2026.json`), die sind dort aber von
Hand vergeben und laufen der Realität voraus — kein Zeitstempel, keine
Rotationsregel. Inhaltlich sind jene Dateien genauso kumulativ wie diese. Ein
stabiler Name ist für den Konsumenten zudem einfacher: Spark liest immer
dieselbe Datei, statt herauszufinden, welches Label gerade gilt.

### Wenn die Datei zu groß wird

Jeder Lauf liest die Datei komplett ein und schreibt sie komplett neu. Zum
Vergleich: Martunis Varianten-Datei liegt bei 1,0 MB und wird grob alle zwei
bis vier Wochen archiviert (27 Mal bisher) — vermutlich genau deshalb.

Bei einigen MB ist hier derselbe Punkt erreicht. Dann:

```
analysen/archiv/voigtsbach-blunders-<bis-datum>.json    # echtes Datum diesmal
```

**Die Falle dabei** (aus dem Martuni-Betrieb): Dort steht der Dateiname in
`analyze_cron.config.json`, und wird er nach dem Archivieren nicht angepasst,
schreibt der nächste Lauf munter in die eben archivierte Datei weiter. Hier
ist die entsprechende Stelle `SPARK_ANALYSE_OUTPUT_NAME` in `analyse.env` —
beim Archivieren als Erstes dorthin schauen.
