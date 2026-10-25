//! Gen 3 abilities with handlers, one file per ability. Each file ports the
//! definition `Dex.mod('gen3')` resolves: `data/mods/gen3/abilities.ts`
//! over `data/mods/gen4/abilities.ts` over the gen 5 to 8 mods over base
//! `data/abilities.ts` (`node tools/ps-diff/show-effect.js gen3 ability
//! <id>` prints it).
//!
//! PS: data/mods/gen3/abilities.ts, data/mods/gen4/abilities.ts,
//! data/abilities.ts.

pub mod airlock;
pub mod arenatrap;
pub mod battlearmor;
pub mod cloudnine;
pub mod colorchange;
pub mod damp;
pub mod drizzle;
pub mod drought;
pub mod flashfire;
pub mod forecast;
pub mod immunity;
pub mod innerfocus;
pub mod insomnia;
pub mod intimidate;
pub mod lightningrod;
pub mod limber;
pub mod magmaarmor;
pub mod magnetpull;
pub mod minus;
pub mod oblivious;
pub mod owntempo;
pub mod plus;
pub mod pressure;
pub mod sandstream;
pub mod shadowtag;
pub mod shellarmor;
pub mod soundproof;
pub mod synchronize;
pub mod trace;
pub mod truant;
pub mod vitalspirit;
pub mod voltabsorb;
pub mod waterabsorb;
pub mod waterveil;
pub mod wonderguard;

use ps_dex::ConditionId;
use ps_protocol::{AbilityNum, Arg, Kw, Msg};

use crate::actions::ActiveMove;
use crate::effect::{EffectId, EventArgs};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

/// `ability: NAME` of the running ability (`this.effect`), as a `[from]`
/// or an effect argument.
pub fn this_ability_arg(b: &Battle) -> Arg {
    match b.frame.effect {
        EffectId::Ability(id) => Arg::ability_effect(AbilityNum(id.0)),
        other => panic!("not an ability handler: {other:?}"),
    }
}

/// `this.effectState.target`: the Pokemon holding the running ability.
pub fn holder(b: &mut Battle) -> Option<MonRef> {
    b.effect_state().target.mon()
}

/// The move a move event (`TryHit`, `Hit`, `DamagingHit`) runs for: the
/// active move when it is the event's effect.
pub fn event_move(b: &Battle, a: &EventArgs) -> Option<ActiveMove> {
    let EffectId::Move(id) = a.effect else { return None };
    b.active_move().filter(|am| am.id == id).copied()
}

/// `this.add('-immune', target, '[from] ability: NAME')`.
pub fn add_immune(b: &mut Battle, target: MonRef) {
    let mon = b.ident(target);
    let kw = Kw::from(this_ability_arg(b));
    b.add(Msg::Immune { mon, what: None, kw });
}

/// The status immunity abilities' `onUpdate(pokemon)`: when the Pokemon
/// has one of `statuses`, `-activate|MON|ability: NAME` and `cureStatus()`.
// PS: data/abilities.ts insomnia, vitalspirit, immunity, limber, magmaarmor,
// waterveil onUpdate
pub fn cure_status_update(b: &mut Battle, a: &EventArgs, statuses: &[ConditionId]) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let status = b.mon(pokemon).status;
    if status.is_none() || !statuses.contains(&status) {
        return None;
    }
    let mon = b.ident(pokemon);
    let effect = this_ability_arg(b);
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    b.cure_status(pokemon, false);
    None
}

/// The status immunity abilities' `onSetStatus(status, target, source,
/// effect)`: a status in `statuses` is refused (`false`), with the
/// `-immune` line when a move whose own status it is (`effect.status`)
/// tried.
// PS: data/abilities.ts insomnia, vitalspirit, immunity, limber, waterveil
// onSetStatus
pub fn block_status(b: &mut Battle, a: &EventArgs, statuses: &[ConditionId]) -> Option<Relay> {
    let status = match a.relay {
        Some(Relay::Effect(EffectId::Condition(c))) => c,
        other => panic!("SetStatus relay {other:?}"),
    };
    if !statuses.contains(&status) {
        return None;
    }
    if !b.move_status(a.effect).is_none() {
        let target = a.target.mon()?;
        add_immune(b, target);
    }
    Some(Relay::FAIL)
}

/// `relay.id === id` for the condition relay of `Immunity` and
/// `TryAddVolatile` (`status.id`, or the type string `runStatusImmunity`
/// passes).
pub fn relay_is(a: &EventArgs, id: ConditionId) -> bool {
    a.relay == Some(Relay::Effect(EffectId::Condition(id)))
}

/// `target.baseMaxhp / 4` as `battle.heal` takes it: a positive fraction
/// up to 1 heals 1 (`if (damage && damage <= 1) damage = 1`), anything
/// more is truncated, which for a positive number is integer division.
pub fn quarter_base_maxhp(b: &Battle, m: MonRef) -> i32 {
    let hp = i32::from(b.mon(m).base_maxhp);
    if hp > 0 { (hp / 4).max(1) } else { 0 }
}

#[cfg(test)]
mod tests;
