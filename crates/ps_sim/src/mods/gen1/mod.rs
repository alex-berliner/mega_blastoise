//! Gen 1 effects: the registry of every effect with handlers, one file per
//! effect under `moves/`, `conditions/`, `items/`, `abilities/` and
//! `rules/` (see `mods/mod.rs`). Entries stay sorted by `EffectId`.

pub mod rules;

use crate::effect::{EffectId, Registry, Rule};

/// Gen 1's effect definitions, sorted by id.
pub static REGISTRY: Registry = Registry {
    entries: &[
        (EffectId::Rule(Rule::SleepClauseMod), &rules::sleepclausemod::DEF),
        (EffectId::Rule(Rule::FreezeClauseMod), &rules::freezeclausemod::DEF),
    ],
};
