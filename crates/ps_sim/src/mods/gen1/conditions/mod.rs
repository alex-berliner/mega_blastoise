//! Gen 1 conditions with handlers, one file per condition (statuses,
//! volatiles, and moves' own `condition`s, as `dex.conditions` finds them).
//!
//! The `residualdmg` volatile is Toxic's counter: gen 1's burn and poison
//! read it, so a Pokemon that was poisoned once keeps the escalating
//! damage after a Rest.
//!
//! PS: data/mods/gen1/conditions.ts, data/mods/gen2/conditions.ts,
//! data/mods/gen7/moves.ts.

pub mod bide;
pub mod brn;
pub mod confusion;
pub mod defensecurl;
pub mod disable;
pub mod fakepartiallytrapped;
pub mod flinch;
pub mod focusenergy;
pub mod frz;
pub mod invulnerability;
pub mod leechseed;
pub mod lightscreen;
pub mod lockedmove;
pub mod minimize;
pub mod mist;
pub mod mustrecharge;
pub mod par;
pub mod partiallytrapped;
pub mod partialtrappinglock;
pub mod psn;
pub mod rage;
pub mod reflect;
pub mod residualdmg;
pub mod slp;
pub mod substitute;
pub mod tox;
pub mod toxic;
pub mod trapped;
pub mod twoturnmove;

use ps_dex::gen1::conditions;
use ps_dex::ConditionId;

use crate::effect::EffectId;
use crate::state::{Battle, EffectState, MonRef};

/// PS `this.clampIntRange(Math.floor(pokemon.maxhp / 16), 1)`: what one
/// burn or poison tick costs.
// PS: data/mods/gen1/conditions.ts brn.onAfterMoveSelf (line 21)
pub fn residual_damage(b: &Battle, pokemon: MonRef) -> i32 {
    (i32::from(b.mon(pokemon).maxhp) / 16).max(1)
}

/// PS `pokemon.volatiles['residualdmg']`: Toxic's counter lives on the
/// Pokemon as a volatile of its own.
pub fn residualdmg_state(b: &Battle, pokemon: MonRef) -> Option<EffectState> {
    b.mon(pokemon).volatiles.get(EffectId::Condition(conditions::RESIDUALDMG)).copied()
}

/// PS `pokemon.volatiles['residualdmg'].counter`.
pub fn residualdmg_counter(b: &Battle, pokemon: MonRef) -> i32 {
    residualdmg_state(b, pokemon).map_or(0, |s| i32::from(s.counter))
}

/// PS `pokemon.volatiles['residualdmg'].counter++`.
pub fn bump_residualdmg_counter(b: &mut Battle, pokemon: MonRef) {
    if let Some(s) = b.mon_mut(pokemon).volatiles.get_mut(EffectId::Condition(conditions::RESIDUALDMG)) {
        s.counter = s.counter.saturating_add(1);
    }
}

/// PS `pokemon.volatiles[c]`.
pub fn volatile_state(b: &Battle, pokemon: MonRef, c: ConditionId) -> Option<EffectState> {
    b.mon(pokemon).volatiles.get(EffectId::Condition(c)).copied()
}

/// PS `pokemon.volatiles[c]`, mutable.
pub fn volatile_state_mut(b: &mut Battle, pokemon: MonRef, c: ConditionId) -> Option<&mut EffectState> {
    b.mon_mut(pokemon).volatiles.get_mut(EffectId::Condition(c))
}
