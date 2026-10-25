//! `trapped`: Mean Look, Block and Spider Web's trap (their `onHit` adds
//! it, with a `trapper` volatile on the user).
//!
//! No effect state of its own.
//!
//! PS: data/conditions.ts trapped

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Arg, ArgValue, Kw, Msg, Word};

/// `onTrapPokemon(pokemon)`: `pokemon.tryTrap()`.
// PS: data/conditions.ts trapped.onTrapPokemon
fn on_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.try_trap(pokemon, false);
    None
}

/// `onStart(target)`: the `-activate` line.
// PS: data/conditions.ts trapped.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::Activate { mon: Some(mon), effect: Arg::new(ArgValue::Word(Word::Trapped)), a: None, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TrapPokemon, on_trap_pokemon),
    Handler::on(EventId::Start, on_start),
]);
