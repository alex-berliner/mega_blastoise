//! Burn: the `-status` line, the burn damage at the end of the turn and
//! the burn damage on switch-in.
//!
//! The burn keeps no state of its own. Toxic's counter, when the Pokemon
//! has one, is read from the `residualdmg` volatile (see
//! [`crate::mods::gen1::conditions::residualdmg`]).
//!
//! PS: data/mods/gen1/conditions.ts brn (line 12)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Kw, Msg, Status, TextId};

use super::{residual_damage, residualdmg_counter, residualdmg_state};

/// `onStart(target)`: `this.add('-status', target, 'brn')`.
// PS: data/mods/gen1/conditions.ts brn.onStart (line 14)
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::SetStatus { mon, status: Status::Brn, kw: Kw::NONE });
    None
}

/// `onAfterMoveSelf(pokemon)`: the burn damage, Toxic's counter included.
// PS: data/mods/gen1/conditions.ts brn.onAfterMoveSelf (line 20)
fn on_after_move_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let toxic_counter = if residualdmg_state(b, pokemon).is_some() {
        residualdmg_counter(b, pokemon)
    } else {
        1
    };
    let damage = residual_damage(b, pokemon) * toxic_counter;
    // `this.damage(damage, pokemon)`: the target is the Pokemon, the
    // source the running event's source and the effect the burn.
    b.damage(damage, Some(pokemon), None, EffectId::None);
    if residualdmg_state(b, pokemon).is_some() {
        b.hint(TextId::HintGen1ToxicCounter, true, None);
    }
    None
}

/// `onAfterSwitchInSelf(pokemon)`: the burn damage on switch-in.
// PS: data/mods/gen1/conditions.ts brn.onAfterSwitchInSelf (line 25)
fn on_after_switch_in_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let damage = residual_damage(b, pokemon);
    // `this.damage(damage)`: the target is the running event's target, the
    // source none (a switch-in event has none).
    b.damage(damage, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::AfterMoveSelf, on_after_move_self).priority(2),
    Handler::on(EventId::AfterSwitchInSelf, on_after_switch_in_self),
]);
