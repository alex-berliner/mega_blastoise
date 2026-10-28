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

## Web build and site

`mega_blastoise_web` compiles core to WASM. [`src/lib.rs`](../../crates/mega_blastoise_web/src/lib.rs)
is the web's `DevicePort` plus exports that turn pointer and keyboard input into `DeviceEvent`s;
[`www/device.html`](../../crates/mega_blastoise_web/www/device.html) draws a DS Lite style console with
both seats in one window and an in-page console. URL parameters such as `?demo=1`, `?ai=1` and
`?seed=N` drive it for testing.

[`.github/workflows/pages.yml`](../../.github/workflows/pages.yml) builds and deploys on every push to
`main`:

| Path on Pages | What |
|---|---|
| `/` | Front page, from [site/index.html](../../site/index.html) |
| `/battler/device.html` | The game |
| `/battler/ui_flow.html` | The single-screen UI design notes |
| `/assembly/` | Exploded 3D view of the reference design, built locally by [hardware/ref_design/viewer/](../../hardware/ref_design/viewer) and kept fresh by a pre-push hook |
| `/assembly-cell-on-board/` | The same view of the earlier cell-on-board layout |

## Host harness

`cargo run -p mega-blastoise-test` plays a battle over stdin through the same pipeline as the
device. `cargo run -p mega-blastoise-test --bin screens -- <dir>` renders every screen to PPM
files with no hardware or browser.
