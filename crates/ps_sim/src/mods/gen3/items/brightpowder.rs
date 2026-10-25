//! `brightpowder`: Bright Powder, the accuracy of moves against the
//! holder times 0.9. The holder is `ModifyAccuracy`'s target, so this is
//! its own `onModifyAccuracy`.
//!
//! The accuracy is a JS double (the stage multipliers make fractions), so
//! the relay is a float ([`Relay::Float`]), whole or not.
//!
//! PS: data/mods/gen4/items.ts brightpowder

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::abilities::relay_number;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyAccuracy(accuracy)`: `if (typeof accuracy !== "number")
/// return; this.debug(...); return accuracy * 0.9`.
// PS: data/mods/gen4/items.ts brightpowder.onModifyAccuracy
fn on_modify_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let accuracy = relay_number(a)?;
    b.debug(Text::new(TextId::BrightPowderAccuracy));
    Some(Relay::float(accuracy * 0.9))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyAccuracy, on_modify_accuracy).priority(5)]);
