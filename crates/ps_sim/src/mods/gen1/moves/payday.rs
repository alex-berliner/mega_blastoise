//! Pay Day: the field announces the money the move brings in.
//!
//! No effect state.
//!
//! PS: data/mods/gen4/moves.ts payday.onHit (line 999; gen 4's handler, inherited
//! by gen 1: gen 1 does not override Pay Day)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;
use ps_protocol::{Arg, Kw, Msg, MoveNum};

/// `onHit()`.
// PS: data/mods/gen4/moves.ts payday.onHit
fn on_hit(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `this.add('-fieldactivate', 'move: Pay Day')`.
    b.add(Msg::FieldActivate {
        effect: Arg::move_effect(MoveNum(moves::PAYDAY.0)),
        kw: Kw::NONE,
    });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
