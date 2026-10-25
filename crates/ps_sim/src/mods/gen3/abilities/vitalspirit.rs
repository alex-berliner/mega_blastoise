//! Vital Spirit: the holder cannot be put to sleep. A status it has anyway (it had it
//! before the ability, by Trace or Skill Swap) is cured at the next
//! `Update` (`-activate|MON|ability: Vital Spirit`, then `cureStatus`). Yawn's drowsiness is refused with
//! `-immune` (`null`).
//!
//! No ability state.
//!
//! PS: data/abilities.ts vitalspirit

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, block_status, cure_status_update, relay_is};

/// The statuses Vital Spirit keeps out.
const STATUSES: &[ps_dex::ConditionId] = &[c::SLP];

/// `onUpdate(pokemon)`.
// PS: data/abilities.ts vitalspirit.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    cure_status_update(b, a, STATUSES)
}

/// `onSetStatus(status, target, source, effect)`.
// PS: data/abilities.ts vitalspirit.onSetStatus
fn on_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    block_status(b, a, STATUSES)
}

/// `onTryAddVolatile(status, target)`: Yawn is refused, `-immune` and `null`.
// PS: data/abilities.ts vitalspirit.onTryAddVolatile
fn on_try_add_volatile(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if !relay_is(a, c::YAWN) {
        return None;
    }
    let target = a.target.mon()?;
    add_immune(b, target);
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::SetStatus, on_set_status),
    Handler::on(EventId::TryAddVolatile, on_try_add_volatile),
]);
