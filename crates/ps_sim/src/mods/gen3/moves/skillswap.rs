//! Skill Swap: the user and the target trade abilities (`battle.skillSwap`).
//! No effect state.
//!
//! Gens 3-4 announce the swap without the abilities' names; the old
//! abilities' `End` runs, each Pokemon gets a fresh ability state, and the
//! target becomes externally stale (Endless Battle Clause). Gen 3 runs no
//! `Start` for the new abilities.
//!
//! PS: data/moves.ts skillswap.onHit; sim/battle.ts skillSwap

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef};
use ps_dex::gen3::moves;

/// `battle.skillSwap(source, target)`: `false`, a falsy `SetAbility`
/// answer, or `undefined` (`None`) once swapped.
// PS: sim/battle.ts skillSwap
fn skill_swap(b: &mut Battle, source: MonRef, target: MonRef) -> Option<Relay> {
    if b.mon(source).fainted || b.mon(target).fainted {
        return Some(Relay::FAIL);
    }
    // Dynamax is gen 8.
    let (source_ability, target_ability) = (b.mon(source).ability, b.mon(target).ability);
    let flag = |b: &Battle, ab: ps_dex::AbilityId| !ab.is_none() && b.dex.ability_def(ab).flags.has("failskillswap");
    if flag(b, source_ability) || flag(b, target_ability) {
        return Some(Relay::FAIL);
    }
    if b.gen <= 5 && source_ability == target_ability {
        return Some(Relay::FAIL);
    }

    // `dex.conditions.get('skillswap')` is a condition that does not exist
    // and no gen 3 effect answers `SetAbility`, so only its absence is kept.
    let effect_of = |ab: ps_dex::AbilityId| if ab.is_none() { EffectId::None } else { EffectId::Ability(ab) };
    let relay = Some(Relay::Effect(effect_of(source_ability)));
    let target_can_be_set = b.run_event(EventId::SetAbility, Holder::Mon(target), Some(source), EffectId::None, relay);
    if !target_can_be_set.truthy() {
        return Some(target_can_be_set);
    }
    let relay = Some(Relay::Effect(effect_of(target_ability)));
    let source_can_be_set = b.run_event(EventId::SetAbility, Holder::Mon(source), Some(source), EffectId::None, relay);
    if !source_can_be_set.truthy() {
        return Some(source_can_be_set);
    }

    // Gen <= 4 (or allies): no ability names.
    let (mon, of) = (b.ident(source), b.ident(target));
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::SKILLSWAP.0)));
    let empty = Some(Arg::new(ArgValue::Word(Word::Empty)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: empty, b: empty, kw: Kw { of: Some(of), ..Kw::NONE } });

    let (source_state, target_state) = (b.mon(source).ability_state, b.mon(target).ability_state);
    b.single_event(EventId::End, effect_of(source_ability), StateRef::Ability(source), source_state.serial, Holder::Mon(source), None, EffectId::None, None);
    b.single_event(EventId::End, effect_of(target_ability), StateRef::Ability(target), target_state.serial, Holder::Mon(target), None, EffectId::None, None);
    b.mon_mut(source).ability = target_ability;
    b.mon_mut(target).ability = source_ability;
    let st = b.init_effect_state(!target_ability.is_none(), Holder::Mon(source));
    b.mon_mut(source).ability_state = st;
    let st = b.init_effect_state(!source_ability.is_none(), Holder::Mon(target));
    b.mon_mut(target).ability_state = st;
    b.bury(source_state);
    b.bury(target_state);
    b.mon_mut(source).volatile_staleness = false;
    if source.side != target.side {
        b.mon_mut(target).volatile_staleness = true;
    }
    // `Start` of the new abilities: gens 4+.
    None
}

/// `onHit(target, source, move) { return this.skillSwap(source, target); }`
// PS: data/moves.ts skillswap.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Skill Swap hits a Pokemon");
    let source = a.source.expect("Skill Swap has a user");
    skill_swap(b, source, target)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
