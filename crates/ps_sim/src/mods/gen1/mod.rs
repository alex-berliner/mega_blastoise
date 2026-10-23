//! Gen 1 effects: the registry of every effect with handlers, one file per
//! effect under `moves/`, `conditions/`, `items/`, `abilities/` and
//! `rules/` (see `mods/mod.rs`). Entries stay sorted by `EffectId`.

use crate::effect::Registry;

/// Gen 1's effect definitions, sorted by id. Empty until the porting tasks
/// add effects.
pub static REGISTRY: Registry = Registry { entries: &[] };
