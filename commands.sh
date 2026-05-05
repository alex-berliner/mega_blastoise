# . $MB_BASE/mega_blastoise/commands.sh

ELF=$MB_BASE/mega_blastoise/target/thumbv6m-none-eabi/debug/mega-blastoise-fw

alias mb_cd='cd $MB_BASE/mega_blastoise/mega_blastoise_fw'
alias mb_build='mb_cd && cargo build'
alias mb_download='mb_cd && probe-rs download --preset pico "$ELF"'
alias mb_reset='mb_cd && probe-rs reset --preset pico'
alias mb_kill='mb_cd && pkill -9 -f "probe-rs"; pkill -9 -f "picocom"'
alias mb_rttpoll='mb_cd && timeout 0.5 probe-rs attach --preset pico "$ELF"'
alias mb_usbpoll='timeout 0.5 cat /dev/ttyACM1'
function mb_usb_send
{
    echo -ne "$@\n" > /dev/ttyACM1
}

