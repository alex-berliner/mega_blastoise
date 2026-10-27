//! Protect: the `protect` volatile for the turn, if the user is not the
//! last to act and the `stall` chance holds. Detect and Endure share the
//! two handlers. No effect state.
//!
//! PS: data/moves.ts protect (onPrepareHit, onHit)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::conditions;

/// `onPrepareHit(pokemon)`: `!!this.queue.willAct() &&
/// this.runEvent('StallMove', pokemon)`.
// PS: data/moves.ts protect.onPrepareHit
pub fn stall_prepare_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.queue.will_act().is_none() {
        return Some(Relay::FAIL);
    }
    Some(b.run_event(EventId::StallMove, Holder::Mon(pokemon), None, EffectId::None, None))
}

/// `onHit(pokemon)`: `pokemon.addVolatile('stall')`.
// PS: data/moves.ts protect.onHit
pub fn stall_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.add_volatile(pokemon, conditions::STALL, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::PrepareHit, stall_prepare_hit),
    Handler::on(EventId::Hit, stall_hit),
]);
