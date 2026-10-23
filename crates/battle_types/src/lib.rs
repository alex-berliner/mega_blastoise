//! The one `Type` enum, shared by every generation's engine, now owned by
//! `ps_dex` (design section 4, decision 7 of
//! `docs/architecture/showdown-split-plan.md`).
//!
//! This crate is a re-export shim until the Gen 3 engine, its last user, is
//! deleted. The old engines only ever named `battle_types::Type`; the enum,
//! its variant order and `name()` / `abbr()` are unchanged in `ps_dex`.
//!
//! The order is load-bearing. Both old engines' `build.rs` emit their chart
//! rows in this order and index them by `Type as usize` behind a
//! `< TYPE_COUNT` guard, so a variant past the end of a given generation's
//! chart reads as neutral.

#![no_std]

pub use ps_dex::Type;
