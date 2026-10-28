# Mega Blastoise — Technical Reference

Software internals for the Mega Blastoise firmware and host test harness.

For what the product is, see [DESIGN.md](../static/DESIGN.md); the v1 device's overview is [archive/v1/DESIGN.md](../archive/v1/DESIGN.md).

---

## Crate Layout

```
mega_blastoise/
  crates/ps_sim/                   # no_std port of the Pokemon Showdown server (Gen 1 and Gen 3)
  crates/ps_client/                # no_std client: reads the PS protocol into board events
  crates/ps_dex/, crates/ps_protocol/  # the dex tables and the protocol types
  crates/mega_blastoise_core/      # no_std shared library: engine glue, event bus
  crates/mega_blastoise_fw/        # RP2350 (Pico 2) firmware for the stripboard v2 board
  crates/mega_blastoise_test/      # PC host binary for development and testing
```

`mega_blastoise_core` compiles for both the RP2350 and a standard PC. The full battle pipeline (engine integration, event parsing, input handling) can be developed and tested on a laptop without touching hardware. `mega_blastoise_fw` and `mega_blastoise_test` are thin adapters that plug platform-specific I/O into the same shared core.

---

## Battle Engine

Battles run on `ps_sim`, a `no_std` port of the Pokémon Showdown simulator, read by `ps_client` (see `docs/living/showdown-split-plan.md`). Both generations play on it; the legacy per-generation engines are gone (G1-10 took Gen 1's, G3-9 Gen 3's). The firmware is purely presentation and input.

Both compile directly for the RP2350.

---

## Data Storage

Pokémon stats, move definitions and type charts are baked into flash at compile time: `ps_dex` is generated from Showdown's own data. There is no SD card or file system.

---

## Concurrency Model

The firmware is written in Rust with `no_std` (no OS, no standard library). The async runtime is [Embassy](https://embassy.dev/), providing cooperative multitasking via `async/await` on a single CPU core.

Two main futures run cooperatively:

- **Battle loop** — steps the engine, emits typed board events, blocks waiting for player input
- **Input handler** — watches for button presses (or USB serial during development), sends choices back to the battle loop

A separate Embassy task handles USB protocol frames.

All shared state crosses future/task boundaries through Embassy's channel and signal primitives (`Channel`, `Signal` with `NoopRawMutex`) — no mutexes, no unsafe shared memory.

---

## Event Pipeline

When the battle engine resolves a turn it emits a log of what happened. The firmware parses those log lines into typed `BoardEvent` values — `Damage`, `Faint`, `SwitchIn`, `Win`, etc. — and dispatches them to the hardware layer, which drives the LEDs and display accordingly.

See [architecture/05-events-and-input.md](architecture/05-events-and-input.md) for the full event contract.

---

## Development Setup

Requires Rust nightly (pinned in `rust-toolchain.toml`) and `probe-rs` for flashing.

```bash
# Play a battle on your PC (no hardware needed)
cargo run -p mega-blastoise-test

# Run tests
cargo test -p mega-blastoise-test

# Build firmware
cd crates/mega_blastoise_fw
cargo build --release

# Flash and run on hardware
cargo run --release
```

For build scripts and size stats, see `scripts/fw-build.sh`. Full architecture docs are in [architecture/](architecture/README.md).

## Verification: software

- **The gate:** [scripts/check.sh](../../scripts/check.sh) runs before every code commit: all host
  tests, a firmware `cargo check` for its real target, the WASM build, and the headless UI probe
  (`--fast` skips the last two). It exists because two test binaries once rotted silently.
- **Showdown parity:** tools/showdown-parity/ runs the real
  Showdown simulator under Node. `showdown_parity` compares hand-written scenarios and
  `showdown_fuzz` compares seeded random ones (`FUZZ_N`, `FUZZ_SEED`, `FUZZ_ONLY`), with hit,
  crit and damage roll scripted on both sides so every branch is compared. Unarmed checkouts skip
  these suites loudly. Engine changes get a 2000-seed sweep before they are pushed.
- **Headless UI probe:** [scripts/ui-probe.sh](../../scripts/ui-probe.sh) boots the web page in headless
  Chromium on a pinned seed, plays real turns in both generations through the same exports the
  touch controls use, and asserts on a per-seat state trace. It catches wiring bugs that unit
  tests cannot see.
- **Screens:** the `screens` binary renders every UI state to image files on a host.
- **Device debugging:** [scripts/mb-console.py](../../scripts/mb-console.py) merges RTT and USB console
  output and forwards typed lines; [scripts/fw-build.sh](../../scripts/fw-build.sh) reports flash and RAM
  use; `scripts/commands.sh` defines shell helpers; `.claude/commands/fwdebug.md` is the firmware
  debug loop for Claude Code.

## Working practices

- Generated files are never hand-edited: the stripboard from `layout.py`, the reference design
  from `design.py`, `placement.py` and `case.scad`, the pin map from `pinmap.py`.
- For engine work, read Showdown's source first, then fuzz to confirm.
- For case work, render and assert headless before printing.
