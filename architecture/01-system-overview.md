# System Overview

Mega Blastoise is organized as a shared battle core plus platform adapters.

## Workspace Components

- [`mega_blastoise_core`](../mega_blastoise_core/) (`no_std`)
  - Battle runner, typed events, input trait, demo teams, and data store.
- [`mega_blastoise_fw`](../mega_blastoise_fw/) (`no_std`, RP2040)
  - Embassy runtime, USB input adapter, PN532 tasks, defmt logging, memory profiling.
- [`mega_blastoise_test`](../mega_blastoise_test/) (host/std)
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
