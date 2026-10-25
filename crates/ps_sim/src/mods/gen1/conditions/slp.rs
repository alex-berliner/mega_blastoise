//! Sleep: the `-status` line, the turn counter, the "cannot move" line
//! and the waking up at the end of the turn.
//!
//! `this.effectState` is the status state (PS's `statusState`): `time` is
//! the turns of sleep left, `startTime` the turns it started with. They
//! live in `data[0]` and `data[1]`.
//!
//! PS: data/mods/gen1/conditions.ts slp (line 53)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::MoveId;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Status};

/// `this.effectState.time`: the turns of sleep left.
const TIME: usize = 0;
/// `this.effectState.startTime`: the turns of sleep it started with.
const START_TIME: usize = 1;

/// `onStart(target, source, sourceEffect)`: the `-status` line, the sleep
/// counter and the nightmare's end.
// PS: data/mods/gen1/conditions.ts slp.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let kw = match a.effect {
        // `[from] move: ${sourceEffect.name}`.
        EffectId::Move(m) => Kw::from(Arg::new(ArgValue::MoveEffect(MoveNum(m.0)))),
        _ => Kw::NONE,
    };
    b.add(Msg::SetStatus { mon, status: Status::Slp, kw });
    // 1-7 turns.
    let start_time = b.prng.random_range(1, 8) as i16;
    {
        let state = b.effect_state();
        state.data[START_TIME] = start_time;
        state.data[TIME] = start_time;
    }
    b.remove_nightmare(target);
    None
}

/// `onBeforeMove(pokemon, target, move)`: the sleep counter and the
/// "cannot move" line.
// PS: data/mods/gen1/conditions.ts slp.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let time = {
        let mon = b.mon_mut(pokemon);
        mon.status_state.data[TIME] -= 1;
        mon.status_state.data[TIME]
    };
    if time > 0 {
        let mon = b.ident(pokemon);
        let reason = Arg::new(ArgValue::Status(Status::Slp));
        b.add(Msg::Cant { mon, reason, mv: None, kw: Kw::NONE });
    }
    b.mon_mut(pokemon).last_move = MoveId::NONE;
    Some(Relay::FAIL)
}

/// `onAfterMoveSelf(pokemon)`: waking up when the counter ran out.
// PS: data/mods/gen1/conditions.ts slp.onAfterMoveSelf
fn on_after_move_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).status_state.data[TIME] <= 0 {
        b.cure_status(pokemon, false);
    }
    None
}

/// `onDisableMove(target)`: `target.maybeLocked = false`.
// PS: data/mods/gen1/conditions.ts slp.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.mon_mut(target).maybe_locked = false;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move).priority(10),
    Handler::on(EventId::AfterMoveSelf, on_after_move_self).priority(3),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
