//! Compound Eyes: the accuracy of the holder's moves is 1.3 times as high.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts compoundeyes (onSourceModifyAccuracy),
//! data/mods/gen4/abilities.ts compoundeyes (onSourceModifyAccuracy)

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Text, TextId};

use super::relay_number;

/// `onSourceModifyAccuracy(accuracy)`: `this.chainModify(1.3)`.
fn on_source_modify_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // `if (typeof accuracy !== "number") return;`
    relay_number(a)?;
    b.debug(Text::new(TextId::CompoundEyesAccuracy));
    b.chain_modify_f(1.3);
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::ModifyAccuracy, on_source_modify_accuracy).prefix(Prefix::Source).priority(9)]);
