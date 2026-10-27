# Mega Blastoise — Technical Reference

Software internals for the Mega Blastoise firmware and host test harness.

For project overview and hardware, see [DESIGN.md](./archive/v1/DESIGN.md) (the v1 device).

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

Battles run on `ps_sim`, a `no_std` port of the Pokémon Showdown simulator, read by `ps_client` (see `docs/architecture/showdown-split-plan.md`). Both generations play on it; the legacy per-generation engines are gone (G1-10 took Gen 1's, G3-9 Gen 3's). The firmware is purely presentation and input.

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

See [architecture/05-events-and-input.md](./architecture/05-events-and-input.md) for the full event contract.

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

For build scripts and size stats, see `scripts/fw-build.sh`. Full architecture docs are in [architecture/](./architecture/README.md).
