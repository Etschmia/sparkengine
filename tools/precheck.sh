#!/usr/bin/env bash
# Pflicht-Sanity-Check vor/nach jedem Engine-Match (Befund 23.09.2026:
# ablation-tuned-200k maß Basis gegen Basis, 20/20 identische Paare).
#
#   tools/precheck.sh pre <engA> <engB> <matchdir>
#     Beide Kandidaten (inkl. Wrapper/Env, z. B. FUNKEN_PARAMS) laufen `bench`.
#     Die Knotenzahlen ALLER Bench-Stellungen müssen sich unterscheiden,
#     sonst Abbruch (Exit 1). md5 + Env-Doku landen in PRECHECK.log.
#   tools/precheck.sh post <matchdir>
#     Zählt identische Partiepaare (r01/r02, r03/r04, ...: gleiche Eröffnung,
#     Farben getauscht). Mehr als 10 % identisch -> Match UNGÜLTIG (Exit 1).
set -u

cmd=${1:-}; case "$cmd" in pre|post) shift;; *) echo "Aufruf: $0 pre <engA> <engB> <matchdir> | post <matchdir>" >&2; exit 2;; esac

if [ "$cmd" = pre ]; then
    A=$1; B=$2; DIR=$3
    mkdir -p "$DIR"; LOG="$DIR/PRECHECK.log"
    { echo "Precheck $(date '+%F %T')"; echo "A=$A"; echo "B=$B"; } > "$LOG"
    for E in "$A" "$B"; do
        [ -x "$E" ] || { echo "FEHLER: nicht ausführbar: $E" | tee -a "$LOG" >&2; exit 2; }
        echo "--- $E" >> "$LOG"
        md5sum "$E" >> "$LOG"
        if [ -n "${FUNKEN_PARAMS:-}" ]; then
            echo "FUNKEN_PARAMS=$FUNKEN_PARAMS" >> "$LOG"
            [ -r "${FUNKEN_PARAMS:-}" ] && md5sum "$FUNKEN_PARAMS" >> "$LOG"
        fi
    done
    bench_nodes() { "$1" bench 2>&1 | grep -o 'nodes [0-9]*' | tr '\n' ' '; }
    NA=$(bench_nodes "$A"); NB=$(bench_nodes "$B")
    echo "bench A: $NA" | tee -a "$LOG"
    echo "bench B: $NB" | tee -a "$LOG"
    if [ "$NA" = "$NB" ]; then
        echo "ABBRUCH: beide Kandidaten liefern identische Bench-Knoten ($NA)" | tee -a "$LOG"
        echo "entweder identische Binaries oder Wrapper-Env wird ignoriert." | tee -a "$LOG"
        exit 1
    fi
    echo "PRE OK: Kandidaten unterscheiden sich." | tee -a "$LOG"
    exit 0
fi

# post
DIR=$1
python3 - "$DIR" <<'PY'
import glob, os, sys
import chess.pgn
d = sys.argv[1]
files = sorted(glob.glob(os.path.join(d, 'r[0-9][0-9]_*.pgn')))
games = []
for f in files:
    g = chess.pgn.read_game(open(f))
    if g is not None:
        games.append([m.uci() for m in g.mainline_moves()])
pairs = len(games) // 2
ident = sum(1 for i in range(0, 2 * pairs, 2) if games[i] == games[i + 1])
print(f"Partien: {len(games)}, Paare: {pairs}, identisch: {ident}")
open(os.path.join(d, 'PRECHECK.log'), 'a').write(f"post {pairs} Paare, {ident} identisch\n")
if pairs == 0:
    print("KEIN BEFUND: keine Paare gefunden"); sys.exit(2)
if ident > pairs // 10:
    print(f"UNGÜLTIG: mehr als 10 % identische Paare ({ident}/{pairs})"); sys.exit(1)
print("POST OK")
PY
