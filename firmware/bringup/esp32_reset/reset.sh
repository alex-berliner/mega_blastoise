#!/usr/bin/env bash
# Send a reset trigger to the esp32_reset board.
#
#   ./reset.sh pico        reset the player Pico
#   ./reset.sh probe       reset the debug-probe Pico
#   ./reset.sh both        reset both
#
# Override the serial port with PORT=/dev/ttyUSBn (default below).
set -euo pipefail

PORT="${PORT:-/dev/ttyUSB0}"
BAUD="${BAUD:-115200}"

case "${1:-}" in
  pico)  byte='p' ;;
  probe) byte='d' ;;
  both)  byte='b' ;;
  *) echo "usage: $0 {pico|probe|both}   (PORT=$PORT)" >&2; exit 2 ;;
esac

if [[ ! -e "$PORT" ]]; then
  echo "error: $PORT not found (set PORT=/dev/ttyUSBn)" >&2
  exit 1
fi

# Raw mode so the single byte goes through untouched, no DTR reset toggle.
stty -F "$PORT" "$BAUD" raw -echo -hupcl
printf '%s' "$byte" > "$PORT"
echo "sent '$byte' -> $PORT  ($1)"
