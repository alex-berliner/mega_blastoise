//! Synchronize: a Pokemon that burns, paralyzes or poisons the holder gets
//! the same status back (bad poison comes back as regular poison; sleep and
//! freeze do not), `-status ... [from] ability: Synchronize|[of] HOLDER`.
//!
//! No ability state.
//!
//! PS: data/mods/gen4/abilities.ts synchronize

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onAfterSetStatus(status, target, source, effect)`:
/// `source.trySetStatus(id, target)`.
// PS: data/mods/gen4/abilities.ts synchronize.onAfterSetStatus
fn on_after_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    if source == target {
        return None;
    }
    // `effect?.id === 'toxicspikes'`: Toxic Spikes is gen 4.
    if let EffectId::Move(m) = a.effect {
        if b.dex.move_id("toxicspikes") == Some(m) {
            return None;
        }
    }
    let mut id = match a.relay {
        Some(Relay::Effect(EffectId::Condition(s))) => s,
        other => panic!("AfterSetStatus relay {other:?}"),
    };
    if id == c::SLP || id == c::FRZ {
        return None;
    }
    if id == c::TOX {
        id = c::PSN;
    }
    b.try_set_status(source, id, Some(target), EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::AfterSetStatus, on_after_set_status)]);
