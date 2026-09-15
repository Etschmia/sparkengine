# Funken — Schachliches Konzept und Entwurfsentscheidungen

Name: **Funken** (Version 1.0). Binary `funken`, UCI-Name `Funken 1.0`.
Annahmen über die Zielhardware: gewöhnlicher CPU-Rechner ohne GPU
(validiert auf 2× AMD EPYC, 3,7 GB RAM, Debian 13, Rust 1.94, Python 3.13).

## 1. Sprach- und Architekturwahl

**Rust**, single-threaded, keine externen Crates (nur Standardbibliothek).

- *Warum Rust:* Zuggenerator und Suche brauchen rohe Schleifenleistung
  (~5–14 Mio. Perft-Knoten/s im Release-Build auf der Test-VM); gleichzeitig
  sollen Bauern- und Rochaderegeln ohne Speicherfehler laufen. Rust gibt beides:
  C-nahe Geschwindigkeit plus Garantien gegen Datenrennen/Use-after-free —
  wichtig für einen Bot, der tagelang unbeaufsichtigt läuft. Kein GC, keine
  Pausen, statisches Binary ohne Laufzeitabhängigkeiten (Deployment per Kopie).
- *Warum single-threaded:* Korrektheit zuerst. SMP (Lazy-SMP/YBWC) verdoppelt
  die Race-Fläche; bei 2 vCPUs ist der Mehrwert klein. Die UCI-Option `Threads`
  existiert, ist aber fix 1 (ehrlich gemeldet statt still ignoriert).
  Mehrthreading ist als dokumentierte Ausbaustufe vorgesehen, nicht behauptet.
- *Warum kein Python für den Kern:* ca. 100× langsamer — für glaubwürdige
  Spielstärke auf CPU ungeeignet. Python wird nur als Test-/Betriebswerkzeug
  eingesetzt (Treiber, Schiedsrichter, Bridge), nie für Zugentscheidungen.

## 2. Brettrepräsentation: Mailbox `[u8; 64]`

Quadrate `a1 = 0 … h8 = 63`, Figuren als Bytes
(`0` leer, `1–6` weiß P/N/B/R/Q/K, `7–12` schwarz), dazu Seite am Zug,
Rochaderechte (4 Bit), En-passant-Feld, Halb-/Vollzugzähler, Königskoordinaten
und inkrementeller Zobrist-Hash (eigener SplitMix64-Strom, fester Seed).

- *Warum kein Bitboard:* Bitboards sind schneller, aber fehleranfälliger in der
  Erstimplementierung (magische Multiplikatoren, Spezialfälle). Die Mailbox mit
  Pseudo-legal + voller Legalitätsprüfung (make → Königscheck → unmake) ist
  einfacher **beweisbar korrekt** — inklusive seltener Fälle wie
  En-passant-Fesselungen — und auf der Referenzmaschine schnell genug
  (Perft ≈ 9–14 Mnps). Stärke kommt hier aus der Suche, nicht aus
  Nanosekunden im Generator. Eigene Entscheidung für Robustheit über Maximaltempo.

## 3. Suche: iterative Vertiefung + Alpha-Beta (eigene Kombination)

Etablierte Verfahren (Lehrbuchstand, u. a. nach Shannon 1950; Knuth/Moore 1975;
Beschreibungen im Chess Programming Wiki — als Theoriequelle genutzt, **kein
Code übernommen**):

- Negamax mit Alpha-Beta-Fenster, iterative Vertiefung, Aspiration (±25 cp ab Tiefe 4)
- Transpositionstabelle (Zobrist-indexiert, tiefenbevorzugte Ersetzung, Matt-Normierung)
- Ruhesuche (nur Schlagzüge + Umwandlungen, Delta-Pruning, Matt-/Patt-Erkennung bei Schach)
- Zugordnung: TT-Zug → MVV-LVA → Killer (2/Ply) → History
- Nullzug-Pruning (R = 2–3), Late-Move-Reduktionen, Schach-Verlängerung,
  Futility-/Reverse-Futility-Pruning, Mattdistanz-Pruning
- Remis: 50-Züge, unzureichendes Material, Stellungswiederholung über Hash-Historie
  (Vereinfachung, offengelegt: im Suchbaum zählt bereits die 2. Wiederholung als
  Remis — stabil, aber theoretisch unexakt; an der Wurzel gilt echtes Dreifach-Remis)

Eigene Anteile: die konkrete Kombination und alle Schwellwerte (Fensterbreiten,
Reduktionstiefen, Margins), die Zeitformel sowie die Bewertungsfunktion (unten).
Keine behauptete Neuheit — das ist solides Handwerk, kein Paper.

## 4. Bewertung: eigene Tapered-Eval (alle Gewichte selbst gewählt)

`evaluate()` liefert Weiß-sicht in Centipawns, getapert zwischen Mittel- und
Endspiel nach unbereinigtem Figurenmaterial (eigene Phasengewichte N/B=1, R=2, Q=4):

- Material (100/320/330/500/900) + **programmatisch erzeugte** Figuren-Felder-Tabellen:
  statt übernommener Zahlen kleine Regeln (Zentralisierung per Distanz zum
  Zentrum, Bauern-Vormarsch quadratisch, Turm auf 7. Reihe, Königssicherheit
  rochiert vs. Zentralisierung im Endspiel). Dadurch ist die Konstruktion
  transparent und garantiert keine Kopie fremder Tabellen.
- Eigene Struktur-/Positionsboni: Läuferpaar, Doppel-/Isolani-/Freibauern
  (fortschrittsabhängig), offene/halboffene Turmlinien, König-Bauernschild,
  Mobilität (N/B/R/Q-gewichtet), Tempo-Bonus.

Keine trainierten Netze, keine fremden Labels — bewusst, weil Datenerzeugung und
Training mit den verfügbaren Ressourcen nicht seriös umsetzbar wären.

## 5. Zeitverwaltung (eigene Formel, offengelegt)

- Plötzlicher Tod: `Basis = Rest/25 + Inkrement/2`; mit `movestogo`: `Rest/movestogo + Ink/2`.
- `movetime`: exakt (minus Overhead). Weiches Limit beendet den Iterationsstart,
  zusätzlich Mid-Iterations-Abbruch (nur mit verwertbarem Tiefe-≥1-Ergebnis);
  hartes Limit bricht Knoten-genau ab (Poll alle 2048 Knoten). `Move Overhead`
  (Default 100 ms) schützt vor Latenz/Flaggen. Messung: 60 s + 1 s → 2903 ms;
  `movetime 500` → 404 ms (siehe `MESSERGEBNISSE.md`).

## 6. UCI und Betrieb

Eigene UCI-Schleife: Standards (`uci/isready/position/go/stop/quit`, `searchmoves`,
`ucinewgame`), Suche im Worker-Thread (stop-fähig), **geflushte Ausgabe**
(Pipe-Deadlock nachweislich gefunden und behoben), Optionsvalidierung,
`bestmove 0000` bei Matt/Patt. Ponder wird ehrlich als „nicht unterstützt"
behandelt (`go ponder` = Suche bis `stop`).

## 7. Bekannte Grenzen (ehrlich)

- Spielstärke gemessen ≈ Stockfish-17-Limiter Stufe 1900 bei 300 ms/Zug
  (4 Partien, Kleinstsample — keine Elo-Behauptung, Details in MESSERGEBNISSE.md).
- Eröffnungsspiel eigenwillig (kein Buch — Vorgabe; Figuren zuerst).
  Eigene Mini-Eröffnungsauswahl wäre regelkonform möglich, fehlt noch.
- Kein SMP, kein Ponder, keine Endspieldatenbanken, keine Aufgabe/Remisangebote
  (bewusste Deaktivierung in der Bridge: jede Partieentscheidung aus eigener
  Berechnung; Matt/Remis adjudiziert Lichess).
- Nur Standard-Schach; Chess960/Varianten abgelehnt.
- Eval-Gewichte ungetunt (Handwerte); systematisches Tuning (z. B. SPSA mit
  selbst erzeugten Daten) ist die größte Stärke-Reserve.
