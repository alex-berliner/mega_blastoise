//! Freeze: the `-status` line, the thaw chance before a move, thawing by a
//! `defrost` move of its own or a Fire move that hits it.
//!
//! The freeze keeps no state of its own.
//!
//! PS: data/conditions.ts frz (onStart, onModifyMove, onAfterMoveSecondary),
//! data/mods/gen4/conditions.ts frz (onBeforeMove),
//! data/mods/gen3/conditions.ts frz (onDamagingHit)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::MoveCategory;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Status};

use super::add_status_line;

/// The dex move an event runs for, if it is one.
fn dex_move(a: &EventArgs) -> Option<ps_dex::MoveId> {
    match a.effect {
        EffectId::Move(m) => Some(m),
        _ => None,
    }
}

/// `onStart(target, source, sourceEffect)`: the `-status` line (the
/// Shaymin-Sky forme change cannot happen in gen 3).
// PS: data/conditions.ts frz.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    add_status_line(b, a, Status::Frz)
}

/// `onBeforeMove(pokemon, target, move)`: a one in five thaw, a `defrost`
/// move going ahead, or the "cannot move" line.
// PS: data/mods/gen4/conditions.ts frz.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.prng.random_chance(1, 5) {
        b.cure_status(pokemon, false);
        return None;
    }
    if dex_move(a).is_some_and(|m| b.dex.move_def(m).flags.has("defrost")) {
        return None;
    }
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::new(ArgValue::Status(Status::Frz)), mv: None, kw: Kw::NONE });
    Some(Relay::FAIL)
}

/// `onModifyMove(move, pokemon)`: a `defrost` move thaws its user.
// PS: data/conditions.ts frz.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let Some(m) = dex_move(a) else { return None };
    if b.dex.move_def(m).flags.has("defrost") {
        let mon = b.ident(pokemon);
        let kw = Kw::from(Arg::new(ArgValue::MoveEffect(MoveNum(m.0))));
        b.add(Msg::CureStatus { mon, status: Status::Frz, kw });
        b.clear_status(pokemon);
    }
    None
}

/// `onAfterMoveSecondary(target, source, move)`: `move.thawsTarget` (no
/// gen 3 move has it).
// PS: data/conditions.ts frz.onAfterMoveSecondary
fn on_after_move_secondary(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if dex_move(a).is_some_and(|m| b.dex.move_def(m).bools.has("thawsTarget")) {
        b.cure_status(target, false);
    }
    None
}

/// `onDamagingHit(damage, target, source, move)`: a Fire move thaws the
/// target, by the dex's type (Hidden Power and Weather Ball do not count).
// PS: data/mods/gen3/conditions.ts frz.onDamagingHit
fn on_damaging_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let Some(m) = dex_move(a) else { return None };
    let category = b.active_move().map_or(b.dex.move_def(m).category, |am| am.category);
    if b.dex.move_def(m).type_ == ps_dex::Type::Fire && category != MoveCategory::Status {
        b.cure_status(target, false);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move).priority(10),
    Handler::on(EventId::ModifyMove, on_modify_move),
    Handler::on(EventId::AfterMoveSecondary, on_after_move_secondary),
    Handler::on(EventId::DamagingHit, on_damaging_hit),
]);
