//! Gen 3 moves with handlers, one file per move.

pub mod aromatherapy;
pub mod assist;
pub mod attract;
pub mod batonpass;
pub mod beatup;
pub mod bellydrum;
pub mod bide;
pub mod block;
pub mod bounce;
pub mod camouflage;
pub mod conversion2;
pub mod conversion;
pub mod counter;
pub mod covet;
pub mod curse;
pub mod destinybond;
pub mod detect;
pub mod dig;
pub mod disable;
pub mod dive;
pub mod doomdesire;
pub mod dreameater;
pub mod endure;
pub mod fakeout;
pub mod fly;
pub mod focuspunch;
pub mod followme;
pub mod foresight;
pub mod furycutter;
pub mod futuresight;
pub mod haze;
pub mod healbell;
pub mod helpinghand;
pub mod hiddenpower;
pub mod iceball;
pub mod imprison;
pub mod knockoff;
pub mod leechseed;
pub mod lockon;
pub mod meanlook;
pub mod metronome;
pub mod mimic;
pub mod mindreader;
pub mod mirrorcoat;
pub mod mirrormove;
pub mod moonlight;
pub mod morningsun;
pub mod naturepower;
pub mod odorsleuth;
pub mod painsplit;
pub mod perishsong;
pub mod protect;
pub mod psychup;
pub mod pursuit;
pub mod rapidspin;
pub mod razorwind;
pub mod recycle;
pub mod refresh;
pub mod rest;
pub mod roleplay;
pub mod rollout;
pub mod sketch;
pub mod skillswap;
pub mod skullbash;
pub mod skyattack;
pub mod sleeptalk;
pub mod snore;
pub mod solarbeam;
pub mod spiderweb;
pub mod spite;
pub mod spitup;
pub mod splash;
pub mod stockpile;
pub mod struggle;
pub mod substitute;
pub mod swallow;
pub mod synthesis;
pub mod teleport;
pub mod thief;
pub mod transform;
pub mod trick;
pub mod uproar;
pub mod yawn;

#[cfg(test)]
mod tests;

use ps_protocol::{MoveNum, Msg};

use crate::effect::{EffectId, EventArgs, EventId};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use ps_dex::gen3::conditions;

/// `onHit(target, source, move)` of Block, Mean Look and Spider Web:
/// `return target.addVolatile('trapped', source, move, 'trapper')`.
// PS: data/moves.ts block.onHit, meanlook.onHit, spiderweb.onHit
fn trap_on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("a trapping move without a source");
    Some(b.add_volatile_linked(target, conditions::TRAPPED, source, a.effect, conditions::TRAPPER))
}

/// What a two-turn move's `onTryMove` does on its charge turn between the
/// `-prepare` line and `ChargeMove`: nothing, Skull Bash's Defense boost,
/// or Solar Beam's skip in the sun.
pub enum ChargeExtra {
    None,
    SkullBash,
    SolarBeam,
}

/// The `onTryMove(attacker, defender, move)` the two-turn moves share
/// (Razor Wind, Solar Beam, Skull Bash, Sky Attack, Dig, Fly, Dive,
/// Bounce): the second turn removes the move's volatile and hits; the
/// first shows `-prepare`, runs `ChargeMove` and adds `twoturnmove`.
// PS: data/moves.ts razorwind.onTryMove (and the others, the same body)
pub fn two_turn_try_move(b: &mut Battle, a: &EventArgs, extra: ChargeExtra) -> Option<Relay> {
    let attacker = a.target.mon()?;
    let defender: Option<MonRef> = a.source;
    let EffectId::Move(m) = a.effect else { panic!("onTryMove from {:?}", a.effect) };
    let key = b.move_volatile_key(m);
    if b.remove_volatile_key(attacker, key) {
        return None;
    }
    let mon = b.ident(attacker);
    b.add(Msg::Prepare { mon, mv: MoveNum(m.0), target: None });
    match extra {
        ChargeExtra::None => {}
        ChargeExtra::SkullBash => {
            // `this.boost({def: 1}, attacker, attacker, move)`.
            let mut boosts = [0i8; 7];
            boosts[1] = 1;
            b.boost(boosts, attacker, Some(attacker), a.effect);
        }
        ChargeExtra::SolarBeam => {
            // `attacker.effectiveWeather()` (Desolate Land is gen 6).
            if b.pokemon_effective_weather(attacker) == conditions::SUNNYDAY {
                b.attr_last_move_still();
                let target = defender.map(|d| b.ident(d));
                b.add_move(Msg::Anim { mon, mv: MoveNum(m.0), target, kw: ps_protocol::Kw::NONE });
                return None;
            }
        }
    }
    if !b.run_event(EventId::ChargeMove, crate::state::Holder::Mon(attacker), defender, a.effect, None).truthy() {
        return None;
    }
    b.add_volatile(attacker, conditions::TWOTURNMOVE, defender, EffectId::None);
    Some(Relay::Null)
}

/// The `onTry(source, target)` Future Sight and Doom Desire share: the
/// `futuremove` slot condition on the target's slot (a second one fails),
/// the damage rolled now from a plain `moveData` object (`basePower`,
/// `category`, `willCrit: false`, type `???`, no id, no `critRatio`), the
/// condition's `duration: 3`, `move`, `source` and `moveData.damage`, and
/// `-start|USER|MOVE`. Answers `null`: the move itself hits nothing now.
// PS: data/mods/gen4/moves.ts futuresight.onTry, data/mods/gen3/moves.ts
// doomdesire.onTry (the same body)
pub fn future_move_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    use crate::actions::{ActiveMove, FakeMove, GenScripts, MoveData, MoveResult};
    use crate::mods::gen3::conditions::futuremove;
    use crate::mods::gen3::scripts::Gen3Scripts;
    use ps_dex::Type;

    let source = a.target.mon()?;
    let target = a.source.expect("a future move's onTry without a target");
    let EffectId::Move(m) = a.effect else { panic!("future move onTry from {:?}", a.effect) };
    let (side, position) = (target.side, b.mon(target).position);
    if !b.add_slot_condition(side, position, conditions::FUTUREMOVE, None, EffectId::None).truthy() {
        return Some(Relay::FAIL);
    }
    let def = b.dex.move_def(m);
    let data = b.new_active_move(ActiveMove {
        fake: FakeMove::Data,
        base_power: def.base_power,
        type_: Type::None,
        category: def.category,
        ..ActiveMove::EMPTY
    });
    let damage = match Gen3Scripts::get_damage(b, source, target, data, MoveData::Move, true) {
        MoveResult::Int(n) => n,
        other => panic!("a future move's damage roll gave {other:?}"),
    };
    let key = EffectId::Condition(conditions::FUTUREMOVE);
    let st = b.sides[side as usize].slot_conditions[position as usize].get_mut(key).expect("futuremove was just added");
    st.duration = 3;
    st.move_ = m;
    st.source = Some(source);
    st.data[futuremove::MOVE_DAMAGE] = i16::try_from(damage).expect("a future move's damage fits i16");
    let mon = b.ident(source);
    b.add(Msg::StartEffect { mon, effect: ps_protocol::Arg::new(ps_protocol::ArgValue::Move(MoveNum(m.0))), a: None, b: None, kw: ps_protocol::Kw::NONE });
    Some(Relay::Null)
}

/// `onTry(source)` of Sleep Talk and Snore: `source.status === 'slp' ||
/// source.hasAbility('comatose')` (Comatose is gen 7: no gen 3 Pokemon
/// has it).
// PS: data/moves.ts sleeptalk.onTry, snore.onTry
fn try_asleep(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    let comatose = b.dex.ability_id("comatose");
    let asleep = b.mon(source).status == conditions::SLP || comatose.is_some_and(|c| b.has_ability(source, c));
    Some(Relay::Bool(asleep))
}

/// `this.add('-start', pokemon, 'typechange', type)` (Conversion,
/// Conversion 2, Camouflage).
fn add_typechange(b: &mut Battle, pokemon: crate::state::MonRef, t: ps_dex::Type) {
    use ps_protocol::{Arg, Kw, Msg, TypeNum, Word};
    let mon = b.ident(pokemon);
    let effect = Arg::word(Word::TypeChange);
    let a = Some(Arg::ty(TypeNum(t.index())));
    b.add(Msg::StartEffect { mon, effect, a, b: None, kw: Kw::NONE });
}

#[cfg(test)]
mod tests_call;

/// Counter's and Mirror Coat's `beforeTurnCallback(pokemon)`:
/// `pokemon.addVolatile(id)`.
// PS: data/moves.ts counter.beforeTurnCallback, mirrorcoat.beforeTurnCallback
pub fn counter_before_turn(b: &mut Battle, a: &EventArgs, id: ps_dex::ConditionId) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.add_volatile(pokemon, id, None, EffectId::None);
    None
}

/// Counter's and Mirror Coat's `onTry(source)`: `false` without the
/// volatile, or while its `slot` is `null`.
// PS: data/moves.ts counter.onTry, mirrorcoat.onTry
pub fn counter_try(b: &mut Battle, a: &EventArgs, id: ps_dex::ConditionId) -> Option<Relay> {
    use crate::mods::gen3::conditions::counter::SLOT;
    let source = a.target.mon()?;
    match b.mon(source).volatiles.get(EffectId::Condition(id)) {
        None => Some(Relay::FAIL),
        Some(st) if st.data[SLOT] == 0 => Some(Relay::FAIL),
        Some(_) => None,
    }
}

/// Counter's and Mirror Coat's `damageCallback(pokemon)`: `0` without the
/// volatile, else its `damage || 1`.
// PS: data/moves.ts counter.damageCallback, mirrorcoat.damageCallback
pub fn counter_damage(b: &mut Battle, a: &EventArgs, id: ps_dex::ConditionId) -> Option<Relay> {
    use crate::mods::gen3::conditions::counter::damage;
    let pokemon = a.target.mon()?;
    let Some(st) = b.mon(pokemon).volatiles.get(EffectId::Condition(id)) else { return Some(Relay::Int(0)) };
    let damage = damage(st);
    Some(Relay::Int(if damage == 0 { 1 } else { damage }))
}

/// Thief's and Covet's `onAfterHit(target, source, move)`: an empty-handed
/// user (no gems in gen 3) takes the target's item, unless `takeItem`, the
/// item's own `TakeItem` or `setItem` refuses (the item goes back). Thief
/// adds the `-enditem|TARGET|ITEM|[silent]|[from] move: Thief|[of] SOURCE`
/// line; both add `-item|SOURCE|ITEM|[from] move: NAME|[of] TARGET`.
// PS: data/moves.ts thief.onAfterHit, covet.onAfterHit
pub fn steal_item_after_hit(b: &mut Battle, a: &EventArgs, mv: ps_dex::MoveId, end_line: bool) -> Option<Relay> {
    use ps_protocol::{Arg, ArgValue, ItemNum, Kw, KwFlags};
    let target = a.target.mon()?;
    let source = a.source.expect("an item-stealing move's onAfterHit without a source");
    if !b.mon(source).item.is_none() {
        return None;
    }
    let Ok(your_item) = b.take_item(target, Some(source)) else { return None };
    if !trick::item_take_event(b, your_item, target, source, target, a.effect) || !b.set_item(source, your_item, None, EffectId::None) {
        b.mon_mut(target).item = your_item;
        return None;
    }
    let from = Arg::new(ArgValue::MoveEffect(MoveNum(mv.0)));
    let (tgt, src) = (b.ident(target), b.ident(source));
    if end_line {
        let kw = Kw::from_of(from, src).with(KwFlags::SILENT.union(KwFlags::FLAGS_FIRST));
        b.add(Msg::EndItem { mon: tgt, item: ItemNum(your_item.0), kw });
    }
    b.add(Msg::Item { mon: src, item: ItemNum(your_item.0), kw: Kw::from_of(from, tgt) });
    None
}

/// `onHit(pokemon)` of Moonlight, Morning Sun and Synthesis: two thirds
/// of the maximum HP in the sun, a quarter of the base maximum HP in rain,
/// sand or hail, half otherwise (Desolate Land and Primordial Sea are gen
/// 6).
// PS: data/mods/gen4/moves.ts moonlight.onHit, morningsun.onHit,
// synthesis.onHit
fn weather_heal(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let (maxhp, base_maxhp) = (i32::from(b.mon(pokemon).maxhp), i32::from(b.mon(pokemon).base_maxhp));
    let (hp, den) = if b.is_weather(&[conditions::SUNNYDAY]) {
        (maxhp * 2, 3)
    } else if b.is_weather(&[conditions::RAINDANCE, conditions::SANDSTORM, conditions::HAIL]) {
        (base_maxhp, 4)
    } else {
        (base_maxhp, 2)
    };
    // `this.heal(x)` with a fractional `x`: `heal` makes anything in
    // (0, 1] one and truncates the rest, so a positive `x` below 1 heals 1.
    b.heal((hp / den).max(1), None, None, EffectId::None);
    None
}

#[cfg(test)]
mod item_counter_tests;
#[cfg(test)]
mod tests_protect;
