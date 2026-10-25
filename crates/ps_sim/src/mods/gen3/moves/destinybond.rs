//! Destiny Bond: a bond still up from the last use goes first (gen 6's
//! `onPrepareHit`, which gen 3 inherits); the volatile does the rest
//! (`conditions::destinybond`).
//!
//! PS: data/mods/gen6/moves.ts destinybond.onPrepareHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onPrepareHit(pokemon)`: `pokemon.removeVolatile('destinybond')`.
// PS: data/mods/gen6/moves.ts destinybond.onPrepareHit
fn on_prepare_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, conditions::DESTINYBOND);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::PrepareHit, on_prepare_hit)]);
