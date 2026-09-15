#!/usr/bin/env bash
# Funken-Bot starten (Vordergrund, Logs auf stdout/journal).
# Das Token kommt aus der Umgebung (LICHESS_BOT_TOKEN) und erscheint
# weder in der Config noch in Logs.
#
# Aufruf:
#   PREFIX=/opt/funken LICHESS_BOT_TOKEN='<token>' ./start.sh
set -euo pipefail

PREFIX="${PREFIX:-/opt/funken}"
BRIDGE_DIR="$PREFIX/bridge"

if [ -z "${LICHESS_BOT_TOKEN:-}" ]; then
  echo "FEHLT: Umgebungsvariable LICHESS_BOT_TOKEN ist nicht gesetzt." >&2
  echo "Token erzeugen: https://lichess.org/account/oauth/token (Scope: bot:play)." >&2
  exit 1
fi
[ -x "$PREFIX/funken" ] || { echo "FEHLT: $PREFIX/funken (erst setup.sh ausfuehren)"; exit 1; }
[ -f "$BRIDGE_DIR/config.yml" ] || { echo "FEHLT: $BRIDGE_DIR/config.yml (erst setup.sh ausfuehren)"; exit 1; }

# Token-Wert nie ausgeben (auch nicht teilweise).
set +x
exec "$BRIDGE_DIR/venv/bin/python" "$BRIDGE_DIR/lichess-bot.py" --config "$BRIDGE_DIR/config.yml"
