//! Sand Veil: the holder takes no sandstorm damage, and moves against it
//! are less accurate in a sandstorm.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts sandveil (onImmunity), data/mods/gen4/abilities.ts
//! sandveil (onModifyAccuracy)

use ps_dex::gen3::conditions as c;
use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{relay_is, relay_number};

/// `onImmunity(type, pokemon)`: `if (type === "sandstorm") return false`.
fn on_immunity(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_is(a, c::SANDSTORM).then_some(Relay::FAIL)
}

/// `onModifyAccuracy(accuracy)`: `if (typeof accuracy !== "number")
/// return`, then in a sandstorm the debug line and `chainModify(0.8)`.
fn on_modify_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_number(a)?;
    if !b.is_weather(&[c::SANDSTORM]) {
        return None;
    }
    b.debug(Text::new(TextId::SandVeilAccuracy));
    b.chain_modify_f(0.8);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Immunity, on_immunity),
    Handler::on(EventId::ModifyAccuracy, on_modify_accuracy).priority(8),
]);
