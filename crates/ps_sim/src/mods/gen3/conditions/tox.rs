//! Toxic poison: the `-status` line and the growing poison damage at the
//! end of the turn; the count starts over on a switch-in.
//!
//! `this.effectState.stage` is the status state's `counter`.
//!
//! PS: data/conditions.ts tox

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::Status;

use super::add_status_line;

/// `onStart(target, source, sourceEffect)`: the stage and the `-status`
/// line (no Toxic Orb in gen 3).
// PS: data/conditions.ts tox.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter = 0;
    add_status_line(b, a, Status::Tox)
}

/// `onSwitchIn()`: `this.effectState.stage = 0`.
// PS: data/conditions.ts tox.onSwitchIn
fn on_switch_in(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter = 0;
    None
}

/// `onResidual(pokemon)`: the stage goes up to 15, then
/// `this.damage(this.clampIntRange(pokemon.baseMaxhp / 16, 1) * stage)`.
// PS: data/conditions.ts tox.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let stage = {
        let st = b.effect_state();
        if st.counter < 15 {
            st.counter += 1;
        }
        i32::from(st.counter)
    };
    // `clampIntRange` floors: `max(floor(baseMaxhp / 16), 1) * stage`.
    let damage = super::fraction_of_base_maxhp(b, pokemon, 16) * stage;
    b.damage(damage, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::SwitchIn, on_switch_in),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(6),
]);
