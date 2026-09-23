#!/usr/bin/env python3
"""Match-Serie mit Zufalls-Eröffnungen, Precheck und Auswertung.

Jedes Paar spielt dieselbe Zufalls-Eröffnung (6 Halbzüge, Seed+i) mit beiden
Farben. Feste Knoten, sequentiell (eine Partie gleichzeitig).

Aufruf: match_random.py <engA> <engB> <paare> <dir> <nodes> <seed>
  engA/engB: Pfade (Binary oder Wrapper-Skript, z. B. funken-tuned.sh)
  paare: Anzahl Farbtausch-Paare (Partien = 2*paare, mind. 100 empfohlen)
  dir: Zielverzeichnis (wird angelegt)
  nodes: feste Knoten pro Zug (engine_match.py -n)
  seed: Reproduzierbarkeit der Eröffnungen

Ablauf: Gen4-Guard (wartet bei laufenden texel_gen-Jobs) -> precheck pre
(Abbruch bei Divergenz-Fehler) -> Partien via engine_match.py ->
precheck post -> Auswertung (auswertung.py: Score, Elo, CI, LOS).
Dateinamen wie run_series.sh (r01_A_vs_B.pgn, ...), damit precheck post passt.
"""
import os
import random
import subprocess
import sys
import time

import chess

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PRECHECK = os.path.join(REPO, "tools", "precheck.sh")
MATCH_PY = os.environ.get("MATCH_PY",
                           os.path.expanduser("~/engine-arena/engine_match.py"))
AUSWERTUNG = os.environ.get("AUSWERTUNG",
                             os.path.expanduser("~/engine-arena/auswertung.py"))


def opening(rng, plies=6):
    while True:
        b = chess.Board()
        for _ in range(plies):
            moves = list(b.legal_moves)
            if not moves or b.is_game_over():
                break
            b.push(rng.choice(moves))
        else:
            if not b.is_check():
                return b.fen()
            continue
        continue


def texel_running():
    try:
        out = subprocess.run(["pgrep", "-cf", "texel_gen.py"],
                             capture_output=True, text=True).stdout.strip()
        return int(out or 0)
    except Exception:
        return 0


def main():
    eng_a, eng_b, paare, d, nodes, seed = (sys.argv[1], sys.argv[2],
                                           int(sys.argv[3]), sys.argv[4],
                                           sys.argv[5], int(sys.argv[6]))
    os.makedirs(d, exist_ok=True)
    # Test-Hook TEXEL_GUARD=off nur für Tool-Tests (trockene Mechanik-Probe),
    # nie für echte Matches (Guard schützt Gen4-Langläufe vor Konkurrenz).
    if os.environ.get("TEXEL_GUARD", "on") == "on":
        while texel_running():
            print("warte: texel_gen-Langlauf aktiv (60 s)...", flush=True)
            time.sleep(60)
    r = subprocess.run([PRECHECK, "pre", eng_a, eng_b, d])
    if r.returncode != 0:
        print("PRECHECK pre fehlgeschlagen — kein Match.", flush=True)
        sys.exit(1)
    rng = random.Random(seed)
    n = 0
    for i in range(paare):
        fen = opening(rng)
        for w, b in ((eng_a, eng_b), (eng_b, eng_a)):
            n += 1
            rr = f"{n:02d}" if n < 100 else str(n)
            out = (f"{d}/r{rr}_{os.path.basename(w)}_vs_"
                   f"{os.path.basename(b)}.pgn")
            log = f"{d}/r{rr}.log"
            if os.path.exists(out) and os.path.getsize(out):
                print(f"Runde {rr} existiert, übersprungen", flush=True)
                continue
            with open(log, "w") as lf:
                subprocess.run(
                    [sys.executable, MATCH_PY, w, b, "-r", str(n),
                     "-e", f"{os.path.basename(eng_a)} vs "
                           f"{os.path.basename(eng_b)}, Serie {d}",
                     "-o", out, "-n", nodes, "-f", fen, "-q"],
                    stdout=lf, stderr=subprocess.STDOUT)
            print(open(log).read().strip().splitlines()[-1], flush=True)
    print("Serie beendet", flush=True)
    r = subprocess.run([PRECHECK, "post", d])
    if r.returncode != 0:
        print("PRECHECK post: UNGÜLTIG — keine Auswertung.", flush=True)
        sys.exit(1)
    subprocess.run([sys.executable, AUSWERTUNG, d])


if __name__ == "__main__":
    main()
