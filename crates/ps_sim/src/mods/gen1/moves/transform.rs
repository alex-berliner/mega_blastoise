//! Transform: the user becomes a copy of the target (`transformInto`,
//! ported as [`Battle::transform_into`]). No effect state.
//!
//! Gen 1 inherits the base move. Gen 1 lets a transformed Pokemon be
//! copied (the `transformed` check starts in gen 2), copies the modified
//! stats, and its Transform hits through a semi-invulnerable target
//! (`invulnerability`, G1-3). The PP of the copied slots and the PP sync
//! with the real slots are in `scripts.rs` (`deductPP`, `useMove`).
//!
//! PS: data/moves.ts transform (lines 19822-19838); sim/pokemon.ts
//! transformInto (lines 1276-1391)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHit(target, pokemon) { return pokemon.transformInto(target); }`
// PS: data/moves.ts transform.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Transform hits a Pokemon");
    let pokemon = a.source.expect("Transform has a user");
    Some(Relay::Bool(b.transform_into(pokemon, target)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
