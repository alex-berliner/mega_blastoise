# Host Runtime and Test Harness

This page describes `mega_blastoise_test`.

## Purpose

Host runs provide fast feedback using the same shared battle logic before flashing hardware.

## Main Harness

[`src/harness.rs`](../mega_blastoise_test/src/harness.rs):

- builds a battle with `FlashDataStore`
- attaches `StdinBattleInput` (interactive choice input)
- attaches `BoardGameEffects` (human-readable event output)
- calls shared `run_battle`
- prints active-mon snapshots after each turn

## Why This Matters

Because host and firmware both call the same `run_battle` and typed event pipeline, behavioral issues can usually be reproduced and debugged on host first.

Continue with [Events and Input Contracts](./05-events-and-input.md).
