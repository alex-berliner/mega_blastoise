//! Pay Day: scatters coins.
//!
//! PS: data/mods/gen4/moves.ts payday.onHit

use ps_protocol::{Arg, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHit()`: `-fieldactivate|move: Pay Day`.
// PS: data/mods/gen4/moves.ts payday.onHit
fn on_hit(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let effect = Arg::move_effect(MoveNum(ps_dex::gen3::moves::PAYDAY.0));
    b.add(Msg::FieldActivate { effect, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
