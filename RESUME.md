# Resume point — workspace restructure

## What we were doing
Restructuring the single-crate `mega_blastoise/` into a Cargo workspace with three members:
- `mega_blastoise_core` — no_std lib (FlashDataStore, battle data codegen)
- `mega_blastoise_fw` — RP2040 embassy binary (unchanged behaviour)
- `mega_blastoise_test` — PC std binary (same battle, plain fn main + println!)

## Current state (complete)
- `cargo build -p mega-blastoise-test` — passes (once_cell `race` gated by `target_has_atomic`).
- Firmware: from workspace root use `--target thumbv6m-none-eabi` (see below), or build inside `mega_blastoise_fw/`.

## Original issue (resolved)
`ahash` (via hashbrown → battler) needed `once_cell::race` on the host; the patch restores
`alloc = ["race"]` but skips compiling `race` on thumbv6m where atomics are unavailable.

## The fix (applied)

### Step 1 — revert the Cargo.toml feature change (DONE)
`patches/once_cell/Cargo.toml` line 49 has been restored to:
```toml
alloc = ["race"]
```

### Step 2 — gate the race module by target_has_atomic (DONE)
In `patches/once_cell/src/lib.rs` line 1421-1422:
```rust
// CURRENT (broken on thumbv6m):
#[cfg(feature = "race")]
pub mod race;

// CHANGE TO:
#[cfg(all(feature = "race", target_has_atomic = "ptr"))]
pub mod race;
```
This makes `race` compile on x86_64 (has atomic ptr, ahash works) but not on
thumbv6m (no atomic ptr, race module silently absent — ahash is not in the
embedded dep graph so nothing breaks there).

## Verify both targets:
```
cargo build -p mega-blastoise-test
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi   # from workspace root
```

## Then run the PC test to see actual battle output:
```
cargo run -p mega-blastoise-test
```
(as of 2026-04: also fixed type-chart parsing for emitted KV tables, embedded
`abilities/gen1.json`, and removed superseded root `src/` / `build.rs` /
`memory.x`; commits on branch `multiplat`.)

## Firmware note (workspace root)
From the workspace root, Cargo does **not** pick up `mega_blastoise_fw/.cargo/config.toml`
default target; use either:
```
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi
```
or `cd mega_blastoise_fw && cargo build`.

Superseded root files (`src/`, root `build.rs`, root `memory.x`) have been removed.
