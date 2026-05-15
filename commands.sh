# mega-blastoise dev commands
# Source with:  . $MB_BASE/commands.sh   (MB_BASE = workspace root)

# Unset previous definitions so re-sourcing always picks up the latest versions.
# NOTE FOR CLAUDE: every name defined anywhere in this file (functions, aliases,
# variables) MUST have a matching compgen unset line here.
unset -f $(compgen -A function mb_)  2>/dev/null
unalias  $(compgen -a          mb_)  2>/dev/null
unset    $(compgen -v          _MB_) 2>/dev/null

_MB_FW_DIR="$MB_BASE/mega_blastoise_fw"
_MB_ELF_DEBUG="$MB_BASE/target/thumbv6m-none-eabi/debug/mega-blastoise-fw"
_MB_ELF_RELEASE="$MB_BASE/target/thumbv6m-none-eabi/release/mega-blastoise-fw"
_MB_CONSOLE="$MB_BASE/scripts/mb-console.py"

# ── Build ────────────────────────────────────────────────────────────────────

# Build firmware (embedded target) and verify the test crate compiles for host.
# Extra args are forwarded to `cargo build` for the fw only (e.g. --release).
function mb_build {
    echo "=== fw ===" &&
    (cd "$_MB_FW_DIR" && cargo build "$@") &&
    echo "=== test ===" &&
    (cd "$MB_BASE" && cargo check -p mega-blastoise-test)
}

# Run host-side tests (no hardware required).
function mb_test {
    (cd "$MB_BASE" && cargo test -p mega-blastoise-test "$@")
}

# ── Flash / reset ────────────────────────────────────────────────────────────

# Flash debug ELF.  Pass --release for the release build.
function mb_flash {
    local elf="$_MB_ELF_DEBUG"
    for arg in "$@"; do [[ "$arg" == "--release" ]] && elf="$_MB_ELF_RELEASE"; done
    (cd "$MB_BASE" && probe-rs download --preset pico "$elf")
}

function mb_reset { (cd "$MB_BASE" && probe-rs reset --preset pico); }
function mb_kill  { (cd "$MB_BASE" && pkill -f probe-rs || true); }

# ── Console ──────────────────────────────────────────────────────────────────

# Stream RTT + USB output; forward keyboard input to USB.
# Flags are passed through to mb-console.py (--no-rtt, --no-usb, --log FILE, etc.)
function mb_console {
    (cd "$MB_BASE" && python3 "$_MB_CONSOLE" "$@")
}

# ── Combined workflows ───────────────────────────────────────────────────────

# Flash + reset + open console (board must already be connected).
function mb_run {
    mb_flash "$@" && mb_reset && mb_console
}

# Full cycle: build → flash → reset → console.
function mb_dev {
    mb_build "$@" && mb_run "$@"
}
