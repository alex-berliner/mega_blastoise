//! `focusenergy`: Focus Energy's volatile, +2 critical hit stages. No
//! effect state.
//!
//! PS: data/moves.ts focusenergy.condition

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onStart(target, source, effect)`: `-start|MON|move: Focus Energy`,
/// `[silent]` when Psych Up or Transform copies it (Dragon Cheer, Z-Power,
/// Costar and Imposter are later gens).
// PS: data/moves.ts focusenergy.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let dragoncheer = b.dex.condition_id("dragoncheer");
    if dragoncheer.is_some_and(|c| b.mon(target).volatiles.contains(EffectId::Condition(c))) {
        return Some(crate::relay::Relay::FAIL);
    }
    let silent = matches!(a.effect, EffectId::Move(m) if m == moves::PSYCHUP || m == moves::TRANSFORM);
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::FOCUSENERGY.0)));
    let kw = if silent { Kw::flags(KwFlags::SILENT) } else { Kw::NONE };
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw });
    None
}

/// `onModifyCritRatio(critRatio)`: `critRatio + 2`.
// PS: data/moves.ts focusenergy.condition.onModifyCritRatio
fn on_modify_crit_ratio(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(Relay::Int(crit_ratio)) = a.relay else { panic!("ModifyCritRatio relay {:?}", a.relay) };
    Some(Relay::Int(crit_ratio + 2))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::ModifyCritRatio, on_modify_crit_ratio),
]);
