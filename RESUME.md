# Resume point — workspace restructure

## What we were doing
Restructuring the single-crate `mega_blastoise/` into a Cargo workspace with three members:
- `mega_blastoise_core` — no_std lib (FlashDataStore, battle data codegen)
- `mega_blastoise_fw` — RP2040 embassy binary (unchanged behaviour)
- `mega_blastoise_test` — PC std binary (same battle, plain fn main + println!)

## Current state
All files have been written. The firmware target builds clean:
```
cargo build -p mega-blastoise-fw   # ✓
```
The PC test target fails:
```
cargo build -p mega-blastoise-test  # ✗
```
Error: `ahash` (a transitive dep via hashbrown → battler) uses `once_cell::race::OnceBox`
on x86_64, but our once_cell patch broke the `alloc → race` feature coupling that ahash
relied on.

## The fix in progress (half done, needs finishing)

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

## After applying step 2, verify both:
```
cargo build -p mega-blastoise-fw    # must stay green
cargo build -p mega-blastoise-test  # must go green
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

## Then commit — suggested breakdown:
1. `restructure: convert to Cargo workspace with core, fw, and test members`
   - new Cargo.toml (workspace), mega_blastoise_core/, mega_blastoise_fw/,
     mega_blastoise_test/, removal of old root .cargo/config.toml
2. `patches/once_cell: gate race module by target_has_atomic instead of removing alloc coupling`
   - patches/once_cell/Cargo.toml (revert alloc=[])
   - patches/once_cell/src/lib.rs (add target_has_atomic cfg)

## Files that still exist at workspace root but are now superseded
These are leftover from before the restructure and should be deleted after
both builds pass:
- `src/main.rs`       — replaced by mega_blastoise_fw/src/main.rs
- `src/data_store.rs` — replaced by mega_blastoise_core/src/data_store.rs
- `build.rs`          — replaced by mega_blastoise_core/build.rs
- `memory.x`          — replaced by mega_blastoise_fw/memory.x
- `.cargo/config.toml` — already deleted

The `src/` dir itself can be removed once confirmed both targets build.
