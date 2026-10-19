# esp32_reset

A tiny hardware recovery line. When an RP2040 in the rig wedges so hard that
nothing software-side can reach it, this ESP32 yanks its `RUN` pin low for
~100 ms to force a clean power-on reset.

Targets: the player **Pico** and the **debug probe** (a Pico running
debugprobe). Triggered by a magic byte over USB.

Standalone crate — **not** in the root workspace (different toolchain +
target). Build it from inside this directory.

## Hardware

DOIT ESP32 DevKit V1 (original ESP32 / WROOM-32, Xtensa). The trigger
arrives over the onboard CP2102 USB↔UART bridge on UART0 — the original
ESP32 has no native USB.

```text
  ESP32 GPIO25 ───────────────► Pico   RUN  (Pico pin 30)
  ESP32 GPIO26 ───────────────► Probe  RUN  (probe Pico pin 30)
  ESP32 GND    ───────────────► common GND   (REQUIRED)
```

- Lines are **open-drain**: ESP32 only ever pulls `RUN` low or floats it
  (Hi-Z). It never drives high. If the ESP32 is unplugged, crashed, or in
  its bootloader, every line is Hi-Z and each RP2040's internal ~50 kΩ
  `RUN` pull-up keeps it running. The fail-safe is structural — no MOSFET,
  no external pull-up, no firmware needed for the safe state.
- A 220 Ω–1 kΩ series resistor per line is an optional nice-to-have for
  protection; not required.
- GPIO25/26 chosen because they're Hi-Z input at power-on, not strapping
  pins, and don't glitch at boot — the ESP32's own power-up can't trip a
  reset.
- **Common ground is mandatory.** Without it, none of this works.

> Bridging instead of separate pins also works (one line to both `RUN`s,
> resets together) — but two pins gives independent reset and avoids one
> powered target weakly back-feeding an unpowered one's `RUN`.

## Protocol

Magic bytes on UART0 @ 115200 (the same port you flash/monitor):

| byte | action                |
|------|-----------------------|
| `p`  | reset the Pico        |
| `d`  | reset the debug probe |
| `b`  | reset both            |

Any other byte is ignored, so stray console noise is harmless. Each
accepted command echoes a one-line ack (`RST pico` / `RST probe` /
`RST both`).

## Build & flash

One-time Xtensa toolchain setup (the original ESP32 isn't supported by
upstream Rust):

```sh
cargo install espup espflash
espup install
source ~/export-esp.sh        # add to your shell rc; needed every build shell
```

Then, from this directory:

```sh
cargo run --release           # builds, flashes, opens the monitor
```

`cargo run` uses `espflash flash --monitor` (see `.cargo/config.toml`).
Plain build only: `cargo build --release`.

> esp-hal's API moves fast between releases. Versions are pinned in
> `Cargo.toml` against esp-hal 0.23.x; if a future `espup` toolchain pulls
> incompatible crates, bump the pins together and adjust the few HAL calls
> in `src/main.rs` (init / GPIO / UART).

## Triggering from the host

```sh
./reset.sh pico            # or: probe | both
PORT=/dev/ttyUSB1 ./reset.sh both
```

Or by hand: `printf 'p' > /dev/ttyUSB0` (after `stty -F /dev/ttyUSB0
115200 raw`).

Note: opening the port may toggle DTR/RTS and reset the *ESP32* itself
(not the targets) — harmless, it just re-prints the ready banner.
`reset.sh` uses raw mode to minimize this.
