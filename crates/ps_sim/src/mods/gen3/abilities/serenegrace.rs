//! Serene Grace: the secondary effects of the holder's moves happen twice
//! as often.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/mods/gen4/abilities.ts serenegrace (onModifyMove)

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move)`: `if (move.secondaries && move.id !== "chatter")`,
/// the debug line and `secondary.chance *= 2` for each secondary with a
/// chance (this use's copy of them). Chatter is a gen 4 move.
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let mv = b.active_move().expect("ModifyMove without an active move");
    if b.dex.move_def(mv.id).secondaries().is_empty() {
        return None;
    }
    b.debug(Text::new(TextId::DoublingSecondaryChance));
    b.active_move_mut().expect("ModifyMove without an active move").secondary_chances_doubled = true;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move).priority(-2)]);
