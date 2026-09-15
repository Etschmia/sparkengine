#!/usr/bin/env python3
"""Minimal UCI driver for testing Funken: waits for bestmove, measures time."""
import subprocess, sys, time, select, os

class Engine:
    def __init__(self, path):
        self.p = subprocess.Popen([path], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, bufsize=0)
        self.buf = b""
        self.send("uci")
        self.wait_prefix("uciok")
        self.send("isready")
        self.wait_prefix("readyok")

    def send(self, cmd):
        self.p.stdin.write((cmd + "\n").encode())
        self.p.stdin.flush()

    def readline(self, timeout=30.0):
        fd = self.p.stdout.fileno()
        while b"\n" not in self.buf:
            r, _, _ = select.select([fd], [], [], timeout)
            if not r:
                raise TimeoutError("no engine output within %.1fs" % timeout)
            chunk = os.read(fd, 65536)
            if not chunk:
                raise EOFError("engine closed stdout")
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return line.decode().strip()

    def wait_prefix(self, prefix, timeout=30.0):
        while True:
            line = self.readline(timeout)
            if line.startswith(prefix):
                return line

    def go(self, args, timeout=120.0):
        infos = []
        t0 = time.time()
        self.send("go " + args)
        while True:
            line = self.readline(timeout)
            if line.startswith("info "):
                infos.append(line)
            elif line.startswith("bestmove"):
                dt = time.time() - t0
                return line.split()[1], dt, infos

    def close(self):
        try:
            self.send("quit")
            self.p.wait(timeout=5)
        except Exception:
            self.p.kill()

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "./target/release/funken"
    e = Engine(path)

    # 1. fixed depth 8 from startpos
    bm, dt, infos = e.go("depth 8")
    print(f"depth8: bestmove={bm} time={dt:.2f}s")
    for i in infos[-3:]:
        print("  ", i)

    # 2. movetime 500
    e.send("position startpos")
    e.wait_idle = None
    bm, dt, infos = e.go("movetime 500")
    print(f"movetime500: bestmove={bm} elapsed={dt*1000:.0f}ms ndepth={infos[-1] if infos else '-'}")

    # 3. simulated 60s+1s clock (white)
    e.send("position startpos")
    bm, dt, infos = e.go("wtime 60000 btime 60000 winc 1000 binc 1000")
    print(f"clock60+1: bestmove={bm} elapsed={dt*1000:.0f}ms last={infos[-1][:120] if infos else '-'}")

    # 4. stop test: infinite search, then stop
    e.send("position startpos")
    e.send("go infinite")
    time.sleep(0.5)
    t0 = time.time()
    e.send("stop")
    line = e.wait_prefix("bestmove", timeout=15.0)
    print(f"stop: {line} after {(time.time()-t0)*1000:.0f}ms")

    e.close()
    print("OK")
