# Host Runtime and Test Harness

This page describes `mega_blastoise_test`.

## Purpose

Host runs provide fast feedback using the same shared battle logic before flashing hardware.

## Main Harness

[`src/harness.rs`](../../crates/mega_blastoise_test/src/harness.rs):

- draws two Gen 1 teams and a `runner::ps::BattleSetup` over a `LocalTransport`
- attaches `HostBattleController` (stdin choice input, or a pre-fed queue in tests)
- attaches `HostBattleEffects` (human-readable event output)
- calls shared `runner::ps::run_battle`

## Why This Matters

Because host and firmware both call the same `run_battle` and typed event pipeline, behavioral issues can usually be reproduced and debugged on host first.

Continue with [Events and Input Contracts](./05-events-and-input.md).
