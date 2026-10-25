//! `destinybond`: Destiny Bond's volatile. If a move makes the holder
//! faint, the attacker faints too; it goes before the holder's next move
//! (unless that is Destiny Bond again) or when that move is aborted. No
//! effect state of its own.
//!
//! PS: data/moves.ts destinybond.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(pokemon)`: `-singlemove|MON|Destiny Bond`.
// PS: data/moves.ts destinybond.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::SingleMove { mon, effect: Arg::new(ArgValue::Move(MoveNum(moves::DESTINYBOND.0))), kw: Kw::NONE });
    None
}

/// `onFaint(target, source, effect)`: a foe's move (not a future move)
/// takes the attacker down with it (no Dynamax in gen 3).
// PS: data/moves.ts destinybond.condition.onFaint
fn on_faint(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    if b.mon(source).hp == 0 || a.effect.is_none() || source.side == target.side {
        return None;
    }
    let EffectId::Move(id) = a.effect else { return None };
    if b.dex.move_def(id).flags.has("futuremove") {
        return None;
    }
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::DESTINYBOND.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    b.faint(source, None, EffectId::None);
    None
}

/// `onBeforeMove(pokemon, target, move)`: removed before any other move.
// PS: data/moves.ts destinybond.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if a.effect == EffectId::Move(moves::DESTINYBOND) {
        return None;
    }
    b.debug(Text::new(TextId::RemovingDestinyBond));
    b.remove_volatile(pokemon, conditions::DESTINYBOND);
    None
}

/// `onMoveAborted(pokemon, target, move)`: removed.
// PS: data/moves.ts destinybond.condition.onMoveAborted
fn on_move_aborted(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, conditions::DESTINYBOND);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Faint, on_faint),
    Handler::on(EventId::BeforeMove, on_before_move).priority(-1),
    Handler::on(EventId::MoveAborted, on_move_aborted),
]);
