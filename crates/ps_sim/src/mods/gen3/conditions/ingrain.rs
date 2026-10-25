//! `ingrain`: Ingrain's volatile. The holder heals a sixteenth of its base
//! maximum HP each turn, cannot switch out and cannot be dragged out. No
//! effect state.
//!
//! PS: data/moves.ts ingrain.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn ingrain_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::INGRAIN.0)))
}

/// `onStart(pokemon)`: `-start|MON|move: Ingrain`.
// PS: data/moves.ts ingrain.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: ingrain_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onResidual(pokemon)`: `this.heal(pokemon.baseMaxhp / 16)`; `heal`
/// makes anything in (0, 1] one and truncates the rest.
// PS: data/moves.ts ingrain.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let amount = (i32::from(b.mon(pokemon).base_maxhp) / 16).max(1);
    b.heal(amount, None, None, EffectId::None);
    None
}

/// `onTrapPokemon(pokemon)`: `pokemon.tryTrap()`.
// PS: data/moves.ts ingrain.condition.onTrapPokemon
fn on_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.try_trap(pokemon, false);
    None
}

/// `onDragOut(pokemon)`: `-activate|MON|move: Ingrain`, then `null`.
// PS: data/moves.ts ingrain.condition.onDragOut
fn on_drag_out(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::Activate { mon: Some(mon), effect: ingrain_arg(), a: None, b: None, kw: Kw::NONE });
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(1),
    Handler::on(EventId::TrapPokemon, on_trap_pokemon),
    Handler::on(EventId::DragOut, on_drag_out),
]);
