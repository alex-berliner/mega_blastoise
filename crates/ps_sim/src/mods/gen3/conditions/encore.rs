//! `encore`: Encore's volatile. For 3 to 6 turns the holder's chosen move
//! is replaced by its last move (`OverrideAction`) and its other moves are
//! disabled; it ends early when that move runs out of PP.
//!
//! Effect state: `move` is `move_`.
//!
//! PS: data/mods/gen4/moves.ts encore.condition.onStart; data/moves.ts
//! encore.condition (onOverrideAction, onResidual, onEnd, onDisableMove);
//! data/mods/gen3/moves.ts encore.condition.durationCallback

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use ps_dex::gen3::{conditions, moves};
use ps_dex::MoveId;

fn encore_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::ENCORE.0)))
}

/// `pokemon.getMoveData(move)`: the first slot holding the move.
fn move_data(b: &Battle, pokemon: MonRef, mv: MoveId) -> Option<crate::state::MoveSlot> {
    b.mon(pokemon).move_slots.iter().copied().find(|s| !s.id.is_none() && s.id == mv)
}

/// `durationCallback()`: `this.random(3, 7)`.
// PS: data/mods/gen3/moves.ts encore.condition.durationCallback
fn duration_callback(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(b.prng.random_range(3, 7) as i32))
}

/// `onStart(target, source)`: fails without a last move, with a
/// `failencore` move or with its PP gone; else `-start|MON|Encore`, and
/// an encored Pursuit gets its switch-out volatile.
// PS: data/mods/gen4/moves.ts encore.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let last = b.mon(target).last_move;
    let move_slot = if last.is_none() { None } else { move_data(b, target, last) };
    let Some(slot) = move_slot.filter(|s| !b.dex.move_def(last).flags.has("failencore") && s.pp > 0) else {
        return Some(Relay::FAIL);
    };
    b.effect_state().move_ = slot.id;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: encore_arg(), a: None, b: None, kw: Kw::NONE });
    if slot.id == moves::PURSUIT {
        // `target.addVolatile('pursuit', target, getActiveMove('pursuit'))`.
        b.require_ported(EffectId::Condition(conditions::PURSUIT));
        b.add_volatile(target, conditions::PURSUIT, Some(target), EffectId::Move(moves::PURSUIT));
    }
    None
}

/// `onOverrideAction(pokemon, target, move)`: the encored move instead.
// PS: data/moves.ts encore.condition.onOverrideAction
fn on_override_action(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let encored = b.effect_state().move_;
    match a.effect {
        EffectId::Move(id) if id == encored => None,
        _ => Some(Relay::Move(encored)),
    }
}

/// `onResidual(target)`: ends once the encored move has no PP left (or is
/// gone).
// PS: data/moves.ts encore.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let encored = b.effect_state().move_;
    if move_data(b, target, encored).is_none_or(|s| s.pp <= 0) {
        b.remove_volatile(target, conditions::ENCORE);
    }
    None
}

/// `onEnd(target)`: `-end|MON|Encore`.
// PS: data/moves.ts encore.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: encore_arg(), kw: Kw::NONE });
    None
}

/// `onDisableMove(pokemon)`: every other move is disabled, while the
/// holder still has the encored one.
// PS: data/moves.ts encore.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let encored = b.effect_state().move_;
    if encored.is_none() || !b.has_move(pokemon, encored) {
        return None;
    }
    let slots = b.mon(pokemon).move_slots;
    for slot in slots.iter().filter(|s| !s.id.is_none()) {
        if slot.id != encored {
            b.disable_move(pokemon, slot.id, false, EffectId::None);
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::OverrideAction, on_override_action),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(14),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::DisableMove, on_disable_move),
    Handler::on(EventId::DurationCallback, duration_callback),
]);
