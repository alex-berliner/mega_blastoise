# Core Battle Engine Flow

This page describes shared logic inside `mega_blastoise_core`.

## Primary Modules

- [`runner/`](../../../crates/mega_blastoise_core/src/runner/mod.rs) (the PS engine, every
  battle in both generations)
  - `run_battle` is the main orchestration loop.
  - setup: `ps::BattleSetup::local`.
- [`board_event.rs`](../../../crates/mega_blastoise_core/src/board_event.rs)
  - the `BoardEvent` model (re-exported from `ps_client`) and its helpers.
- [`battle_effects.rs`](../../../crates/mega_blastoise_core/src/battle_effects.rs)
  - defines `BoardEffects`.
  - provides `BoardEventQueue` and split/private/public handling.
- [`battle_input.rs`](../../../crates/mega_blastoise_core/src/battle_input.rs)
  - defines async `BattleInput::read_choice`.
  - formats move/switch command strings.

Battle data comes from `ps_dex` (generated from Showdown's own data); there is
no data store object.

## `runner::ps::run_battle` Loop Behavior

1. Send `>start` and the two `>player` commands.
2. Poll the transport: log lines go through the `ps_client` view to the effect
   sink as `BoardEvent`s; requests are collected.
3. Distil every request that asks for a choice into `SlotOptions` and send all
   the prompts on the `InputBus` before taking any answer, so both players
   choose at once.
4. Take one answer per prompted seat from `bus.choices`, translate it to a PS
   choice and send it.
5. Repeat until the server's `end`.

Continue with [Firmware Runtime (RP2350)](03-firmware-runtime.md).

## How a turn flows

1. The engine writes battle log lines; core parses them into typed `BoardEvent`s (damage, faint,
   switch-in, status and so on) and turns the engine's request into a prompt.
2. `OledController` decides each seat's screen from those events; `DeviceRunner` paces narration.
3. The renderer draws each half from the `Screen` plus per-seat context (foe HUD, cursor) and
   composes the 320x480 frame. The platform's `present` hook only moves pixels: the firmware
   sends the 16x16 tiles that changed; the web page blits the frame to a canvas.
4. A press comes in from the platform as a classified `DeviceEvent`. `DeviceSession` and
   `CursorNav` turn it into a pad event; `ChoiceCollector` validates it and, once both seats are
   ready and the grace window has passed, hands the engine its command strings.
