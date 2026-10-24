//! Dream Eater: only a sleeping target can be eaten. The move fails with
//! `|-immune|` when the target is awake.
//!
//! No effect state.
//!
//! PS: data/mods/gen4/moves.ts dreameater (line 375), inherited by
//! data/mods/gen1/moves.ts (gen 1 has no override).

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onTryImmunity(target)`: `target.status === 'slp' &&
/// !target.volatiles['substitute']`.
// PS: data/mods/gen4/moves.ts dreameater.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let status = b.mon(target).status;
    let asleep = !status.is_none() && b.dex.condition_def(status).key.as_str() == "slp";
    let sub = b.mon(target).volatiles.contains(EffectId::Condition(conditions::SUBSTITUTE));
    Some(Relay::Bool(asleep && !sub))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryImmunity, on_try_immunity)]);
