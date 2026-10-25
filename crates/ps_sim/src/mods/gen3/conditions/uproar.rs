//! `uproar`: Uproar's lock (the move's `self.volatileStatus`), two to five
//! turns in gen 3; while it lasts nobody can fall asleep. No effect state.
//!
//! PS: data/moves.ts uproar.condition (`durationCallback` from
//! data/mods/gen3/moves.ts)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, Status};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::{conditions, moves};

fn uproar_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::UPROAR.0)))
}

/// `durationCallback()`: `this.random(2, 6)`.
// PS: data/mods/gen3/moves.ts uproar.condition.durationCallback
fn duration_callback(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(b.prng.random_range(2, 6) as i32))
}

/// `onStart(target)`: `-start|MON|Uproar`.
// PS: data/moves.ts uproar.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: uproar_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onResidual(target)`: Struggle drops the volatile without its `End`
/// (`delete target.volatiles['uproar']`), and the upkeep line follows
/// either way (Throat Chop is gen 7).
// PS: data/moves.ts uproar.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.mon(target).last_move == moves::STRUGGLE {
        if let Some(gone) = b.mon_mut(target).volatiles.remove(EffectId::Condition(conditions::UPROAR)) {
            b.bury(gone);
        }
    }
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: uproar_arg(), a: None, b: None, kw: Kw::flags(KwFlags::UPKEEP) });
    None
}

/// `onEnd(target)`: `-end|MON|Uproar`.
// PS: data/moves.ts uproar.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: uproar_arg(), kw: Kw::NONE });
    None
}

/// `onAnySetStatus(status, pokemon)`: no one falls asleep.
// PS: data/moves.ts uproar.condition.onAnySetStatus
fn on_any_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if a.relay != Some(Relay::Effect(EffectId::Condition(conditions::SLP))) {
        return None;
    }
    let pokemon = a.target.mon()?;
    let holder = b.effect_state().target;
    let mon = b.ident(pokemon);
    let what = Some(Arg::new(ArgValue::Status(Status::Slp)));
    let mut kw = Kw::from(uproar_arg());
    if holder == Holder::Mon(pokemon) {
        kw = kw.with(KwFlags::MSG);
    }
    b.add(Msg::Fail { mon, what, what2: None, kw });
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(11),
    Handler::on(EventId::End, on_end),
    Handler::value(EventId::LockMove, Relay::Move(moves::UPROAR)),
    Handler::on(EventId::SetStatus, on_any_set_status).prefix(Prefix::Any),
]);
