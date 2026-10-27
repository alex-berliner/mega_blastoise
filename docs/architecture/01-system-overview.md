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

1. Draw two teams (`randbat::draw_two_ps_teams`) and a `runner::ps::BattleSetup`.
2. Enter `runner::ps::run_battle` over a `Transport` (`LocalTransport` on the device).
3. Send `>start` and both `>player` commands to `ps_sim`.
4. Read the PS protocol back through `ps_client` into typed `BoardEvent`s.
5. Distil each `|request|` into `SlotOptions` and put the prompts on the `InputBus`.
6. Read each player's choice back from the bus (the `ChoiceCollector` decides it).
7. Send it as `>p1` / `>p2` and continue until the server says the battle ended.

(Both generations run the same loop, on the PS engine: the legacy Gen 1
and Gen 3 engines are gone.)

Continue with [Core Battle Engine Flow](./02-core-battle-flow.md).
