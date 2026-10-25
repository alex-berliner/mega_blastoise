//! Follow Me: `onTry` fails it unless a side has more than one active
//! Pokemon, so in singles it always fails silently after its `|move|`
//! line.
//!
//! PS: data/moves.ts followme.onTry

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTry(source)`: `this.activePerHalf > 1`.
// PS: data/moves.ts followme.onTry
fn on_try(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Bool(b.sides[0].active.len() > 1))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Try, on_try)]);
