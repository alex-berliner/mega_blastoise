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
pub mod blaze;
pub mod chlorophyll;
pub mod clearbody;
pub mod cloudnine;
pub mod colorchange;
pub mod cutecharm;
pub mod compoundeyes;
pub mod damp;
pub mod drizzle;
pub mod drought;
pub mod effectspore;
pub mod flamebody;
pub mod flashfire;
pub mod forecast;
pub mod guts;
pub mod hugepower;
pub mod hustle;
pub mod hypercutter;
pub mod immunity;
pub mod innerfocus;
pub mod insomnia;
pub mod intimidate;
pub mod keeneye;
pub mod lightningrod;
pub mod limber;
pub mod liquidooze;
pub mod magmaarmor;
pub mod magnetpull;
pub mod marvelscale;
pub mod minus;
pub mod naturalcure;
pub mod oblivious;
pub mod overgrow;
pub mod owntempo;
pub mod plus;
pub mod poisonpoint;
pub mod pressure;
pub mod purepower;
pub mod raindish;
pub mod rockhead;
pub mod roughskin;
pub mod sandstream;
pub mod sandveil;
pub mod serenegrace;
pub mod shadowtag;
pub mod shedskin;
pub mod shellarmor;
pub mod shielddust;
pub mod soundproof;
pub mod speedboost;
pub mod r#static;
pub mod stickyhold;
pub mod sturdy;
pub mod suctioncups;
pub mod swarm;
pub mod swiftswim;
pub mod synchronize;
pub mod thickfat;
pub mod torrent;
pub mod trace;
pub mod truant;
pub mod vitalspirit;
pub mod voltabsorb;
pub mod waterabsorb;
pub mod waterveil;
pub mod whitesmoke;
pub mod wonderguard;

use ps_dex::ConditionId;
use ps_protocol::{AbilityNum, Arg, ArgValue, Kw, Msg, Stat, Word};

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

/// PS's `onDamagingHit(damage, target, source, move)`'s guard: the hit
/// did damage and the move made contact. The holder and the attacker.
pub fn contact_hit(b: &Battle, a: &EventArgs) -> Option<(MonRef, MonRef)> {
    let damage = a.relay?.as_int()?;
    if damage == 0 {
        return None;
    }
    let target = a.target.mon()?;
    let source = a.source?;
    let EffectId::Move(m) = a.effect else { return None };
    if !b.dex.move_def(m).flags.has("contact") {
        return None;
    }
    Some((target, source))
}

/// The number a handler sees as its relay, when PS's is a number
/// (`typeof accuracy === "number"`): the accuracy travels as a float.
pub fn relay_number(a: &EventArgs) -> Option<f64> {
    match a.relay? {
        Relay::Float(bits) => Some(f64::from_bits(bits)),
        Relay::Int(n) => Some(f64::from(n)),
        _ => None,
    }
}

/// PS's `effect.secondaries`: whether the effect (a move) has secondary
/// effects.
pub fn effect_has_secondaries(b: &Battle, effect: EffectId) -> bool {
    match effect {
        EffectId::Move(m) => !b.dex.move_def(m).secondaries().is_empty(),
        _ => false,
    }
}

/// The index of a stat in the boosts table (`atk`, `def`, `spa`, `spd`,
/// `spe`, `accuracy`, `evasion`).
pub fn stat_index(stat: Stat) -> usize {
    Stat::ALL.iter().position(|s| *s == stat).expect("a stat in Stat::ALL")
}

/// PS's `onTryBoost(boost, target, source, effect)` of Keen Eye and
/// Hyper Cutter: the negative boost of one stat is dropped, and a
/// `-fail` line says so (unless the effect has secondaries).
pub fn block_stat_drop(b: &mut Battle, a: &EventArgs, stat: Stat) -> Option<Relay> {
    let target = a.target.mon()?;
    // `if (source && target === source) return;`
    if a.source.is_some_and(|source| source == target) {
        return None;
    }
    let Relay::Boosts(boosts) = a.relay? else { return None };
    let i = stat_index(stat);
    if boosts[i] >= 0 {
        return None;
    }
    let mut table = boosts;
    // `delete boost[stat]`: the key leaves the table.
    table[i] = 0;
    if !effect_has_secondaries(b, a.effect) {
        let mon = b.ident(target);
        let kw = Kw::from_of(b.effect_fullname(b.frame.effect), mon);
        b.add(Msg::Fail {
            mon,
            what: Some(Arg::new(ArgValue::Word(Word::Unboost))),
            what2: Some(Arg::new(ArgValue::Stat(stat))),
            kw,
        });
    }
    Some(Relay::Boosts(table))
}

/// PS's `onTryBoost(boost, target, source, effect)` of Clear Body and
/// White Smoke: every negative boost is dropped, and a `-fail` line says
/// so (unless the effect has secondaries).
pub fn block_all_drops(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    // `if (source && target === source) return;`
    if a.source.is_some_and(|source| source == target) {
        return None;
    }
    let Relay::Boosts(boosts) = a.relay? else { return None };
    let mut table = boosts;
    let mut show_msg = false;
    for value in table.iter_mut() {
        if *value < 0 {
            *value = 0;
            show_msg = true;
        }
    }
    // `effect.id !== "octolock"`: Octolock is a gen 8 move.
    if show_msg && !effect_has_secondaries(b, a.effect) {
        let mon = b.ident(target);
        let kw = Kw::from_of(b.effect_fullname(b.frame.effect), mon);
        b.add(Msg::Fail { mon, what: Some(Arg::new(ArgValue::Word(Word::Unboost))), what2: None, kw });
    }
    Some(Relay::Boosts(table))
}

#[cfg(test)]
mod tests;
