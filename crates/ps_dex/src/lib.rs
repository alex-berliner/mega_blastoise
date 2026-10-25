//! Pokemon Showdown dex tables, one resolved dex per generation.
//!
//! `build.rs` reads the JSON dumps under `data/{gen1,gen3}/*.json` (made by
//! `tools/ps-diff/dump-dex.js` from `Dex.mod('gen1')` and `Dex.mod('gen3')`
//! of the PS pinned in `tools/ps-diff` (`meta.json` records `psVersion` and
//! `psCommit`), the mod chain already resolved) and emits `&'static`
//! tables: [`GEN1`] and [`GEN3`], with `u16` ids in PS's iteration order
//! (design section 4 of `docs/architecture/showdown-split-plan.md`). The
//! [`Type`] enum that used to live in `crates/battle_types` lives here;
//! `battle_types` re-exports it until the Gen 3 engine, its last user, is
//! deleted.
//!
//! To read an effect's resolved definition, handler source included:
//! `node tools/ps-diff/show-effect.js <gen> <kind> <id>`.

#![no_std]

extern crate alloc;

pub mod ids;
pub mod tables;
pub mod text;
pub mod types;

pub use ids::{AbilityId, ConditionId, ItemId, MoveId, SpeciesId};
pub use tables::*;
pub use text::Str;
pub use types::*;

use text::s;

include!(concat!(env!("OUT_DIR"), "/dex.rs"));
