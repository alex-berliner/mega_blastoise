//! Tri Attack: its secondary's `onHit` tries a random one of burn,
//! paralysis and freeze. The secondary has no id of its own: the handler
//! is found through [`super::secondary_handler`], not the registry.
//!
//! PS: data/mods/gen3/moves.ts triattack.secondary.onHit (base's body)

use crate::effect::{EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onHit(target, source)`: `target.trySetStatus(this.sample(['brn',
/// 'par', 'frz']), source)`.
// PS: data/mods/gen3/moves.ts triattack.secondary.onHit
fn secondary_on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let status = *b.prng.sample(&[conditions::BRN, conditions::PAR, conditions::FRZ]);
    b.try_set_status(target, status, a.source, EffectId::None);
    None
}

pub static SECONDARY_ON_HIT: Handler = Handler::on(EventId::Hit, secondary_on_hit);
