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
steht auf `Martuni`, ohne den Schalter würde in Voigtsbachs Partien die
falsche Seite ausgewertet.

## Übertragung — Schalter, bis freigegeben

```
SPARK_ANALYSE_PUBLISH=0
```

Der Upload legt ein Verzeichnis auf einem **fremden** Host an. Der Schalter
steht deshalb auf 0, bis der Eigentümer dieses Hosts zugestimmt hat. Solange
analysiert der Dienst nur lokal und sammelt die Ergebnisse an; hochgeladen
wird nichts, es wird nicht einmal ein `mkdir` abgesetzt.

Nach Freigabe auf `1` setzen und `systemctl restart voigtsbach-analyse.timer`.
Übertragen werden dann, atomar per `.tmp` + `mv`:

```
<REMOTE_DIR>/analysen/voigtsbach-blunders.json
<REMOTE_DIR>/status.json
<REMOTE_DIR>/LIESMICH.md        (einmalig, Orientierung für Spark)
```

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
