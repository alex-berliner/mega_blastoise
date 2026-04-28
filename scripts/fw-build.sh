#!/usr/bin/env bash
# Build Pico firmware and print flash/RAM usage (llvm-size / arm-none-eabi-size).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TRIPLE=thumbv6m-none-eabi
PROFILE="${PROFILE:-debug}"
for arg in "$@"; do
  [[ "$arg" == "--release" ]] && PROFILE=release && break
done
ELF_NAME=mega-blastoise-fw

cargo build -p mega-blastoise-fw --target "$TRIPLE" "$@"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ELF="$TARGET_DIR/$TRIPLE/$PROFILE/$ELF_NAME"

if [[ ! -f "$ELF" ]]; then
  echo "error: expected ELF at $ELF" >&2
  exit 1
fi

SYSROOT=$(rustc --print sysroot)
HOST=$(rustc -vV | awk '/^host:/{print $2}')
LLVM_SIZE="$SYSROOT/lib/rustlib/$HOST/bin/llvm-size"

echo ""
echo "=== $ELF_NAME ($PROFILE / $TRIPLE) ==="

if [[ -x "$LLVM_SIZE" ]]; then
  "$LLVM_SIZE" -t "$ELF"
  echo ""
  "$LLVM_SIZE" -A "$ELF" | head -50
elif command -v arm-none-eabi-size >/dev/null 2>&1; then
  arm-none-eabi-size -A "$ELF"
else
  echo "Install size tooling for stats:"
  echo "  rustup component add llvm-tools-preview"
  echo "  # llvm-size is then at: $LLVM_SIZE"
  exit 0
fi

echo ""
echo "Tip: dev profiles embed DWARF; use --release for totals closer to flash-programmed size."
