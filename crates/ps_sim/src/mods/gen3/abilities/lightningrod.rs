//! Lightning Rod (gen 3): draws the foes' Electric moves to the holder.
//! PS asks `RedirectTarget` only when a side has more than one active
//! Pokemon (`getMoveTargets`, `activePerHalf > 1`), so in singles the
//! handler never runs; it is ported for the registry, and the doubles-only
//! return of a Pokemon (no relay kind here carries one) stops `unported`.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts lightningrod

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onFoeRedirectTarget(target, source, source2, move)`.
// PS: data/mods/gen3/abilities.ts lightningrod.onFoeRedirectTarget
fn on_foe_redirect_target(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let crate::effect::EffectId::Move(mv) = a.effect else { return None };
    if b.dex.move_def(mv).type_ != Type::Electric {
        return None;
    }
    let holder = super::holder(b)?;
    let source = a.source?;
    let target_type = b.dex.move_def(mv).target;
    let loc = b.get_loc_of(source, holder);
    if b.valid_target_loc(loc, source, Some(target_type)) {
        panic!("unported: RedirectTarget returning a Pokemon (doubles only)");
    }
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::RedirectTarget, on_foe_redirect_target).prefix(Prefix::Foe)]);
