//! `imprison`: Imprison's volatile on its user. A foe cannot use a move
//! the user also knows: the moves are disabled hidden (the request shows
//! them, choosing one is refused) and stopped by `BeforeMove`.
//!
//! Effect state: `source` is the Imprison user (`addVolatile` sets it).
//!
//! PS: data/moves.ts imprison.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn imprison_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::IMPRISON.0)))
}

/// `onStart(target)`: `-start|MON|move: Imprison`.
// PS: data/moves.ts imprison.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: imprison_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onFoeDisableMove(pokemon)`: each of the user's moves but Struggle,
/// hidden, and `maybeDisabled`.
// PS: data/moves.ts imprison.condition.onFoeDisableMove
fn on_foe_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = b.effect_state().source.expect("imprison without a source");
    let slots = b.mon(source).move_slots;
    for slot in slots.iter().filter(|s| !s.id.is_none()) {
        if slot.id == moves::STRUGGLE {
            continue;
        }
        b.disable_move(pokemon, slot.id, true, EffectId::None);
    }
    b.mon_mut(pokemon).maybe_disabled = true;
    None
}

/// `onFoeBeforeMove(attacker, defender, move)`: a move the user knows is
/// stopped (`cant|MON|move: Imprison|MOVE`).
// PS: data/moves.ts imprison.condition.onFoeBeforeMove
fn on_foe_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let attacker = a.target.mon()?;
    let EffectId::Move(id) = a.effect else { return None };
    let source = b.effect_state().source.expect("imprison without a source");
    if id == moves::STRUGGLE || !b.has_move(source, id) {
        return None;
    }
    let mon = b.ident(attacker);
    b.add(Msg::Cant { mon, reason: imprison_arg(), mv: Some(Arg::new(ArgValue::Move(MoveNum(id.0)))) });
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::DisableMove, on_foe_disable_move).prefix(Prefix::Foe),
    Handler::on(EventId::BeforeMove, on_foe_before_move).prefix(Prefix::Foe).priority(4),
]);
