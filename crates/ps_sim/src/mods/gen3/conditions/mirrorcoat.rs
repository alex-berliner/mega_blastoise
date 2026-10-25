//! `mirrorcoat`: Mirror Coat's volatile, readied before the turn
//! (`duration: 1`, `noCopy`). It records the last special move other
//! than Hidden Power a foe hit the holder with this turn.
//!
//! Effect state: as `counter`'s ([`super::counter::SLOT`],
//! [`super::counter::DAMAGE`]).
//!
//! PS: data/moves.ts mirrorcoat.condition (`onStart`),
//! data/mods/gen3/moves.ts mirrorcoat.condition (`onRedirectTarget`,
//! `onDamage`)

use ps_dex::MoveCategory;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::counter::{on_redirect_target, on_start, record_damage};

/// `onDamage(damage, target, source, effect)`: a foe's special move that is
/// not Hidden Power.
// PS: data/mods/gen3/moves.ts mirrorcoat.condition.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    record_damage(b, a, |category, hidden_power| category == MoveCategory::Special && !hidden_power)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::RedirectTarget, on_redirect_target).priority(-1),
    Handler::on(EventId::Damage, on_damage).priority(-101),
]);
