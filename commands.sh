# . $MB_BASE/commands.sh  (MB_BASE = workspace root)

ELF=$MB_BASE/target/thumbv6m-none-eabi/debug/mega-blastoise-fw

alias mb_cd='cd $MB_BASE/mega_blastoise_fw'

# Build fw (embedded target via .cargo/config.toml) AND verify the test crate compiles for host.
# Pass extra cargo flags (e.g. --release) and they apply to the fw build only.
function mb_build {
    echo "=== fw ===" &&
    (cd "$MB_BASE/mega_blastoise_fw" && cargo build "$@") &&
    echo "=== test ===" &&
    (cd "$MB_BASE" && cargo check -p mega-blastoise-test)
}

# Run host-side tests (does not touch the fw target).
function mb_test {
    (cd "$MB_BASE" && cargo test -p mega-blastoise-test "$@")
}

alias mb_download='mb_cd && probe-rs download --preset pico "$ELF"'
alias mb_reset='mb_cd && probe-rs reset --preset pico'
alias mb_kill='pkill -9 -f "probe-rs" || true; pkill -9 -f "picocom" || true'
alias mb_rttpoll='timeout 2 probe-rs attach --preset pico "$ELF"'
alias mb_usb_init='stty -F /dev/ttyACM1 raw -echo -hupcl min 0 time 1'
alias mb_usbpoll='mb_usb_init && timeout 2 cat /dev/ttyACM1'
function mb_usb_send
{
    # Open once so HUPCL doesn't reset termios between stty and write
    exec 3<>/dev/ttyACM1
    stty -F /proc/self/fd/3 raw -echo -hupcl min 0 time 1
    printf '%s\n' "$@" >&3
    exec 3>&-
}
