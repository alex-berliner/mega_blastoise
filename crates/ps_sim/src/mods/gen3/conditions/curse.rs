//! `curse`: a Ghost's Curse. The holder loses a quarter of its max HP at
//! each residual. No effect state of its own.
//!
//! PS: data/moves.ts curse.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onStart(pokemon, source)`: `-start|MON|Curse|[of] SOURCE`.
// PS: data/moves.ts curse.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = a.source.expect("curse without a source");
    let mon = b.ident(pokemon);
    let of = b.ident(source);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::CURSE.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    None
}

/// `onResidual(pokemon)`: `this.damage(pokemon.baseMaxhp / 4)`.
// PS: data/moves.ts curse.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let d = super::fraction_of_base_maxhp(b, pokemon, 4);
    b.damage(d, Some(pokemon), None, crate::effect::EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(8),
]);
