#!/usr/bin/env python3
"""Self-play: Funken vs Funken at fixed depth, python-chess as referee.
Checks: every move legal, game terminates, result plausible, no crashes."""
import sys
sys.path.insert(0, '.')
from tests_ucitool import Engine
import chess

def play_game(path, depth, max_plies=300, game_no=0):
    e = Engine(path)
    board = chess.Board()
    e.send("ucinewgame")
    plies = 0
    while not board.is_game_over(claim_draw=True) and plies < max_plies:
        e.send("position fen " + board.fen())
        mv, dt, infos = e.go(f"depth {depth}")
        if mv == "0000":
            print("  engine reports no move; over?", board.is_game_over())
            break
        try:
            move = chess.Move.from_uci(mv)
            assert move in board.legal_moves, f"ILLEGAL: {mv} in {board.fen()}"
        except AssertionError as ex:
            print("  FAIL:", ex)
            e.close()
            return None
        board.push(move)
        plies += 1
    e.close()
    return board

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "./target/release/funken"
    depth = int(sys.argv[2]) if len(sys.argv) > 2 else 6
    games = int(sys.argv[3]) if len(sys.argv) > 3 else 2
    for g in range(games):
        b = play_game(path, depth, game_no=g)
        if b is None:
            print(f"game {g+1}: FAILED (illegal move)")
            continue
        print(f"game {g+1}: result={b.result(claim_draw=True)} plies={len(b.move_stack)} "
              f"over={b.is_game_over(claim_draw=True)}")
        print("  moves:", " ".join(m.uci() for m in b.move_stack[:20]),
              "..." if len(b.move_stack) > 20 else "")
        print("  final:", b.fen())
    print("DONE")
