//! Suction Cups: the holder cannot be dragged out by another Pokemon.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts suctioncups (onDragOut)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Kw, Msg};

/// `onDragOut(pokemon)`: the `-activate` line, then `null` (no drag out).
fn on_drag_out(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = b.effect_fullname(b.frame.effect);
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DragOut, on_drag_out).priority(1)]);
