//! Toxic's counter: the `residualdmg` volatile, which counts the turns
//! a poisoned Pokemon has spent poisoned. Burn and poison read the
//! counter, so Toxic's damage keeps growing (and keeps growing after a
//! Rest).
//!
//! `this.effectState` is the volatile's own state: `counter` is the
//! number of turns counted so far.
//!
//! PS: data/mods/gen2/conditions.ts residualdmg (line 232)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onStart(target)`: `target.volatiles['residualdmg'].counter = 0`.
// PS: data/mods/gen2/conditions.ts residualdmg.onStart
fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter = 0;
    None
}

/// `onAfterMoveSelf(pokemon)`: the counter grows while the Pokemon is
/// burned, poisoned or toxic.
// PS: data/mods/gen2/conditions.ts residualdmg.onAfterMoveSelf
fn on_after_move_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if [conditions::BRN, conditions::PSN, conditions::TOX].contains(&b.mon(pokemon).status) {
        super::bump_residualdmg_counter(b, pokemon);
    }
    None
}

/// `onAfterSwitchInSelf(pokemon)`: the same count on switch-in.
// PS: data/mods/gen2/conditions.ts residualdmg.onAfterSwitchInSelf
fn on_after_switch_in_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if [conditions::BRN, conditions::PSN, conditions::TOX].contains(&b.mon(pokemon).status) {
        super::bump_residualdmg_counter(b, pokemon);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::AfterMoveSelf, on_after_move_self).priority(100),
    Handler::on(EventId::AfterSwitchInSelf, on_after_switch_in_self),
]);
