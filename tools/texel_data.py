#!/usr/bin/env python3
"""Texel-Datensatz aus Selbstspiel-PGNs (eigene Daten, keine fremden Labels).

Filter (ruhige, aussagekraeftige Stellungen):
- Halbzug >= 24 (keine Theorie, kein Zufall der ersten Zuege)
- Seite am Zug steht nicht im Schach
- kein Matt/Patt auf dem Brett (Spiel laeuft)
- Duplikate raus (FEN ohne Zaehler)

Ausgabe: TSV `fen<TAB>result` (result aus Weiss-Sicht: 1.0/0.5/0.0).
Aufruf: texel_data.py <pgn-verzeichnis> <train.tsv> <holdout.tsv>
Holdout: jede 10. Partie (partieweise, kein Leck).
"""
import glob
import sys

import chess.pgn


def main():
    pgndir, train_path, hold_path = sys.argv[1], sys.argv[2], sys.argv[3]
    files = sorted(glob.glob(f"{pgndir}/*.pgn"))
    train, hold = [], []
    seen = set()
    n_games, n_pos, n_dup, n_check = 0, 0, 0, 0
    for gi, f in enumerate(files):
        with open(f) as fh:
            game = chess.pgn.read_game(fh)
        if game is None:
            continue
        r = game.headers.get("Result", "*")
        if r == "1-0":
            res = 1.0
        elif r == "0-1":
            res = 0.0
        elif r.startswith("1/2"):
            res = 0.5
        else:
            continue
        n_games += 1
        dest = hold if gi % 10 == 9 else train
        node = game
        while node.variations:
            parent = node
            node = node.variations[0]
            b = node.board()
            if b.ply() < 24:
                continue
            if b.is_check() or b.is_game_over():
                n_check += 1
                continue
            # Nach Schlagzug/Umwandlung: statische Eval ist Rauschen -> raus
            mv = node.move
            if mv is not None and (parent.board().is_capture(mv) or mv.promotion):
                n_check += 1
                continue
            fen = b.fen()
            key = " ".join(fen.split()[:4])
            if key in seen:
                n_dup += 1
                continue
            seen.add(key)
            dest.append((fen, res))
            n_pos += 1
    for path, rows in ((train_path, train), (hold_path, hold)):
        with open(path, "w") as fh:
            for fen, res in rows:
                fh.write(f"{fen}\t{res}\n")
    print(f"Partien: {n_games}, Positionen: {n_pos} "
          f"(train {len(train)}, holdout {len(hold)}), "
          f"dup {n_dup}, schach/beendet {n_check}")


if __name__ == "__main__":
    main()
