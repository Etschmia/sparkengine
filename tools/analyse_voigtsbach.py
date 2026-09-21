#!/usr/bin/env python3
"""Analyse Voigtsbach's own games and publish the results for Spark.

Runs `analyze_blunders.py` over the Lichess-bot PGN archive of the Voigtsbach
account (engine: Funken) and pushes the cumulative result to the host where
sparkengine is developed, so Spark can work on the identified blunders.

Deliberately separate from the Martuni blunder analysis:

* different output file, no `analyse-*` prefix, own remote directory — nothing
  here may ever be picked up by the Martuni side's `analyse-*.json` globs or by
  `schleuse_sync.py`, which merges *everything* in its inbox into Martuni's
  state files;
* yields to the Martuni queue. The schleuse is the paying job; this is not.
  While PGNs wait in the remote outbox, this script defers and exits 0.

Configuration is via SPARK_ANALYSE_* environment variables (see defaults
below), so the same file works on any host.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def _env(name: str, default: str) -> str:
    return os.environ.get(name, default)


ROOT = Path(__file__).resolve().parent.parent

# What we analyse.
GAME_DIR = Path(_env("SPARK_ANALYSE_GAME_DIR", "/var/www/but2/botdir/lichess-bot/game_records"))
PLAYER = _env("SPARK_ANALYSE_PLAYER", "Voigtsbach")

# How we analyse it. Same parameters as the Martuni series so the two engines
# stay comparable; depth in particular should not drift.
PYTHON = _env("SPARK_ANALYSE_PYTHON", sys.executable)
ANALYZE_SCRIPT = _env(
    "SPARK_ANALYSE_SCRIPT", "/var/www/but2/botdir/schleuse-work/analyze_blunders.py"
)
ENGINE = _env("SPARK_ANALYSE_ENGINE", "stockfish")
ENGINE_PATH = _env("SPARK_ANALYSE_ENGINE_PATH", "/usr/games")
DEPTH = int(_env("SPARK_ANALYSE_DEPTH", "17"))
THREADS = int(_env("SPARK_ANALYSE_THREADS", "2"))
HASH_MB = int(_env("SPARK_ANALYSE_HASH", "256"))
MIN_MOVETIME = int(_env("SPARK_ANALYSE_MIN_MOVETIME", "0"))
TIMEOUT_S = int(_env("SPARK_ANALYSE_TIMEOUT_S", str(45 * 60)))

# Where results live locally and where they go.
WORK_DIR = Path(_env("SPARK_ANALYSE_WORK_DIR", "/var/www/but2/botdir/voigtsbach-analyse"))
OUTPUT_NAME = _env("SPARK_ANALYSE_OUTPUT_NAME", "voigtsbach-blunders.json")
REMOTE_HOST = _env("SPARK_ANALYSE_REMOTE_HOST", "martuni.de")
REMOTE_DIR = _env("SPARK_ANALYSE_REMOTE_DIR", "/home/librechat/voigtsbach_analysen")
# Uploading creates a directory on somebody else's host. Off until the owner of
# that host has said yes; analysis still runs and results accumulate locally.
PUBLISH_ENABLED = _env("SPARK_ANALYSE_PUBLISH", "0") not in ("0", "", "no", "false")

# Yielding to the Martuni schleuse.
SCHLEUSE_OUTBOX = _env(
    "SPARK_ANALYSE_SCHLEUSE_OUTBOX", "/home/librechat/grok_bot_schleuse/outbox"
)
# Run only when at most this many PGNs wait in the Martuni outbox.
QUEUE_THRESHOLD = int(_env("SPARK_ANALYSE_QUEUE_THRESHOLD", "0"))

LOG_PATH = Path(_env("SPARK_ANALYSE_LOG_PATH", str(WORK_DIR / "analyse_voigtsbach.log")))


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def log(msg: str) -> None:
    line = f"{utc_now_iso()} {msg}"
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as fh:
        fh.write(line + "\n")
    print(line, flush=True)


def ssh(remote_cmd: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=30", REMOTE_HOST, remote_cmd],
        capture_output=True,
        text=True,
        check=check,
    )


def scp_to(local: Path, remote_path: str) -> None:
    # host:path as ONE argv, unquoted — see schleuse_worker.py for the reason.
    proc = subprocess.run(
        ["scp", "-o", "BatchMode=yes", "-o", "ConnectTimeout=30",
         str(local), f"{REMOTE_HOST}:{remote_path}"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"scp failed rc={proc.returncode}: {(proc.stderr or proc.stdout or '').strip()}"
        )


def martuni_queue_len() -> int:
    """PGNs still waiting in the Martuni outbox."""
    proc = ssh(
        f"find {shlex.quote(SCHLEUSE_OUTBOX)} -maxdepth 1 -type f -name '*.pgn' 2>/dev/null | wc -l",
        check=True,
    )
    try:
        return int(proc.stdout.strip())
    except ValueError:
        return -1


def schleuse_busy_locally() -> bool:
    """True while the schleuse worker has an analysis running on this host."""
    proc = subprocess.run(
        ["pgrep", "-f", "schleuse_worker.py"], capture_output=True, text=True
    )
    if proc.returncode != 0:
        return False
    # The worker idles between games; only an active engine run is contention.
    return subprocess.run(
        ["pgrep", "-x", "stockfish"], capture_output=True, text=True
    ).returncode == 0 or subprocess.run(
        ["pgrep", "-x", "fairy-stockfish"], capture_output=True, text=True
    ).returncode == 0


def publish(output: Path, analysed: int, blunders: int) -> None:
    """Upload result and status atomically (.tmp + mv), like the schleuse does."""
    ssh(f"mkdir -p {shlex.quote(REMOTE_DIR + '/analysen')}", check=True)

    # Orientation for whoever finds this directory (Spark), uploaded once.
    readme = WORK_DIR / "LIESMICH.md"
    if readme.exists():
        present = ssh(
            f"test -e {shlex.quote(REMOTE_DIR + '/LIESMICH.md')} && echo yes || echo no",
            check=True,
        ).stdout.strip()
        if present != "yes":
            scp_to(readme, f"{REMOTE_DIR}/LIESMICH.md.tmp")
            ssh(
                f"mv {shlex.quote(REMOTE_DIR + '/LIESMICH.md.tmp')} "
                f"{shlex.quote(REMOTE_DIR + '/LIESMICH.md')}",
                check=True,
            )
            log("Uploaded LIESMICH.md")

    remote_json = f"{REMOTE_DIR}/analysen/{OUTPUT_NAME}"
    scp_to(output, remote_json + ".tmp")
    ssh(f"mv {shlex.quote(remote_json + '.tmp')} {shlex.quote(remote_json)}", check=True)

    status = {
        "last_run": utc_now_iso(),
        "source": "SYR-PE-BUTDEV",
        "account": PLAYER,
        "engine_under_test": "Funken (sparkengine)",
        "analysis_engine": ENGINE,
        "params": {
            "depth": DEPTH,
            "threads": THREADS,
            "hash": HASH_MB,
            "min_movetime": MIN_MOVETIME,
        },
        "games_analyzed": analysed,
        "blunders": blunders,
        "result_file": f"analysen/{OUTPUT_NAME}",
    }
    local_status = WORK_DIR / "status.json"
    local_status.write_text(json.dumps(status, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    remote_status = f"{REMOTE_DIR}/status.json"
    scp_to(local_status, remote_status + ".tmp")
    ssh(f"mv {shlex.quote(remote_status + '.tmp')} {shlex.quote(remote_status)}", check=True)
    log(f"Published {analysed} game(s), {blunders} blunder(s) to {REMOTE_HOST}:{REMOTE_DIR}")


def read_state(output: Path) -> tuple[int, int]:
    if not output.exists():
        return 0, 0
    try:
        data = json.loads(output.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return 0, 0
    return len(data.get("analyzed_pgns", [])), len(data.get("blunders", []))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--force",
        action="store_true",
        help="Analyse even while the Martuni queue still has work",
    )
    parser.add_argument(
        "--no-publish", action="store_true", help="Analyse locally, do not upload"
    )
    args = parser.parse_args()

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    output = WORK_DIR / OUTPUT_NAME

    if not args.force:
        try:
            queued = martuni_queue_len()
        except subprocess.CalledProcessError as exc:
            log(f"Cannot reach schleuse outbox, deferring: {exc}")
            return 0
        if queued < 0 or queued > QUEUE_THRESHOLD:
            log(f"Martuni queue has {queued} PGN(s) (threshold {QUEUE_THRESHOLD}); deferring")
            return 0
        if schleuse_busy_locally():
            log("Schleuse analysis running locally; deferring")
            return 0

    if not GAME_DIR.is_dir():
        log(f"No game directory: {GAME_DIR}")
        return 1

    before_games, _ = read_state(output)

    env = os.environ.copy()
    env["PATH"] = f"{ENGINE_PATH}:{env.get('PATH', '')}"
    cmd = [
        PYTHON, ANALYZE_SCRIPT,
        "--game-dir", str(GAME_DIR),
        "--player", PLAYER,
        "--engine", ENGINE,
        "--depth", str(DEPTH),
        "--threads", str(THREADS),
        "--hash", str(HASH_MB),
        "--min-movetime", str(MIN_MOVETIME),
        "--output", str(output),
    ]
    log("Running: " + " ".join(shlex.quote(c) for c in cmd))
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, env=env,
            cwd=str(WORK_DIR), timeout=TIMEOUT_S,
        )
    except subprocess.TimeoutExpired:
        log(f"TIMEOUT after {TIMEOUT_S}s")
        return 1
    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or f"rc={proc.returncode}").strip()
        log(f"FAIL rc={proc.returncode}: {err[:2000]}")
        return proc.returncode

    after_games, blunders = read_state(output)
    new = after_games - before_games
    log(f"Analysed {new} new game(s); {after_games} total, {blunders} blunder(s)")

    if new == 0 and before_games > 0:
        log("Nothing new; skipping upload")
        return 0
    if args.no_publish:
        log("--no-publish set; not uploading")
        return 0
    if not PUBLISH_ENABLED:
        log(
            f"Upload disabled (SPARK_ANALYSE_PUBLISH=0); {after_games} game(s) "
            f"held locally in {output}"
        )
        return 0
    try:
        publish(output, after_games, blunders)
    except Exception as exc:
        log(f"Publish failed: {type(exc).__name__}: {exc}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
