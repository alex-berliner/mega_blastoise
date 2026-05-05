# . $MB_BASE/mega_blastoise/commands.sh

# cat $1

ELF=$MB_BASE/mega_blastoise/target/thumbv6m-none-eabi/debug/mega-blastoise-fw

alias mb_cd='cd $MB_BASE/mega_blastoise/mega_blastoise_fw'
alias mb_build='mb_cd && cargo build'
alias mb_download='mb_cd && probe-rs download --preset pico "$ELF"'
alias mb_reset='mb_cd && probe-rs reset --preset pico'
alias mb_kill='mb_cd && pkill -9 -f "probe-rs" || true; pkill -9 -f "picocom"  || true'
alias mb_rttpoll='mb_cd && timeout 0.5 probe-rs attach --preset pico "$ELF"'
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

