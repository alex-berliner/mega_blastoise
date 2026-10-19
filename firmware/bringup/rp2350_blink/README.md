# rp2350-blink

Bring-up smoke test for the RP2350. Not part of the game firmware, and kept as
its own crate: it was written while the parent workspace still targeted the
RP2040 and pinned `embassy-rp` to its `rp2040` feature.

## What it proves

* **GP25 blinks at 2 Hz.** That is the on-board LED on a Pico 2. On a Pico 2 W
  the LED hangs off the wireless chip, so watch the serial output instead.
* **A USB CDC port prints `rp2350 alive, tick N` every second.** Enumerating at
  all exercises the crystal, the PLLs and the USB block, which a blinking pin
  does not.

## Flashing

With a CMSIS-DAP probe on SWD (what this bench has):

    cargo run --release

or, without the runner:

    cargo build --release
    probe-rs download --chip RP235x --binary-format elf \
        target/thumbv8m.main-none-eabihf/release/rp2350-blink
    probe-rs reset --chip RP235x

With no probe, over BOOTSEL:

    tools/uf2.sh          # build + emit rp2350-blink.uf2
    tools/flash.sh        # copy it to a board in BOOTSEL

`tools/uf2.sh` exists because `elf2uf2-rs` 2.2.0 is RP2040-only: it lays out the
blocks correctly but stamps family id `0xe48bff56` (RP2040), which an RP2350
rejects. The script restamps every block to `0xe48bff59` (rp2350-arm-s) after
checking its magic. `picotool` would do this in one step but is not packaged for
this distro.

`tools/flash.sh` refuses to write while an RP2040 is in BOOTSEL: both chips
mount a drive named RPI-RP2 and both accept a dropped UF2, so the USB product id
(`2e8a:000f` RP2350, `2e8a:0003` RP2040) is the only safe discriminator.

## Reading the serial output

The port blocks on open waiting for carrier, so `cat /dev/ttyACM0` hangs. Open
it non-blocking and set `CLOCAL`:

    python3 - <<'PY'
    import os, select, termios, time
    fd = os.open('/dev/ttyACM0', os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    a = termios.tcgetattr(fd); a[2] |= termios.CLOCAL | termios.CREAD
    a[3] &= ~(termios.ICANON | termios.ECHO | termios.ISIG); a[0] = a[1] = 0
    termios.tcsetattr(fd, termios.TCSANOW, a)
    end = time.time() + 8
    while time.time() < end:
        if select.select([fd], [], [], 0.5)[0]:
            print(os.read(fd, 256).decode(errors='replace'), end='')
    PY

Pick the port by vendor id, not by number: the Debugprobe enumerates its own
`/dev/ttyACM*` as well. The device is `c0de:cafe`.
