#!/usr/bin/env python3
"""Texel-Datenerzeugung v3: diverse Selbstspiele aus Zufalls-Eroeffnungen.

Problem v2: deterministische Engine + 40 Buchstellungen = nur ~80 distinkte
Partien (64k Duplikate). Fix: 6 Zufalls-Halbzuege ab Grundstellung (Seed ->
reproduzierbar), dann Engine gegen sich. Eigene Daten, keine fremden Labels.

Aufruf: texel_gen.py <outdir> <anzahl> <knoten> <seed>
Namen: funken-base (nie Martuni — Trennung zum Schleusen-Betrieb).
"""
import random
import subprocess
import sys

import chess

BASE = "/tmp/opencode/funken-base"
MATCH = "/home/librechat/engine-arena/engine_match.py"


def random_opening(rng, plies=6):
    b = chess.Board()
    for _ in range(plies):
        moves = list(b.legal_moves)
        if not moves or b.is_game_over():
            return None
        b.push(rng.choice(moves))
    if b.is_check():
        return None
    return b.fen()


def main():
    outdir, count, nodes, seed = sys.argv[1], int(sys.argv[2]), sys.argv[3], int(sys.argv[4])
    rng = random.Random(seed)
    ok = 0
    i = 0
    while ok < count:
        i += 1
        fen = random_opening(rng)
        if fen is None:
            continue
        out = f"{outdir}/g{ok:04d}.pgn"
        r = subprocess.run(
            ["python3", MATCH, BASE, BASE, "-o", out, "-n", nodes,
             "-f", fen, "-e", "texel-gen", "-q"],
            capture_output=True, text=True)
        if r.returncode != 0:
            print(f"FEHLER {out}: {r.stderr[-500:]}", flush=True)
            continue
        ok += 1
        if ok % 100 == 0:
            print(f"{ok}/{count}", flush=True)


if __name__ == "__main__":
    main()
