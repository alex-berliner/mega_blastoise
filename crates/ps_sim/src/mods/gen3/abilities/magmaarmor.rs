//! Magma Armor: the holder cannot be frozen (`Immunity` to `frz`, so
//! `runStatusImmunity` refuses it before `SetStatus`). A freeze it has
//! anyway is thawed at the next `Update`.
//!
//! No ability state.
//!
//! PS: data/abilities.ts magmaarmor

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{cure_status_update, relay_is};

/// `onUpdate(pokemon)`.
// PS: data/abilities.ts magmaarmor.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    cure_status_update(b, a, &[c::FRZ])
}

/// `onImmunity(type, pokemon)`: `false` for `frz`.
// PS: data/abilities.ts magmaarmor.onImmunity
fn on_immunity(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_is(a, c::FRZ).then_some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::Immunity, on_immunity),
]);
