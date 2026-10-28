# Firmware Runtime (RP2350)

This page describes `mega_blastoise_fw`, the firmware for the stripboard v2
board: a Pico 2 (RP2350), one ST7796 320x480 panel and 16 direct-GPIO buttons.
A plain `cargo build` in the crate builds it (default features `rp2350`,
`board_v2`; default target `thumbv8m.main-none-eabihf`).

## Startup Sequence

[`main.rs`](../../../crates/mega_blastoise_fw/src/main.rs) does the following:

1. Initialize the RP2350 peripherals via `embassy_rp::init`.
2. Initialize the heap (`mem_profile::init_heap`, 384 KB: one 320x480 frame in flight).
3. Configure the RTT/defmt channel.
4. Spawn the ST7796 display task, the button task, the sprite-bob pulse task,
   the USB CDC console and the battery task.
5. Hand over to [`board_v2::run`](../../../crates/mega_blastoise_fw/src/board_v2.rs),
   which runs core's `DeviceRunner` forever.

## Long-Lived Tasks

- `st7796::task`: blits each presented frame to the panel (PIO SPI + DMA).
- `buttons_v2::task`: debounces the 16 keys, tells an A tap from an A hold,
  and pushes `DeviceEvent::Button` into the runner's input channel.
- `board_v2::pulse_task`: the sprite-bob heartbeat and repaint tick.
- `usb_v2`: the CDC console (typed lines in, log lines out).
- `battery::task`: samples the cell voltage for the status bar.

## Firmware Adapters

- [`board_v2.rs`](../../../crates/mega_blastoise_fw/src/board_v2.rs)
  - implements core's `DevicePort`: clock, timers, console lines, frame
    presentation, entropy and battery level.
  - everything the device does (lobby, ready sequence, battles, narration) is
    `mega_blastoise_core::device_runner`, shared with the web build.

The v1 firmware (two OLEDs, LED strips, buzzer, button matrix) is described in
[docs/archive/v1/firmware/](../../archive/v1/firmware/INTERACTIONS.md).

Continue with [Host Runtime and Test Harness](04-host-runtime.md).

## Firmware

`mega_blastoise_fw` runs on the Pico 2 in the stripboard v2 board. A plain `cargo build` in the
crate builds the device; [03-firmware-runtime.md](03-firmware-runtime.md) has
the startup and task list. In short: Embassy async tasks for the ST7796 panel (PIO SPI with DMA),
the 16 buttons (debounce, A tap versus hold), the USB CDC console, battery sensing and the sprite
bob, and [`board_v2.rs`](../../../crates/mega_blastoise_fw/src/board_v2.rs) implements core's
`DevicePort` and runs `DeviceRunner` forever. A panic lights the Pico's GP25 LED and its message
survives a reset. Features: `input-debug` (held-key overlay), `mem-profile`, `trace`.

There is no firmware yet for the reference design: it needs the `rp235xb` chip feature, its pin
map, an ILI9488 panel driver (18-bit colour over SPI, unlike the ST7796), audio and power control.
