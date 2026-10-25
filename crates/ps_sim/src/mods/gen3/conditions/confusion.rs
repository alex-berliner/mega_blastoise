//! Confusion: the `-start` line, the turn counter and the self-hit.
//!
//! `this.effectState.time` (the turns of confusion left) is `data[0]`.
//!
//! PS: data/conditions.ts confusion (onStart, onEnd),
//! data/mods/gen4/conditions.ts confusion (onBeforeMove)

use crate::actions::{ActiveMove, FakeMove, GenScripts, MoveData, MoveResult};
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Pseudo};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_dex::{MoveCategory, Type};
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, Msg, Word};

/// `this.effectState.time`.
const TIME: usize = 0;

/// `onStart(target, source, sourceEffect)`: the `-start` line and 2-5
/// turns (Axe Kick's 3-5 is gen 9).
// PS: data/conditions.ts confusion.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let kw = match (a.effect, a.source) {
        (EffectId::Condition(c), _) if c == conditions::LOCKEDMOVE => Kw::flags(KwFlags::FATIGUE),
        (EffectId::Ability(_), Some(source)) => Kw::from_of(b.effect_fullname(a.effect), b.ident(source)),
        _ => Kw::NONE,
    };
    let effect = Arg::new(ArgValue::Word(Word::Confusion));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw });
    let time = b.prng.random_range(2, 6) as i16;
    b.effect_state().data[TIME] = time;
    None
}

/// `onEnd(target)`: the `-end` line.
// PS: data/conditions.ts confusion.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::Word(Word::Confusion));
    b.add(Msg::EndEffect { mon, effect, kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon)`: the counter, then half the time a 40 power
/// typeless physical hit on itself.
// PS: data/mods/gen4/conditions.ts confusion.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let time = {
        let st = b.mon_mut(pokemon).volatiles.get_mut(EffectId::Condition(conditions::CONFUSION))?;
        st.data[TIME] -= 1;
        st.data[TIME]
    };
    if time == 0 {
        b.remove_volatile(pokemon, conditions::CONFUSION);
        return None;
    }
    let mon = b.ident(pokemon);
    b.add(Msg::Activate { mon: Some(mon), effect: Arg::new(ArgValue::Word(Word::Confusion)), a: None, b: None, kw: Kw::NONE });
    if b.prng.random_chance(1, 2) {
        return None;
    }
    // `this.actions.getDamage(pokemon, pokemon, 40)`: a number is a new
    // move object `{basePower: 40, type: '???', category: 'Physical',
    // willCrit: false}` with `hit` 0.
    let mv = b.new_active_move(ActiveMove {
        fake: FakeMove::BasePower,
        base_power: 40,
        type_: Type::None,
        category: MoveCategory::Physical,
        ..ActiveMove::EMPTY
    });
    let damage = match Gen3Scripts::get_damage(b, pokemon, pokemon, mv, MoveData::Move, false) {
        MoveResult::Int(n) => n,
        other => panic!("Confusion damage not dealt: {other:?}"),
    };
    // `this.damage(damage, pokemon, pokemon, {id: 'confused', effectType:
    // 'Move', type: '???'})`.
    b.damage(damage, Some(pokemon), Some(pokemon), EffectId::Pseudo(Pseudo::Confused));
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::BeforeMove, on_before_move).priority(3),
]);
