//! Mind Reader: Lock-On with its own name on the `-activate` line.
//!
//! PS: data/moves.ts mindreader (onTryHit, onHit)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

// PS: data/moves.ts mindreader.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::lockon::hit(b, a, moves::MINDREADER)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, super::lockon::on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
