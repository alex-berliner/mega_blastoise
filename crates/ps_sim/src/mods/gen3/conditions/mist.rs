//! `mist`: Mist's side condition. Other Pokemon cannot lower the side's
//! stats. Five turns. No effect state.
//!
//! PS: data/moves.ts mist.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::abilities::effect_has_secondaries;
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::moves;

/// `onTryBoost(boost, target, source, effect)`: another Pokemon's drops
/// are deleted, with `-activate|MON|move: Mist` unless the effect has
/// secondaries. `effect.infiltrates` is never set in gen 3 (Infiltrator's
/// is gen 5).
// PS: data/moves.ts mist.condition.onTryBoost
fn on_try_boost(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    if target == source {
        return None;
    }
    let Relay::Boosts(mut boost) = a.relay? else { return None };
    let mut show_msg = false;
    for value in boost.iter_mut() {
        if *value < 0 {
            *value = 0;
            show_msg = true;
        }
    }
    if show_msg && !effect_has_secondaries(b, a.effect) {
        let mon = b.ident(target);
        let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::MIST.0)));
        b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    }
    Some(Relay::Boosts(boost))
}

/// `onSideStart(side)`: `-sidestart|SIDE|Mist`.
// PS: data/moves.ts mist.condition.onSideStart
fn on_side_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideStart { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::MIST.0))), kw: Kw::NONE });
    None
}

/// `onSideEnd(side)`: `-sideend|SIDE|Mist`.
// PS: data/moves.ts mist.condition.onSideEnd
fn on_side_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideEnd { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::MIST.0))), kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryBoost, on_try_boost),
    Handler::on(EventId::SideStart, on_side_start),
    Handler::on(EventId::SideEnd, on_side_end),
]);
