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
import re
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


# Mirrors analyze_cron.VANILLA_VARIANTS on the Martuni server, so "needs a
# variant engine" means the same thing on both sides.
VANILLA_VARIANTS = {
    "",
    "standard",
    "chess",
    "normal",
    "from position",
    "chess960",
    "fischerandom",
    "fischerrandom",
}


def pgn_variant(pgn_text: str) -> str:
    """Variant name as analyze_cron.read_pgn_variant() reads it."""
    for line in pgn_text.splitlines():
        line = line.strip()
        if not line:
            break  # blank line ends the header block
        if line.startswith("[Variant "):
            parts = line.split('"')
            if len(parts) >= 2:
                return parts[1].strip().lower()
    return "standard"


def pgn_headers(pgn_text: str) -> dict[str, str]:
    """Tag pairs from the first header block."""
    out: dict[str, str] = {}
    for line in pgn_text.splitlines():
        line = line.strip()
        if not line:
            break  # blank line ends the header block
        m = re.match(r'\[(\w+)\s+"(.*)"\]$', line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def collect_games() -> list[dict]:
    """Metadata for every game in the archive, analysed or not.

    Written for the status command on the sparkengine dev host, which shows
    "games total" and "of those analysed" as two separate numbers — so this
    deliberately lists *all* games, not just the ones already in
    voigtsbach-blunders.json. Everything comes from the PGN headers; no API
    calls.
    """
    games = []
    for pgn in sorted(GAME_DIR.glob("*.pgn")):
        text = pgn.read_text(encoding="utf-8", errors="replace")
        h = pgn_headers(text)
        white, black = h.get("White", ""), h.get("Black", "")
        if PLAYER.lower() == white.lower():
            opponent, title = black, h.get("BlackTitle", "")
        elif PLAYER.lower() == black.lower():
            opponent, title = white, h.get("WhiteTitle", "")
        else:
            continue  # not one of our games

        started = None
        date, clock = h.get("UTCDate", ""), h.get("UTCTime", "")
        if date and clock:
            started = f"{date.replace('.', '-')}T{clock}Z"

        games.append({
            "pgn": pgn.name,
            "opponent": opponent,
            "opponent_is_bot": title.upper() == "BOT",
            "variant": pgn_variant(text),
            "started_at": started,
            "result": h.get("Result"),
            "time_control": h.get("TimeControl"),
        })
    games.sort(key=lambda g: (g["started_at"] or "", g["pgn"]))
    return games


def games_path() -> Path:
    """Local games.json — the record of what was last *published*, not merely
    computed. It is written inside publish(), after a successful upload, so a
    dry run can never make the next real run believe the remote is current."""
    return WORK_DIR / "games.json"


def games_differ(games: list[dict]) -> bool:
    """True if `games` differs from what we last published."""
    path = games_path()
    if not path.exists():
        return True
    try:
        old = json.loads(path.read_text(encoding="utf-8")).get("games", [])
    except (OSError, json.JSONDecodeError):
        return True
    return json.dumps(old, ensure_ascii=False, sort_keys=True) != json.dumps(
        games, ensure_ascii=False, sort_keys=True
    )


def write_games_local(games: list[dict]) -> None:
    games_path().write_text(
        json.dumps(
            {"version": 1, "updated_at": utc_now_iso(), "games": games},
            ensure_ascii=False, indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


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


def sync_pgns() -> int:
    """Mirror the PGN archive to the remote directory, incrementally.

    Spark works on the engine, and the analysis JSON only carries symptoms
    (eval, best move, motif). Replaying a position in Funken needs the game
    itself: opening repertoire, clock situation, endgame technique, repetition
    patterns. Only files not already there are copied, so this stays cheap as
    the archive grows.
    """
    remote_dir = f"{REMOTE_DIR}/pgn"
    ssh(f"mkdir -p {shlex.quote(remote_dir)}", check=True)
    have = {
        n for n in ssh(
            f"find {shlex.quote(remote_dir)} -maxdepth 1 -type f -name '*.pgn' -printf '%f\\0'",
            check=True,
        ).stdout.split("\0") if n
    }
    missing = [p for p in sorted(GAME_DIR.glob("*.pgn")) if p.name not in have]
    for pgn in missing:
        # .tmp + mv, so a reader never sees a half-written game
        remote = f"{remote_dir}/{pgn.name}"
        scp_to(pgn, remote + ".tmp")
        ssh(f"mv {shlex.quote(remote + '.tmp')} {shlex.quote(remote)}", check=True)
    if missing:
        log(f"Synced {len(missing)} new PGN(s) to {REMOTE_HOST}:{remote_dir}")
    return len(have) + len(missing)


def publish(output: Path, analysed: int, blunders: int, games: list[dict] | None) -> None:
    """Upload result and status atomically (.tmp + mv), like the schleuse does."""
    ssh(f"mkdir -p {shlex.quote(REMOTE_DIR + '/analysen')}", check=True)
    pgn_count = sync_pgns()

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
        "games_total": len(games) if games is not None else analysed,
        "blunders": blunders,
        "result_file": f"analysen/{OUTPUT_NAME}",
        "games_file": "games.json",
        "pgn_dir": "pgn/",
        "pgn_files": pgn_count,
    }
    if games is not None:
        remote_games = f"{REMOTE_DIR}/games.json"
        tmp = WORK_DIR / "games.json.tmp"
        tmp.write_text(
            json.dumps(
                {"version": 1, "updated_at": utc_now_iso(), "games": games},
                ensure_ascii=False, indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        scp_to(tmp, remote_games + ".tmp")
        ssh(f"mv {shlex.quote(remote_games + '.tmp')} {shlex.quote(remote_games)}", check=True)
        tmp.replace(games_path())  # only now is the local copy the published state

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

    # analyze_blunders.py defaults --player to "Martuni" because it comes from
    # that project. Silently inheriting it here would analyse the opponent's
    # moves instead of Funken's, and the numbers would look plausible for a
    # long time. Refuse rather than produce a wrong series.
    if PLAYER.strip().lower() in ("", "martuni"):
        log(f"Refusing to run: SPARK_ANALYSE_PLAYER is {PLAYER!r}, expected the "
            f"Voigtsbach account. This analyses Funken's own moves, not Martuni's.")
        return 2

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

    # One engine is used for the whole directory, and it is Stockfish. That is
    # correct only as long as Funken plays nothing but standard chess (the
    # bridge config accepts `variants: [standard]`). If a variant ever shows up
    # here, Stockfish would analyse it as if it were normal chess and produce
    # numbers that look entirely plausible. Refuse instead — the same reasoning
    # as the --player guard above.
    odd = sorted(
        {
            v
            for pgn in GAME_DIR.glob("*.pgn")
            if (v := pgn_variant(pgn.read_text(encoding="utf-8", errors="replace")))
            not in VANILLA_VARIANTS
        }
    )
    if odd:
        log(f"Refusing to run: non-standard variant(s) present in {GAME_DIR}: "
            f"{', '.join(odd)}. This script analyses everything with "
            f"{ENGINE!r}; variants need fairy-stockfish and a separate series.")
        return 3

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

    games = collect_games()
    games_changed = games_differ(games)

    if new == 0 and before_games > 0 and not games_changed:
        log("Nothing new; skipping upload")
        return 0
    if games_changed:
        log(f"games.json: {len(games)} game(s) total, {after_games} analysed")
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
        publish(output, after_games, blunders, games)
    except Exception as exc:
        log(f"Publish failed: {type(exc).__name__}: {exc}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
