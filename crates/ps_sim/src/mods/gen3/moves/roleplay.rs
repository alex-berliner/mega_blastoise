//! Role Play: the user copies the target's ability (`setAbility`, which
//! writes `-ability|USER|NEW|OLD|[from] move: Role Play|[of] TARGET`). No
//! effect state.
//!
//! PS: data/moves.ts roleplay.onTryHit, roleplay.onHit

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryHit(target, source)`: fails for the same ability, a
/// `failroleplay` target ability or a `cantsuppress` user ability.
// PS: data/moves.ts roleplay.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("Role Play has a user");
    let (ta, sa) = (b.mon(target).ability, b.mon(source).ability);
    if ta == sa {
        return Some(Relay::FAIL);
    }
    let flag = |b: &Battle, ab: ps_dex::AbilityId, f: &str| !ab.is_none() && b.dex.ability_def(ab).flags.has(f);
    if flag(b, ta, "failroleplay") || flag(b, sa, "cantsuppress") {
        return Some(Relay::FAIL);
    }
    None
}

/// `onHit(target, source)`: `setAbility(target.ability, target)`; a falsy
/// result is returned.
// PS: data/moves.ts roleplay.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Role Play hits a Pokemon");
    let source = a.source.expect("Role Play has a user");
    let ability = b.mon(target).ability;
    match b.set_ability(source, ability, Some(target), EffectId::None, false, false) {
        // The old ability's id: `''` is falsy, but every gen 3 Pokemon has
        // an ability.
        Relay::Effect(old) => {
            assert!(!old.is_none(), "unported: Role Play from a Pokemon with no ability");
            None
        }
        falsy => Some(falsy),
    }
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
