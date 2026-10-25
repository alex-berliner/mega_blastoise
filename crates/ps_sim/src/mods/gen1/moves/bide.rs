//! Bide: the move adds the `bide` volatile (see `conditions/bide.rs`),
//! which then answers `BeforeMove` every turn until it unleashes. Gen 1
//! gives it `accuracy: true` and the `???` type (the dex resolves both).
//! No effect state.
//!
//! PS: data/moves.ts bide (lines 1264-1336) for `beforeMoveCallback`;
//! data/mods/gen1/moves.ts bide (lines 33-68) for the rest

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `beforeMoveCallback(pokemon)`.
// PS: data/moves.ts bide.beforeMoveCallback
fn before_move_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).volatiles.contains(EffectId::Condition(conditions::BIDE)) {
        return Some(Relay::TRUE);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BeforeMoveCallback, before_move_callback)]);
