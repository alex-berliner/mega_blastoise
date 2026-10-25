//! Solar Beam: a two-turn move that skips its charge turn in the sun (a
//! handler-less `solarbeam` volatile, from `twoturnmove`, otherwise), and
//! hits with half power in rain, sandstorm and hail. No effect state.
//!
//! PS: data/moves.ts solarbeam

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::{two_turn_try_move, ChargeExtra};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Text, TextId};

/// `onTryMove(attacker, defender, move)`.
// PS: data/moves.ts solarbeam.onTryMove
fn on_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    two_turn_try_move(b, a, ChargeExtra::SolarBeam)
}

/// `onBasePower(basePower, pokemon, target)`: half power in the weak
/// weathers (Primordial Sea and Snowscape are later gens).
// PS: data/moves.ts solarbeam.onBasePower
fn on_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let weather = b.pokemon_effective_weather(pokemon);
    if [conditions::RAINDANCE, conditions::SANDSTORM, conditions::HAIL].contains(&weather) {
        b.debug(Text::new(TextId::WeakenedByWeather));
        b.chain_modify(1, 2);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryMove, on_try_move),
    Handler::on(EventId::BasePower, on_base_power),
]);
