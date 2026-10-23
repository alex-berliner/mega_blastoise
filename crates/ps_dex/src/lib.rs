//! Pokemon Showdown dex tables.
//!
//! Per the design (section 4 of `docs/architecture/showdown-split-plan.md`),
//! `build.rs` reads the per-gen JSON dumps under `data/{gen1,gen3}/*.json`
//! (produced by `tools/ps-diff/dump-dex.js`, task P1-3) and emits `&'static`
//! lookup tables with `u16` ids in PS's iteration order. The `Type` enum
//! that used to live in `crates/battle_types` moves here; `battle_types`
//! becomes a re-export shim until the Gen 3 engine, its last user, is
//! deleted. This crate starts empty in P1-0 (scaffold only); the dex dump
//! and tables land in P1-3.

#![no_std]

extern crate alloc;
