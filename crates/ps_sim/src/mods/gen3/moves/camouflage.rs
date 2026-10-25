//! Camouflage: the user becomes Normal-type (gen 3's battle terrain is
//! always a building); it fails for a Normal-type user. No effect state.
//!
//! PS: data/mods/gen4/moves.ts camouflage.onHit

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHit(target)`.
// PS: data/mods/gen4/moves.ts camouflage.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Camouflage targets its user");
    if b.has_type(target, Type::Normal) || !b.set_type(target, Type::Normal, false) {
        return Some(Relay::FAIL);
    }
    super::add_typechange(b, target, Type::Normal);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
