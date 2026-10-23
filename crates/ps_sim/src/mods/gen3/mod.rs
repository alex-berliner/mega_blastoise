//! Gen 3 effects: the registry of every effect with handlers, one file per
//! effect under `moves/`, `conditions/`, `items/`, `abilities/` and
//! `rules/` (see `mods/mod.rs`). Entries stay sorted by `EffectId`.

use crate::effect::Registry;

/// Gen 3's effect definitions, sorted by id. Empty until the porting tasks
/// add effects.
pub static REGISTRY: Registry = Registry { entries: &[] };

/// Ported effects for Gen 3: every `kind:id` the new `ps_sim` carries
/// handlers for. Today the list is empty (the gen-3 ports fill it in).
/// The protocol-diff harness (`ps_sim_cli --ported 3`) reads this list
/// to restrict the wide generator; an empty list is the signal that
/// `--teams wide` against `ps_sim_cli` has nothing to draw from.
pub static PORTED: [&'static str; 0] = [];
