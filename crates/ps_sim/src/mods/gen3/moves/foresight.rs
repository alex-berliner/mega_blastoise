//! Foresight: `onTryHit` fails against Miracle Eye (gen 4, so never in gen
//! 3); the volatile does the rest (`conditions::foresight`). Odor Sleuth
//! shares the handler and the volatile.
//!
//! PS: data/moves.ts foresight.onTryHit, odorsleuth.onTryHit

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryHit(target)`: `false` with a `miracleeye` volatile.
// PS: data/moves.ts foresight.onTryHit
pub fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let miracleeye = b.dex.condition_id("miracleeye");
    if miracleeye.is_some_and(|c| b.mon(target).volatiles.contains(EffectId::Condition(c))) {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
