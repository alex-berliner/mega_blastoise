# Mega Blastoise — Technical Reference

Software internals for the Mega Blastoise firmware and host test harness.

For project overview and hardware, see [DESIGN.md](./DESIGN.md).

---

## Crate Layout

```
battler/                    # upstream battle engine (git checkout)
mega_blastoise/
  mega_blastoise_core/      # no_std shared library: engine glue, event bus
  mega_blastoise_fw/        # RP2040 firmware binary
  mega_blastoise_test/      # PC host binary for development and testing
```

`mega_blastoise_core` compiles for both the RP2040 and a standard PC. The full battle pipeline — engine integration, event parsing, input handling — can be developed and tested on a laptop without touching hardware. `mega_blastoise_fw` and `mega_blastoise_test` are thin adapters that plug platform-specific I/O into the same shared core.

---

## Battle Engine

All game logic is handled by [`battler`](https://github.com/jackson-nestelroad/battler), an open-source Pokémon battle engine written in Rust. This project does not reimplement damage formulas, type effectiveness, status effects, or turn resolution — `battler` handles all of that. The firmware is purely presentation and input.

`battler`'s core crates are `no_std`-compatible and compile directly for the RP2040.

---

## Data Storage

Pokémon stats, move definitions, and the Gen 1 type chart are baked into flash at compile time. A build script reads the `battler` JSON data files and generates static Rust tables. There is no SD card or file system.

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
cd mega_blastoise_fw
cargo build --release

# Flash and run on hardware
cargo run --release
```

For build scripts and size stats, see `scripts/fw-build.sh`. Full architecture docs are in [architecture/](./architecture/README.md).
