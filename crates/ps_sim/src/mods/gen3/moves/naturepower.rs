//! Nature Power: uses Swift (gen 3's battle terrain is always a building).
//! No effect state.
//!
//! PS: data/mods/gen3/moves.ts naturepower.onHit

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target) { this.actions.useMove('swift', target); }`
// PS: data/mods/gen3/moves.ts naturepower.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Nature Power targets its user");
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, moves::SWIFT));
    b.use_move(mv, target, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
