//! Guaranteed Secondary Mod: every secondary effect of a move happens.
//!
//! PS has one definition for every gen (`data/rulesets.ts`), so gen 3
//! registers this one too. No effect state.
//!
//! PS: data/rulesets.ts guaranteedsecondarymod

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move)`: `if (move.secondaries)`, the debug line and
/// `secondary.chance = 100` for each of this use's secondaries (the ones
/// on the list now: a `ModifyMove` of lower priority, King's Rock's, adds
/// its flinch after).
// PS: data/rulesets.ts guaranteedsecondarymod.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let count = b.active_move().expect("ModifyMove without an active move").secondaries(b.dex).count();
    if count == 0 {
        return None;
    }
    b.debug(Text::new(TextId::GuaranteeingSecondary));
    assert!(count <= 8, "more secondaries than the mask holds");
    b.active_move_mut().expect("ModifyMove without an active move").secondaries_guaranteed = u8::MAX >> (8 - count);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
