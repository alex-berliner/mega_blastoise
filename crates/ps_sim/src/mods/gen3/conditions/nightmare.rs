//! `nightmare`: Nightmare's volatile. A sleeping holder loses a quarter of
//! its max HP at each residual; waking ends it (`cureStatus` removes it).
//! No effect state of its own.
//!
//! PS: data/moves.ts nightmare.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(pokemon)`: asleep only (no Comatose in gen 3), then
/// `-start|MON|Nightmare`.
// PS: data/moves.ts nightmare.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).status != conditions::SLP {
        return Some(Relay::FAIL);
    }
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::NIGHTMARE.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onResidual(pokemon)`: `this.damage(pokemon.baseMaxhp / 4)`.
// PS: data/moves.ts nightmare.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let d = super::fraction_of_base_maxhp(b, pokemon, 4);
    b.damage(d, Some(pokemon), None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(7),
]);
