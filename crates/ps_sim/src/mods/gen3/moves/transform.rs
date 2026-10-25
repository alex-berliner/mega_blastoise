//! Transform: the user becomes a copy of the target (`transformInto`,
//! ported as [`Battle::transform_into`], with gen 3's ability copy and the
//! foe's types left unknown, `knownType`). No effect state.
//!
//! PS: data/moves.ts transform.onHit; sim/pokemon.ts transformInto

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
