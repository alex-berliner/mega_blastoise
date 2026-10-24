//! `lightscreen`: gen 1's Light Screen is a volatile of its user, not a
//! side condition. Gen 1's `getDamage` doubles the holder's Special against
//! special moves while it is up; Haze ends it. Only `onStart` has a
//! handler.
//!
//! Effect state: none.
//!
//! PS: data/mods/gen1/moves.ts lightscreen.condition (lines 474-478)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;

/// `onStart(pokemon)`: `-start|MON|Light Screen`.
// PS: data/mods/gen1/moves.ts lightscreen.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::LIGHTSCREEN.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, on_start)]);
