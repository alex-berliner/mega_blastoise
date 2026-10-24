//! Toxic: the `-status` line, Toxic's counter (the `residualdmg`
//! volatile), the poison damage it deals and the plain poison a
//! switch-in leaves behind.
//!
//! The toxic status keeps no state of its own: the counter lives on the
//! Pokemon as the `residualdmg` volatile (see
//! [`crate::mods::gen1::conditions::residualdmg`]).
//!
//! PS: data/mods/gen2/conditions.ts tox (line 94), inherited by
//! data/mods/gen1/conditions.ts tox (line 126), which lowers
//! `onAfterMoveSelfPriority` to 2.

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;
use ps_protocol::{Kw, KwFlags, Msg, Status};

use super::{residual_damage, residualdmg_counter, residualdmg_state};

/// `onStart(target)`: the `-status` line and Toxic's counter.
// PS: data/mods/gen2/conditions.ts tox.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::SetStatus { mon, status: Status::Tox, kw: Kw::NONE });
    if residualdmg_state(b, target).is_none() {
        b.add_volatile(target, conditions::RESIDUALDMG, None, EffectId::None);
    }
    // `target.volatiles['residualdmg'].counter = 0`.
    if let Some(s) = b.mon_mut(target).volatiles.get_mut(EffectId::Condition(conditions::RESIDUALDMG)) {
        s.counter = 0;
    }
    None
}

/// `onAfterMoveSelf(pokemon)`: the poison damage, Toxic's counter times.
// PS: data/mods/gen2/conditions.ts tox.onAfterMoveSelf
fn on_after_move_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let damage = residual_damage(b, pokemon) * residualdmg_counter(b, pokemon);
    // `this.damage(damage, pokemon, pokemon)`: the Pokemon is its own
    // source, so the line carries no `[of]`.
    b.damage(damage, Some(pokemon), Some(pokemon), EffectId::None);
    None
}

/// `onSwitchIn(pokemon)`: a switch-in leaves plain poison behind.
// PS: data/mods/gen2/conditions.ts tox.onSwitchIn
fn on_switch_in(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // PS writes the status straight into the Pokemon (`pokemon.status =
    // 'psn'`), so the tox state stays but the tox handlers stop running.
    b.mon_mut(pokemon).status = conditions::PSN;
    let mon = b.ident(pokemon);
    b.add(Msg::SetStatus { mon, status: Status::Psn, kw: Kw::flags(KwFlags::SILENT) });
    None
}

/// `onAfterSwitchInSelf(pokemon)`: the poison damage on switch-in.
// PS: data/mods/gen2/conditions.ts tox.onAfterSwitchInSelf
fn on_after_switch_in_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let damage = residual_damage(b, pokemon);
    b.damage(damage, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::AfterMoveSelf, on_after_move_self).priority(2),
    Handler::on(EventId::SwitchIn, on_switch_in),
    Handler::on(EventId::AfterSwitchInSelf, on_after_switch_in_self),
]);
