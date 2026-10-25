//! Shed Skin: the holder has a chance of curing its status at the end of
//! every turn.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts shedskin (onResidual)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Kw, Msg, Text, TextId};

/// `onResidual(pokemon)`: the `-activate` line, then the status goes.
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.mon(pokemon);
    if mon.hp == 0 || mon.status.is_none() {
        return None;
    }
    if !b.prng.random_chance(33, 100) {
        return None;
    }
    b.debug(Text::new(TextId::ShedSkin));
    let ident = b.ident(pokemon);
    let effect = b.effect_fullname(b.frame.effect);
    b.add(Msg::Activate { mon: Some(ident), effect, a: None, b: None, kw: Kw::NONE });
    // `pokemon.cureStatus()`.
    b.cure_status(pokemon, false);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Residual, on_residual).order(10).sub_order(3)]);
