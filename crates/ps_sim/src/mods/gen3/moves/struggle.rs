//! Struggle: `onModifyMove` makes it typeless (`???`), so it never gets
//! STAB, hits every type neutrally and no type is immune. No effect state.
//!
//! The move object `onModifyMove(move)` changes is the relay, the move in
//! use: `battle.activeMove` (`useMoveInner` sets it just before).
//!
//! PS: data/mods/gen4/moves.ts struggle

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

/// `onModifyMove(move) { move.type = '???'; }`
// PS: data/mods/gen4/moves.ts struggle.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let mv = b.active_move_mut().expect("ModifyMove without an active move");
    mv.type_ = Type::None;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
