//! `watersport`: Water Sport's volatile (the move targets `all`, so the user gets
//! it). Fire moves' base power is halved while the holder is active. No
//! effect state.
//!
//! PS: data/mods/gen5/moves.ts watersport.condition (onStart),
//! data/mods/gen4/moves.ts watersport.condition (onAnyBasePower)

use ps_dex::Type;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onStart(pokemon)`: `-start|MON|move: Water Sport`.
// PS: data/mods/gen5/moves.ts watersport.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::WATERSPORT.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onAnyBasePower(basePower, user, target, move)`: Fire moves at half
/// power.
// PS: data/mods/gen4/moves.ts watersport.condition.onAnyBasePower
fn on_any_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // `move.type`: the move object's (`???` for a future move's plain
    // `moveData`, whose event effect is none).
    if super::event_move_type(b, a) == Type::Fire {
        b.debug(Text::new(TextId::WaterSportWeaken));
        b.chain_modify(1, 2);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BasePower, on_any_base_power).prefix(Prefix::Any).priority(3),
]);
