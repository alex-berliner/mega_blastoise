//! Secret Power: `onModifyMove` swaps the secondary by terrain; gen 3 has
//! no terrain, so it always keeps the paralysis chance.
//!
//! PS: data/moves.ts secretpower.onModifyMove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move, pokemon)`: `if (this.field.isTerrain('')) return`.
/// `effectiveTerrain` runs a `TryTerrain` event no gen 3 effect answers,
/// and the terrain is always `''`, so it returns here.
// PS: data/moves.ts secretpower.onModifyMove
fn on_modify_move(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
