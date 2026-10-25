//! Light Screen: gen 1's is a volatile on the user (`volatileStatus`, no side
//! condition), and the move fails when the user already has it.
//!
//! PS: data/mods/gen1/moves.ts lightscreen.onTryHit (lines 469-473)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onTryHit(pokemon)`: `false` when the volatile is already up.
// PS: data/mods/gen1/moves.ts lightscreen.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.has_vol(pokemon, conditions::LIGHTSCREEN) {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
