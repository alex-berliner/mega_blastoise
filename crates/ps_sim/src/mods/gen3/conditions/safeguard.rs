//! `safeguard`: Safeguard's side condition. Other Pokemon cannot give the
//! side's Pokemon a status, confusion or Yawn's drowsiness. Five turns
//! (seven with Persistent, which is gen 4). No effect state.
//!
//! PS: data/moves.ts safeguard.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::abilities::effect_has_secondaries;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};
use ps_dex::gen3::{abilities, conditions, moves};

fn safeguard_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::SAFEGUARD.0)))
}

/// `source?.hasAbility('persistent')`: Persistent is gen 4, so no gen 3
/// Pokemon has it.
fn persistent(b: &mut Battle, source: Option<MonRef>) -> bool {
    let persistent = b.dex.ability_id("persistent");
    match (source, persistent) {
        (Some(s), Some(p)) => {
            assert!(!b.has_ability(s, p), "unported: Persistent's Safeguard");
            false
        }
        _ => false,
    }
}

/// `durationCallback(target, source, effect)`: 5.
// PS: data/moves.ts safeguard.condition.durationCallback
fn duration_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    persistent(b, a.source);
    Some(Relay::Int(5))
}

/// `onSetStatus(status, target, source, effect)`. Yawn's own sleep gets
/// through (`effect.id === 'yawn'`); `effect.infiltrates` is never set in
/// gen 3 (Infiltrator's is gen 5).
// PS: data/moves.ts safeguard.condition.onSetStatus
fn on_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    match a.effect {
        EffectId::None => return None,
        EffectId::Move(m) if m == moves::YAWN => return None,
        EffectId::Condition(c) if c == conditions::YAWN => return None,
        _ => {}
    }
    if target != source {
        b.debug(Text::new(TextId::InterruptingSetStatus));
        let synchronize = a.effect == EffectId::Ability(abilities::SYNCHRONIZE);
        let move_without_secondaries = matches!(a.effect, EffectId::Move(_)) && !effect_has_secondaries(b, a.effect);
        if synchronize || move_without_secondaries {
            let mon = b.ident(target);
            b.add(Msg::Activate { mon: Some(mon), effect: safeguard_arg(), a: None, b: None, kw: Kw::NONE });
        }
        return Some(Relay::Null);
    }
    None
}

/// `onTryAddVolatile(status, target, source, effect)`: confusion and
/// Yawn from another Pokemon.
// PS: data/moves.ts safeguard.condition.onTryAddVolatile
fn on_try_add_volatile(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    if a.effect.is_none() {
        return None;
    }
    let Some(Relay::Effect(EffectId::Condition(status))) = a.relay else { return None };
    if (status == conditions::CONFUSION || status == conditions::YAWN) && target != source {
        if matches!(a.effect, EffectId::Move(_)) && !effect_has_secondaries(b, a.effect) {
            let mon = b.ident(target);
            b.add(Msg::Activate { mon: Some(mon), effect: safeguard_arg(), a: None, b: None, kw: Kw::NONE });
        }
        return Some(Relay::Null);
    }
    None
}

/// `onSideStart(side, source)`: `-sidestart|SIDE|Safeguard`.
// PS: data/moves.ts safeguard.condition.onSideStart
fn on_side_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    persistent(b, a.source);
    let side = b.sides[side as usize].id();
    b.add(Msg::SideStart { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::SAFEGUARD.0))), kw: Kw::NONE });
    None
}

/// `onSideEnd(side)`: `-sideend|SIDE|Safeguard`.
// PS: data/moves.ts safeguard.condition.onSideEnd
fn on_side_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideEnd { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::SAFEGUARD.0))), kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::SetStatus, on_set_status),
    Handler::on(EventId::TryAddVolatile, on_try_add_volatile),
    Handler::on(EventId::SideStart, on_side_start),
    Handler::on(EventId::SideEnd, on_side_end),
]);
