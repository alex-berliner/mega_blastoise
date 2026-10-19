#!/usr/bin/env bash
# Copy the UF2 to an RP2350 sitting in BOOTSEL.
#
# Refuses to write to an RP2040 in BOOTSEL. Both boards mount a drive called
# RPI-RP2 and both accept a drag-and-drop UF2, so the only safe discriminator
# is the USB product id: 2e8a:000f is RP2350, 2e8a:0003 is RP2040. Flashing an
# Arm-v8 image onto an RP2040 is recoverable, but it costs a confusing hour.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
UF2=${1:-rp2350-blink.uf2}
[[ -f $UF2 ]] || { echo "no $UF2 — run tools/uf2.sh first" >&2; exit 1; }

if lsusb -d 2e8a:0003 >/dev/null 2>&1; then
    echo "An RP2040 is in BOOTSEL (2e8a:0003). Unplug it before flashing the RP2350." >&2
    exit 1
fi
lsusb -d 2e8a:000f >/dev/null 2>&1 || {
    echo "No RP2350 in BOOTSEL (looking for 2e8a:000f)." >&2
    echo "Hold BOOTSEL while plugging the board in, then re-run." >&2
    exit 1
}

MOUNT=$(lsblk -o LABEL,MOUNTPOINT -nr | awk '$1 == "RP2350" || $1 == "RPI-RP2" { print $2; exit }')
if [[ -z ${MOUNT:-} ]]; then
    DEV=$(lsblk -o NAME,LABEL -nr | awk '$2 == "RP2350" || $2 == "RPI-RP2" { print "/dev/"$1; exit }')
    [[ -n ${DEV:-} ]] || { echo "RP2350 is in BOOTSEL but no drive appeared." >&2; exit 1; }
    MOUNT=$(udisksctl mount -b "$DEV" | sed -n 's/.* at \(.*\)\.$/\1/p')
fi

echo "flashing $UF2 -> $MOUNT"
cp "$UF2" "$MOUNT/"
sync
echo "done — the board reboots on its own and the drive disappears"
