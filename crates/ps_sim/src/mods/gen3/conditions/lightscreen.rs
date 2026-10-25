//! `lightscreen`: Light Screen's side condition. Special damage to the
//! side's Pokemon is halved, except a critical hit. Five turns (eight with
//! Light Clay, which is gen 4). No effect state.
//!
//! PS: data/moves.ts lightscreen.condition (durationCallback, onSideStart,
//! onSideEnd), data/mods/gen4/moves.ts lightscreen.condition
//! (onAnyModifyDamagePhase1)

use ps_dex::MoveCategory;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::moves;

use super::reflect::{screen_duration, screen_modify};

fn light_screen_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::LIGHTSCREEN.0)))
}

/// `onSideStart(side)`: `-sidestart|SIDE|move: Light Screen`.
// PS: data/moves.ts lightscreen.condition.onSideStart
fn on_side_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideStart { side, effect: light_screen_arg(), kw: Kw::NONE });
    None
}

/// `onSideEnd(side)`: `-sideend|SIDE|move: Light Screen`.
// PS: data/moves.ts lightscreen.condition.onSideEnd
fn on_side_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideEnd { side, effect: light_screen_arg(), kw: Kw::NONE });
    None
}

/// `onAnyModifyDamagePhase1`: special moves.
// PS: data/mods/gen4/moves.ts lightscreen.condition.onAnyModifyDamagePhase1
fn on_any_modify_damage_phase1(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    screen_modify(b, a, MoveCategory::Special, TextId::LightScreenWeaken)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, screen_duration),
    Handler::on(EventId::SideStart, on_side_start),
    Handler::on(EventId::SideEnd, on_side_end),
    Handler::on(EventId::ModifyDamagePhase1, on_any_modify_damage_phase1).prefix(Prefix::Any),
]);
