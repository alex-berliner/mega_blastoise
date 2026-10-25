//! `disable`: Disable's volatile. The holder's last move is disabled for
//! 2 to 5 turns (one more when the holder has not moved yet this turn),
//! and a use of it already queued is stopped by `BeforeMove`.
//!
//! Effect state: `move` is `move_`.
//!
//! PS: data/mods/gen4/moves.ts disable.condition.onStart; data/moves.ts
//! disable.condition (onEnd, onBeforeMove, onDisableMove);
//! data/mods/gen3/moves.ts disable.condition.durationCallback

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn disable_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::DISABLE.0)))
}

/// `durationCallback()`: `this.random(2, 6)`.
// PS: data/mods/gen3/moves.ts disable.condition.durationCallback
fn duration_callback(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(b.prng.random_range(2, 6) as i32))
}

/// `onStart(pokemon)`: fails without a last move or with its PP gone,
/// else `-start|MON|Disable|MOVE` and the move is kept.
// PS: data/mods/gen4/moves.ts disable.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.will_move(pokemon).is_none() {
        b.effect_state().duration += 1;
    }
    let last = b.mon(pokemon).last_move;
    if last.is_none() {
        return Some(Relay::FAIL);
    }
    let slots = b.mon(pokemon).move_slots;
    for slot in slots.iter().filter(|s| !s.id.is_none()) {
        if slot.id == last {
            if slot.pp == 0 {
                return Some(Relay::FAIL);
            }
            let mon = b.ident(pokemon);
            let mv = Arg::new(ArgValue::Move(MoveNum(slot.id.0)));
            b.add(Msg::StartEffect { mon, effect: disable_arg(), a: Some(mv), b: None, kw: Kw::NONE });
            b.effect_state().move_ = last;
            return None;
        }
    }
    Some(Relay::FAIL)
}

/// `onEnd(pokemon)`: `-end|MON|Disable`.
// PS: data/moves.ts disable.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: disable_arg(), kw: Kw::NONE });
    None
}

/// `onBeforeMove(attacker, defender, move)`: the disabled move is stopped
/// (`cant|MON|Disable|MOVE`). No Z or Max moves in gen 3.
// PS: data/moves.ts disable.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let attacker = a.target.mon()?;
    let EffectId::Move(id) = a.effect else { return None };
    if id != b.effect_state().move_ {
        return None;
    }
    let mon = b.ident(attacker);
    b.add(Msg::Cant { mon, reason: disable_arg(), mv: Some(Arg::new(ArgValue::Move(MoveNum(id.0)))) });
    Some(Relay::FAIL)
}

/// `onDisableMove(pokemon)`: disables the slot of the kept move.
// PS: data/moves.ts disable.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mv = b.effect_state().move_;
    let slots = b.mon(pokemon).move_slots;
    for slot in slots.iter() {
        if !slot.id.is_none() && slot.id == mv {
            b.disable_move(pokemon, slot.id, false, EffectId::None);
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::BeforeMove, on_before_move).priority(7),
    Handler::on(EventId::DisableMove, on_disable_move),
    Handler::on(EventId::DurationCallback, duration_callback),
]);
