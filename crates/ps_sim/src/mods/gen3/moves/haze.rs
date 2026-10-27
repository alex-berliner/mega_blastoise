//! Haze (gen 3): every active Pokemon's stat stages go back to 0. No
//! effect state.
//!
//! PS: data/mods/gen3/moves.ts haze.onHitField

use ps_protocol::{Kw, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHitField()`: `-clearallboost`, then `clearBoosts()` on each of
/// `getAllActive()`.
// PS: data/mods/gen3/moves.ts haze.onHitField
fn on_hit_field(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::ClearAllBoost { kw: Kw::NONE });
    let (active, n) = b.all_active(false);
    for &pokemon in &active[..n] {
        b.clear_boosts(pokemon);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::HitField, on_hit_field)]);
