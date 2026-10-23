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

/// Ported effects for Gen 1: every `kind:id` the new `ps_sim` carries
/// handlers for. Today the list is empty (P1-8 fills in the gen-1
/// handlers). The protocol-diff harness (`ps_sim_cli --ported 1`) reads
/// this list to restrict the wide generator; an empty list is the signal
/// that `--teams wide` against `ps_sim_cli` has nothing to draw from.
pub static PORTED: [&'static str; 0] = [];
