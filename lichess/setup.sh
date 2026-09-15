#!/usr/bin/env bash
# Funken: Installation auf einem Linux-Rechner ohne GUI.
# Baut die Engine, installiert sie nach $PREFIX (Default /opt/funken),
# richtet die offizielle lichess-bot-Bridge in einem Python-venv ein und
# legt eine lauffaehige Bridge-Konfiguration an (ohne Geheimnisse).
#
# Aufruf:
#   sudo PREFIX=/opt/funken ./setup.sh        # systemweit (empfohlen)
#        PREFIX=$HOME/funken ./setup.sh       # ohne root, nur Engine-Test
#
# Das Bot-Token gehoert NICHT in diese Datei und NICHT ins Repo.
# Es wird zur Laufzeit als Umgebungsvariable LICHESS_BOT_TOKEN gelesen
# (siehe start.sh / funken.service).
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${PREFIX:-/opt/funken}"
BRIDGE_DIR="$PREFIX/bridge"
BRIDGE_REPO="https://github.com/lichess-bot-devs/lichess-bot.git"

echo "==> [1/5] Werkzeuge pruefen"
command -v cargo >/dev/null || { echo "FEHLT: cargo (Rust)"; exit 1; }
command -v python3 >/dev/null || { echo "FEHLT: python3"; exit 1; }
command -v git >/dev/null || { echo "FEHLT: git"; exit 1; }
python3 -c "import sys; assert sys.version_info >= (3,10), 'Python >= 3.10 noetig'"

echo "==> [2/5] Engine bauen (release)"
cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"

echo "==> [3/5] Engine installieren nach $PREFIX"
mkdir -p "$PREFIX"
install -m 0755 "$REPO_DIR/target/release/funken" "$PREFIX/funken"
"$PREFIX/funken" perft 3 | tail -n 1

echo "==> [4/5] Bridge einrichten in $BRIDGE_DIR"
if [ -d "$BRIDGE_DIR/.git" ]; then
  git -C "$BRIDGE_DIR" pull --ff-only
else
  git clone --depth 1 "$BRIDGE_REPO" "$BRIDGE_DIR"
fi
[ -x "$BRIDGE_DIR/venv/bin/python" ] || python3 -m venv "$BRIDGE_DIR/venv"
"$BRIDGE_DIR/venv/bin/pip" install -q -r "$BRIDGE_DIR/requirements.txt"

echo "==> [5/5] Bridge-Konfiguration anlegen (falls fehlend)"
if [ ! -f "$BRIDGE_DIR/config.yml" ]; then
  sed -e "s#dir: \"/opt/funken/\"#dir: \"$PREFIX/\"#" \
      "$REPO_DIR/lichess/config.yml.example" > "$BRIDGE_DIR/config.yml"
  chmod 600 "$BRIDGE_DIR/config.yml"
  echo "    config.yml angelegt (Token noch als Platzhalter!)."
else
  echo "    config.yml existiert bereits, wird nicht ueberschrieben."
fi

echo
echo "Fertig. Naechste Schritte (siehe README.md, Abschnitt Go-Live):"
echo "  1. Lichess-Account anlegen (noch KEINE Partie spielen) + Token mit Scope 'bot:play' erzeugen."
echo "  2. Token als Umgebungsvariable bereitstellen, z.B.:"
echo "       export LICHESS_BOT_TOKEN='<token>'   # nur fuer diese Shell, nie committen"
echo "     oder fuer systemd: /etc/funken/token.env mit LICHESS_BOT_TOKEN=<token> (chmod 600)."
echo "  3. Testlauf (casual, Token aus Env):"
echo "       LICHESS_BOT_TOKEN='<token>' $BRIDGE_DIR/venv/bin/python $BRIDGE_DIR/lichess-bot.py --config $BRIDGE_DIR/config.yml"
echo "  4. BOT-Upgrade erst nach ausdruecklicher Freigabe (irreversibel!)."
