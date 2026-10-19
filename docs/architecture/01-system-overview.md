# System Overview

Mega Blastoise is organized as a shared battle core plus platform adapters.

## Workspace Components

- [`mega_blastoise_core`](../../crates/mega_blastoise_core/) (`no_std`)
  - Battle runner, typed events, input trait, demo teams, and data store.
- [`mega_blastoise_fw`](../../crates/mega_blastoise_fw/) (`no_std`, RP2350 / Pico 2)
  - Embassy runtime, ST7796 panel, 16 direct buttons, USB CDC console, battery sense, defmt logging.
- [`mega_blastoise_web`](../../crates/mega_blastoise_web/) (wasm)
  - The same device in a browser page (`www/device.html`).
- [`mega_blastoise_test`](../../crates/mega_blastoise_test/) (host/std)
  - Stdin-based interactive harness and tests reusing the same core pipeline.

## Architectural Goal

One battle/event model should drive all runtimes. Platform-specific code should only handle I/O and effects.

## End-to-End Battle Lifecycle

1. Build a `PublicCoreBattle` with `FlashDataStore`.
2. Load teams and call `battle.start()`.
3. Enter `run_battle`.
4. Parse engine logs into typed `BoardEvent`.
5. Emit prompt events for pending requests.
6. Read player choice through `BattleInput`.
7. Submit choice to battler and continue until `battle.ended()`.

Continue with [Core Battle Engine Flow](./02-core-battle-flow.md).
