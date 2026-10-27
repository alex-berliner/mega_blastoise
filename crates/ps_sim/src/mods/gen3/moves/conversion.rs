//! Conversion: the user becomes the type of a random one of its moves'
//! types it does not have (Curse left out; duplicates kept, as PS's list
//! keeps them). No effect state.
//!
//! PS: data/mods/gen3/moves.ts conversion.onHit

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target)`.
// PS: data/mods/gen3/moves.ts conversion.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Conversion targets its user");
    let mut possible = [Type::None; 4];
    let mut n = 0;
    let slots = b.mon(target).move_slots;
    for ms in slots.iter().filter(|s| !s.id.is_none()) {
        let t = b.dex.move_def(ms.id).type_;
        if ms.id != moves::CURSE && !b.has_type(target, t) {
            possible[n] = t;
            n += 1;
        }
    }
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let t = *b.prng.sample(&possible[..n]);
    if !b.set_type(target, t, false) {
        return Some(Relay::FAIL);
    }
    super::add_typechange(b, target, t);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
