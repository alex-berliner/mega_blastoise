//! Water Veil: the holder cannot be burned. A status it has anyway (it had it
//! before the ability, by Trace or Skill Swap) is cured at the next
//! `Update` (`-activate|MON|ability: Water Veil`, then `cureStatus`).
//!
//! No ability state.
//!
//! PS: data/abilities.ts waterveil

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{block_status, cure_status_update};

/// The statuses Water Veil keeps out.
const STATUSES: &[ps_dex::ConditionId] = &[c::BRN];

/// `onUpdate(pokemon)`.
// PS: data/abilities.ts waterveil.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    cure_status_update(b, a, STATUSES)
}

/// `onSetStatus(status, target, source, effect)`.
// PS: data/abilities.ts waterveil.onSetStatus
fn on_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    block_status(b, a, STATUSES)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::SetStatus, on_set_status),
]);
