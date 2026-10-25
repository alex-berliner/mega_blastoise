//! `taunt`: Taunt's volatile. For two turns (gen 3) the holder's status
//! moves are disabled and stopped by `BeforeMove`. No effect state of its
//! own. Its end comes from its `duration` in the residual, ordered by
//! `onResidualOrder` 10 / `onResidualSubOrder` 15 (keys with no
//! `onResidual`: the dex's `residual_order`).
//!
//! PS: data/mods/gen4/moves.ts taunt.condition (onStart, onDisableMove,
//! onBeforeMove); data/mods/gen3/moves.ts taunt.condition (duration 2,
//! onEnd, no `onBeforeMovePriority`)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;
use ps_dex::MoveCategory;

fn taunt_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::TAUNT.0)))
}

/// `onStart(target)`: `-start|MON|move: Taunt`.
// PS: data/mods/gen4/moves.ts taunt.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: taunt_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onEnd(target)`: `-end|MON|move: Taunt|[silent]`.
// PS: data/mods/gen3/moves.ts taunt.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: taunt_arg(), kw: Kw::flags(KwFlags::SILENT) });
    None
}

/// `onDisableMove(pokemon)`: every move whose dex category is Status.
// PS: data/mods/gen4/moves.ts taunt.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let slots = b.mon(pokemon).move_slots;
    for slot in slots.iter().filter(|s| !s.id.is_none()) {
        if b.dex.move_def(slot.id).category == MoveCategory::Status {
            b.disable_move(pokemon, slot.id, false, EffectId::None);
        }
    }
    None
}

/// `onBeforeMove(attacker, defender, move)`: a status move is stopped
/// (`cant|MON|move: Taunt|MOVE`).
// PS: data/mods/gen4/moves.ts taunt.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let attacker = a.target.mon()?;
    let EffectId::Move(id) = a.effect else { return None };
    let category = match b.active_move() {
        Some(am) if am.id == id => am.category,
        _ => b.dex.move_def(id).category,
    };
    if category != MoveCategory::Status {
        return None;
    }
    let mon = b.ident(attacker);
    b.add(Msg::Cant { mon, reason: taunt_arg(), mv: Some(Arg::new(ArgValue::Move(MoveNum(id.0)))), kw: Kw::NONE });
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::DisableMove, on_disable_move),
    Handler::on(EventId::BeforeMove, on_before_move),
]);
