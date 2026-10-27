//! Refresh: cures the user's burn, poison or paralysis (not sleep or
//! freeze). No effect state.
//!
//! PS: data/moves.ts refresh.onHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onHit(pokemon)`: fails with no status, sleep or freeze; otherwise
/// `cureStatus()`.
// PS: data/moves.ts refresh.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let status = b.mon(pokemon).status;
    if status.is_none() || status == conditions::SLP || status == conditions::FRZ {
        return Some(Relay::FAIL);
    }
    b.cure_status(pokemon, false);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
