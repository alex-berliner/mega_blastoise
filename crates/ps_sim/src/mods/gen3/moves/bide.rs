//! Bide: the user stores the damage it takes for two turns (the `bide`
//! volatile, its `volatileStatus`), then strikes back for twice that; the
//! condition does the work (`conditions/bide.rs`). While the volatile
//! lasts, the move itself does nothing more. No effect state.
//!
//! PS: data/moves.ts bide.beforeMoveCallback

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `beforeMoveCallback(pokemon) { if (pokemon.volatiles['bide']) return
/// true; }`
// PS: data/moves.ts bide.beforeMoveCallback
fn before_move_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.mon(pokemon).volatiles.contains(EffectId::Condition(conditions::BIDE)).then_some(Relay::TRUE)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BeforeMoveCallback, before_move_callback)]);
