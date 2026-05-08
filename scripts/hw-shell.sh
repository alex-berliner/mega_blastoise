#!/usr/bin/env bash
# Interactive hardware shell: build → flash → poll RTT + USB → send USB input, repeat.
#
# Usage: ./scripts/hw-shell.sh [--no-build] [--release]
#
# The firmware USB CDC port (VID=0xc0de PID=0xcafe) is detected automatically
# via sysfs. Override with USB_DEV=/dev/ttyACMx if needed.
#
# Shell commands (at the hw> prompt):
#   :reflash   rebuild + re-flash + reset without leaving the loop
#   :reset     reset the board without reflashing
#   :q / :quit exit
#   <anything else>  send over USB, then poll

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE=thumbv6m-none-eabi
RTT_TIMEOUT="${RTT_TIMEOUT:-2}"
USB_TIMEOUT="${USB_TIMEOUT:-2}"

FW_VID="c0de"
FW_PID="cafe"

# --- arg parsing ---

NO_BUILD=0
BUILD_ARGS=()
for arg in "$@"; do
    case "$arg" in
        --no-build) NO_BUILD=1 ;;
        *) BUILD_ARGS+=("$arg") ;;
    esac
done

PROFILE=debug
for arg in "${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}"; do
    [[ "$arg" == "--release" ]] && PROFILE=release
done

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ELF="$TARGET_DIR/$TRIPLE/$PROFILE/mega-blastoise-fw"

# --- USB device detection ---

# Find the ttyACM* whose parent USB device matches VID:PID.
# Sysfs layout: /sys/class/tty/ttyACMx/device -> USB interface;
#               ../idVendor and ../idProduct are on the USB device one level up.
_find_fw_tty() {
    for tty in /sys/class/tty/ttyACM*/device; do
        local vid pid
        vid=$(cat "$tty/../idVendor" 2>/dev/null) || continue
        pid=$(cat "$tty/../idProduct" 2>/dev/null) || continue
        if [[ "$vid" == "$FW_VID" && "$pid" == "$FW_PID" ]]; then
            basename "$(dirname "$tty")"
            return 0
        fi
    done
    return 1
}

# Wait up to $1 seconds for the firmware CDC port to enumerate, then set USB_DEV.
_detect_usb() {
    local deadline=$(( $(date +%s) + ${1:-5} ))
    while (( $(date +%s) < deadline )); do
        local name
        if name=$(_find_fw_tty 2>/dev/null); then
            USB_DEV="/dev/$name"
            echo "USB CDC: $USB_DEV (${FW_VID}:${FW_PID})"
            return 0
        fi
        sleep 0.5
    done
    # Fall back to env var or default if sysfs detection fails
    USB_DEV="${USB_DEV:-/dev/ttyACM1}"
    echo "USB CDC: not detected, using $USB_DEV"
}

# --- helpers ---

_sep() { printf '%s\n' "─────────────────────────────────────────"; }

_poll_rtt() {
    _sep
    echo "[RTT  $(date +%H:%M:%S)]"
    timeout "$RTT_TIMEOUT" probe-rs attach --preset pico --no-location --no-timestamps "$ELF" 2>/dev/null || true
}

_poll_usb() {
    _sep
    echo "[USB  $(date +%H:%M:%S)]"
    if [[ ! -e "$USB_DEV" ]]; then
        echo "(${USB_DEV} not present)"
        return
    fi
    stty -F "$USB_DEV" raw -echo -hupcl min 0 time 1 2>/dev/null || true
    timeout "$USB_TIMEOUT" cat "$USB_DEV" 2>/dev/null || true
}

_poll_both() {
    _poll_rtt
    _poll_usb
    _sep
}

_usb_send() {
    local msg="$1"
    if [[ ! -e "$USB_DEV" ]]; then
        echo "warning: ${USB_DEV} not found, cannot send" >&2
        return 1
    fi
    exec 3<>"$USB_DEV"
    stty -F /proc/self/fd/3 raw -echo -hupcl min 0 time 1 2>/dev/null
    printf '%s\n' "$msg" >&3
    exec 3>&-
}

_build() {
    echo ""
    echo "=== build ==="
    "$ROOT/scripts/fw-build.sh" "${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}"
}

_flash() {
    echo ""
    echo "=== flash ==="
    probe-rs download --preset pico "$ELF"
}

_reset() {
    echo ""
    echo "=== reset ==="
    probe-rs reset --preset pico
    echo ""
    _detect_usb 20
}

# --- main ---

if [[ "$NO_BUILD" -eq 0 ]]; then
    _build
fi

_flash
_reset

echo ""
echo "hw-shell ready. Ctrl-D or :quit to exit."
echo "  :reflash  rebuild + flash + reset"
echo "  :reset    reset board"
echo "  <text>    send over USB, then poll"
echo "  <empty>   poll RTT + USB"
echo ""

while true; do
    _poll_both

    if ! read -r -p "hw> " input 2>/dev/null; then
        echo ""
        echo "bye"
        break
    fi

    case "$input" in
        :quit|:q|:exit)
            echo "bye"
            break
            ;;
        :reflash|:flash)
            _build && _flash && _reset
            ;;
        :reset)
            _reset
            ;;
        "")
            ;;
        *)
            echo "sending: $input"
            _usb_send "$input"
            ;;
    esac
done
