#!/usr/bin/env python3
"""Match harness: Funken vs an external *evaluation-only* opponent
(Stockfish, used solely for strength measurement -- never for move choice).
Referee: python-chess. Adjudication: natural game end or 300-ply cap (draw).
"""
import sys
sys.path.insert(0, '.')
from tests_ucitool import Engine
import chess

class Opponent(Engine):
    def setup_stockfish(self, elo=None, movetime=300):
        if elo is not None:
            self.send("setoption name UCI_LimitStrength value true")
            self.send(f"setoption name UCI_Elo value {elo}")
        self.send("isready")
        self.wait_prefix("readyok")
        self.movetime = movetime

def play(engine_w, engine_b, go_w, go_b, max_plies=300, verbose=False):
    board = chess.Board()
    engines = {chess.WHITE: (engine_w, go_w), chess.BLACK: (engine_b, go_b)}
    plies = 0
    while not board.is_game_over(claim_draw=True) and plies < max_plies:
        eng, go = engines[board.turn]
        eng.send("position fen " + board.fen())
        mv, dt, _ = eng.go(go)
        if mv == "0000":
            break
        move = chess.Move.from_uci(mv)
        if move not in board.legal_moves:
            return "ILLEGAL:" + mv, board
        board.push(move)
        plies += 1
    res = board.result(claim_draw=True)
    if plies >= max_plies and res == "*":
        res = "1/2-1/2"
    return res, board

if __name__ == "__main__":
    funken_path = sys.argv[1] if len(sys.argv) > 1 else "./target/release/funken"
    sf_path = sys.argv[2] if len(sys.argv) > 2 else "/usr/games/stockfish"
    elo = int(sys.argv[3]) if len(sys.argv) > 3 else 1500
    fmove = f"movetime {sys.argv[4]}" if len(sys.argv) > 4 else "movetime 300"
    smove = f"movetime {sys.argv[5]}" if len(sys.argv) > 5 else "movetime 300"

    f = Engine(funken_path)
    s = Opponent(sf_path)
    s.setup_stockfish(elo=elo)
    print(f"Funken({fmove}) vs Stockfish17(Elo {elo}, {smove})", flush=True)

    score = 0.0
    for i in range(4):
        if i % 2 == 0:
            res, b = play(f, s, fmove, smove)
            tag = "F-S"
        else:
            res, b = play(s, f, smove, fmove)
            tag = "S-F"
        pts = {"1-0": (1.0 if tag == "F-S" else 0.0),
               "0-1": (0.0 if tag == "F-S" else 1.0)}.get(res, 0.5)
        score += pts
        print(f"game {i+1} [{tag}]: {res} plies={len(b.move_stack)} funken_pts={score:.1f}", flush=True)
    print(f"FINAL: Funken {score:.1f}/4 vs SF{elo}")
    f.close(); s.close()
