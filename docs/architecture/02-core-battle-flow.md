# Core Battle Engine Flow

This page describes shared logic inside `mega_blastoise_core`.

## Primary Modules

- [`runner/`](../../crates/mega_blastoise_core/src/runner/mod.rs) (one runner per engine: `gen1.rs`, `gen3.rs`)
  - `run_battle` is the main orchestration loop.
  - setup helpers: `demo_battle_options`, `demo_engine_opts`, `make_player`.
- [`board_event.rs`](../../crates/mega_blastoise_core/src/board_event.rs)
  - parses battler log rows into `BoardEvent`.
  - maps battler `Request` into prompt events.
- [`battle_effects.rs`](../../crates/mega_blastoise_core/src/battle_effects.rs)
  - defines `BoardEffects`.
  - provides `BoardEventQueue` and split/private/public handling.
- [`battle_input.rs`](../../crates/mega_blastoise_core/src/battle_input.rs)
  - defines async `BattleInput::read_choice`.
  - formats move/switch command strings.
- [`data_store.rs`](../../crates/mega_blastoise_core/src/data_store.rs)
  - `FlashDataStore` implementation for battler data lookups.

## `run_battle` Loop Behavior

1. Pull new battler log lines and dispatch parsed events.
2. While battle is active:
   - query active requests
   - for each request:
     - enqueue prompt event
     - dispatch prompt event to effect sink
     - await `BattleInput::read_choice`
     - apply choice with `set_player_choice`
3. Pull and dispatch new log lines.
4. Run optional per-turn callback.

## Why Requests Are Processed One-at-a-Time

The loop intentionally avoids collecting all active requests into a temporary `Vec`. This lowers peak allocation pressure in constrained firmware runs.

Continue with [Firmware Runtime (RP2350)](./03-firmware-runtime.md).
