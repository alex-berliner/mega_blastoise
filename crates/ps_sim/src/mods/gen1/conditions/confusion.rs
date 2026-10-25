//! Confusion: the `-start` line, the turn counter and the self-hit.
//!
//! `this.effectState` is the volatile's own state: `time` is the turns of
//! confusion left. It lives in `data[0]`.
//!
//! PS: data/mods/gen1/conditions.ts confusion (line 130)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use crate::stats::Stat;
use ps_dex::gen1::conditions;
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, Msg, Word};

/// `this.effectState.time`: the turns of confusion left.
const TIME: usize = 0;

/// `onStart(target, source, sourceEffect)`: the `-start` line and the
/// turn counter.
// PS: data/mods/gen1/conditions.ts confusion.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let kw = match a.effect {
        // `sourceEffect.id === 'lockedmove'`: the Outrage confusion is
        // silent.
        EffectId::Condition(c) if b.dex.condition_def(c).key.as_str() == "lockedmove" => {
            Kw::flags(KwFlags::SILENT)
        }
        _ => Kw::NONE,
    };
    let effect = Arg::new(ArgValue::Word(Word::Confusion));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw });
    let time = b.prng.random_range(2, 6) as i16;
    b.effect_state().data[TIME] = time;
    None
}

/// `onEnd(target)`: the `-end` line.
// PS: data/mods/gen1/conditions.ts confusion.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::Word(Word::Confusion));
    b.add(Msg::EndEffect { mon, effect, kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon, target)`: the turn counter and the self-hit.
// PS: data/mods/gen1/conditions.ts confusion.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let time = {
        let mon = b.mon_mut(pokemon);
        let state = mon.volatiles.get_mut(EffectId::Condition(conditions::CONFUSION));
        match state {
            Some(s) => {
                s.data[TIME] -= 1;
                s.data[TIME]
            }
            None => return None,
        }
    };
    if time == 0 {
        b.remove_volatile(pokemon, conditions::CONFUSION);
        return None;
    }
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Word(Word::Confusion));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    if b.prng.random_chance(128, 256) {
        return None;
    }
    // `Math.floor(Math.floor(((Math.floor(2 * level / 5) + 2) * atk * 40) / def) / 50) + 2`
    let level = i32::from(b.mon(pokemon).level);
    let atk = b.get_stat(pokemon, Stat::Atk, false, false);
    let def = b.get_stat(pokemon, Stat::Def, false, false);
    let damage = ((level * 2) / 5 + 2) * atk * 40 / def / 50 + 2;
    // `this.directDamage(damage, pokemon, target)`: the confused Pokemon is
    // the target, the move's target the source.
    b.direct_damage(damage, Some(pokemon), a.source, EffectId::None);
    b.remove_volatile(pokemon, conditions::BIDE);
    b.remove_volatile(pokemon, conditions::TWOTURNMOVE);
    b.remove_volatile(pokemon, conditions::INVULNERABILITY);
    b.remove_volatile(pokemon, conditions::PARTIALTRAPPINGLOCK);
    b.remove_volatile(pokemon, conditions::LOCKEDMOVE);
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::BeforeMove, on_before_move).priority(3),
]);
