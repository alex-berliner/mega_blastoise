//! Truant: the holder loafs around every other turn (`cant|MON|ability:
//! Truant`). The loafing turn is kept in `pokemon.truantTurn` and shown by
//! the handler-less `truant` volatile; gen 3 starts the count on switch-in
//! (`truantTurn = turn !== 0`) and flips it each residual (order 27).
//!
//! No ability state (`truantTurn` is a Pokemon field,
//! [`crate::state::Pokemon::truant_turn`]).
//!
//! PS: data/mods/gen4/abilities.ts truant (onBeforeMove),
//! data/mods/gen3/abilities.ts truant (onResidual, onSwitchIn)

use ps_dex::gen3::conditions as c;
use ps_protocol::Msg;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

/// Adds or removes the `truant` volatile as `truantTurn` says.
fn sync_volatile(b: &mut Battle, pokemon: MonRef) {
    if b.mon(pokemon).truant_turn {
        b.add_volatile(pokemon, c::TRUANT, None, EffectId::None);
    } else {
        b.remove_volatile(pokemon, c::TRUANT);
    }
}

/// `onBeforeMove(pokemon)` (priority 9).
// PS: data/mods/gen4/abilities.ts truant.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.has_vol(pokemon, c::TRUANT) {
        let mon = b.ident(pokemon);
        let reason = super::this_ability_arg(b);
        b.add(Msg::Cant { mon, reason, mv: None, kw: ps_protocol::Kw::NONE });
        return Some(Relay::FAIL);
    }
    None
}

/// `onResidual(pokemon)` (order 27).
// PS: data/mods/gen3/abilities.ts truant.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.mon_mut(pokemon);
    mon.truant_turn = !mon.truant_turn;
    sync_volatile(b, pokemon);
    None
}

/// `onSwitchIn(pokemon)`.
// PS: data/mods/gen3/abilities.ts truant.onSwitchIn
fn on_switch_in(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let turn = b.turn;
    b.mon_mut(pokemon).truant_turn = turn != 0;
    sync_volatile(b, pokemon);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeMove, on_before_move).priority(9),
    Handler::on(EventId::Residual, on_residual).order(27),
    Handler::on(EventId::SwitchIn, on_switch_in),
]);
