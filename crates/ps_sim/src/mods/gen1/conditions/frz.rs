//! Freeze: the `-status` line, the "cannot move" line, and the thaw a
//! fire move gives the frozen Pokemon.
//!
//! The freeze keeps no state of its own.
//!
//! PS: data/mods/gen1/conditions.ts frz (line 87)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;
use ps_dex::MoveId;
use ps_protocol::{Arg, ArgValue, Kw, Msg, Status};

/// `onStart(target)`: `this.add('-status', target, 'frz')`.
// PS: data/mods/gen1/conditions.ts frz.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::SetStatus { mon, status: Status::Frz, kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon, target, move)`: the frozen Pokemon cannot move.
// PS: data/mods/gen1/conditions.ts frz.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let reason = Arg::new(ArgValue::Status(Status::Frz));
    b.add(Msg::Cant { mon, reason, mv: None });
    b.mon_mut(pokemon).last_move = MoveId::NONE;
    Some(Relay::FAIL)
}

/// `onAfterMoveSecondary(target, source, move)`: a fire move thaws the
/// frozen Pokemon.
// PS: data/mods/gen1/conditions.ts frz.onAfterMoveSecondary
fn on_after_move_secondary(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let EffectId::Move(m) = a.effect else { return None };
    // `move.secondary`: the first secondary, which PS's dump checks to be
    // the move's `secondary` field.
    let burns = b.dex.move_def(m).secondaries().first().is_some_and(|s| s.status == conditions::BRN);
    if burns {
        b.cure_status(target, false);
    }
    None
}

/// `onDisableMove(target)`: `target.maybeLocked = false`.
// PS: data/mods/gen1/conditions.ts frz.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.mon_mut(target).maybe_locked = false;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move).priority(12),
    Handler::on(EventId::AfterMoveSecondary, on_after_move_secondary),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
