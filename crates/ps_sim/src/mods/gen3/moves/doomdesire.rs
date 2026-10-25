//! Doom Desire: the hit lands at the end of the turn after next, on whatever
//! stands in the target's slot (the `futuremove` slot condition), with
//! damage rolled when the move is used. See
//! [`crate::mods::gen3::moves::future_move_try`]. No effect state.
//!
//! PS: data/mods/gen3/moves.ts doomdesire.onTry

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::future_move_try;
use crate::relay::Relay;
use crate::state::Battle;

/// `onTry(source, target)`.
// PS: data/mods/gen3/moves.ts doomdesire.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    future_move_try(b, a)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Try, on_try)]);
