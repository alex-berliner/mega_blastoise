//! Belly Drum: the user loses half its maximum HP and its Attack goes to
//! +6. Fails at half HP or less, at +6 already, or with 1 maximum HP. No
//! effect state.
//!
//! PS: data/moves.ts bellydrum.onHit

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHit(target)`.
// PS: data/moves.ts bellydrum.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let (hp, maxhp) = (i32::from(b.mon(target).hp), i32::from(b.mon(target).maxhp));
    // `target.hp <= target.maxhp / 2` (a real division).
    if 2 * hp <= maxhp || b.mon(target).boosts[0] >= 6 || maxhp == 1 {
        return Some(Relay::FAIL);
    }
    // `this.directDamage(target.maxhp / 2)`: `pokemon.damage` truncates.
    b.direct_damage(maxhp / 2, None, None, EffectId::None);
    // `this.boost({atk: 12}, target)`.
    let mut boosts = [0i8; 7];
    boosts[0] = 12;
    b.boost(boosts, target, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
