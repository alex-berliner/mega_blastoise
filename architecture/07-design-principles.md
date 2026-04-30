# Design Principles and Extension Guide

## Principles

- Keep battle semantics in [`mega_blastoise_core`](../mega_blastoise_core/).
- Keep platform-specific I/O in [`mega_blastoise_fw`](../mega_blastoise_fw/) and [`mega_blastoise_test`](../mega_blastoise_test/).
- Use typed `BoardEvent` as the boundary, not raw log strings.
- Keep host and firmware paths behaviorally aligned by sharing `run_battle`.

## Adding New Board Behavior

1. Add/extend event parsing in `board_event.rs`.
2. Add handling in platform `BoardEffects` implementations.
3. Add tests in `mega_blastoise_test` for queue/event semantics.
   - Core parser: [`board_event.rs`](../mega_blastoise_core/src/board_event.rs)
   - Queue/effects trait: [`battle_effects.rs`](../mega_blastoise_core/src/battle_effects.rs)
   - Test examples: [`board_events_and_queue.rs`](../mega_blastoise_test/tests/board_events_and_queue.rs)

## Adding New Input Modality

1. Implement `BattleInput` for the new transport/device.
2. Keep command formatting compatible with battler.
3. Add traces for request shape and accepted choices.
   - Input contract: [`battle_input.rs`](../mega_blastoise_core/src/battle_input.rs)
   - Firmware adapter example: [`usb_input.rs`](../mega_blastoise_fw/src/usb_input.rs)

## Scaling and Reliability Notes

- Minimize temporary allocations in request/prompt paths.
- Prefer deterministic, typed flow over ad-hoc string handling.
- Keep diagnostics close to runtime boundaries (input/effects/memory).

Back to [Architecture Index](./README.md).
