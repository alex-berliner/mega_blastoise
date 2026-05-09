#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$SCRIPT_DIR/../mega_blastoise_web"

cd "$WEB_DIR"

echo "Building WASM..."
wasm-pack build --target web --release

# Symlink pkg into www so the server can reach it
ln -sf ../pkg www/pkg

PORT="${1:-7890}"
echo "Serving at http://localhost:$PORT"
python3 -m http.server "$PORT" --directory www
