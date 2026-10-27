//! Haze: gen 1's clears every active Pokemon's boosts, cures the foe's
//! status (a sleeping or frozen foe loses its chosen move to
//! `cannotmove`), turns the user's Toxic into plain poison, and ends a set
//! list of volatiles. `onHitField` is gen 3's, which gen 1 inherits but
//! never calls (Haze targets `self`, so `moveHit` runs `onHit`).
//!
//! PS: data/mods/gen1/moves.ts haze.onHit (lines 368-415);
//! data/mods/gen3/moves.ts haze.onHitField (lines 341-346)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, Word};

use crate::choice::SelectedMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::queue::{Action, ActionKind, ActionMove};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use ps_dex::gen1::{conditions, moves};
use ps_dex::ConditionId;

/// `effect.id` of `pokemon.status` (`""` for none).
fn status_key(b: &Battle, pokemon: MonRef) -> &'static str {
    let s = b.mon(pokemon).status;
    if s.is_none() {
        ""
    } else {
        b.effect_key(EffectId::Condition(s))
    }
}

/// `this.log[this.log.length - 1] += '|[silent]'`: the `-end` line the
/// volatile's own `onEnd` just wrote goes silent.
fn silence_last_line(b: &mut Battle) {
    match b.log.as_mut_slice().last_mut() {
        Some(Msg::EndEffect { kw, .. }) => kw.flags.insert(KwFlags::SILENT),
        other => panic!("haze: the line to silence is {other:?}"),
    }
}

/// `onHit(target, source)`.
// PS: data/mods/gen1/moves.ts haze.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source;
    let mon = b.ident(target);
    let haze = Arg::new(ArgValue::MoveEffect(MoveNum(moves::HAZE.0)));
    b.add(Msg::Activate { mon: Some(mon), effect: haze, a: None, b: None, kw: Kw::NONE });
    b.add(Msg::ClearAllBoost { kw: Kw::flags(KwFlags::SILENT) });
    let (actives, n) = b.all_active(false);
    for &pokemon in &actives[..n] {
        b.clear_boosts(pokemon);
        if Some(pokemon) != source {
            if matches!(status_key(b, pokemon), "frz" | "slp") {
                let side = pokemon.side as usize;
                b.sides[side].last_selected_move = SelectedMove::CannotMove;
                b.sides[side].last_enemy_selected_move = SelectedMove::CannotMove;
                if b.will_move(pokemon).is_some() {
                    let mut action = Action::new(ActionKind::Move).with_pokemon(pokemon);
                    action.move_ = ActionMove::CannotMove;
                    b.change_action(pokemon, action);
                }
            }
            b.cure_status(pokemon, true);
        }
        if status_key(b, pokemon) == "tox" {
            let psn = b.dex.condition_id("psn").expect("psn in the dex");
            b.require_ported(EffectId::Condition(psn));
            b.set_status(pokemon, psn, None, EffectId::None, true);
        }
        b.update_mon_speed(pokemon);
        // `hazeVolatiles`, in PS's key order, with the `-end` name each
        // prints (`disable` and `confusion` print their own, made silent).
        const HAZE_VOLATILES: [(ConditionId, Option<ArgValue>); 8] = [
            (conditions::DISABLE, None),
            (conditions::CONFUSION, None),
            (conditions::MIST, Some(ArgValue::Move(MoveNum(moves::MIST.0)))),
            (conditions::FOCUSENERGY, Some(ArgValue::MoveEffect(MoveNum(moves::FOCUSENERGY.0)))),
            (conditions::LEECHSEED, Some(ArgValue::MoveEffect(MoveNum(moves::LEECHSEED.0)))),
            (conditions::LIGHTSCREEN, Some(ArgValue::Move(MoveNum(moves::LIGHTSCREEN.0)))),
            (conditions::REFLECT, Some(ArgValue::Move(MoveNum(moves::REFLECT.0)))),
            (conditions::RESIDUALDMG, Some(ArgValue::Word(Word::ToxicCounter))),
        ];
        for (v, name) in HAZE_VOLATILES {
            if !b.remove_volatile(pokemon, v) {
                continue;
            }
            match (v, name) {
                (conditions::DISABLE | conditions::CONFUSION, _) => silence_last_line(b),
                (_, Some(name)) => {
                    let mon = b.ident(pokemon);
                    b.add(Msg::EndEffect { mon, effect: Arg::new(name), kw: Kw::flags(KwFlags::SILENT) });
                }
                (_, None) => unreachable!("every other Haze volatile has a name"),
            }
        }
    }
    None
}

/// `onHitField()`: gen 3's Haze, never run in gen 1.
// PS: data/mods/gen3/moves.ts haze.onHitField
fn on_hit_field(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    b.add(Msg::ClearAllBoost { kw: Kw::NONE });
    let (actives, n) = b.all_active(false);
    for &pokemon in &actives[..n] {
        b.clear_boosts(pokemon);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::HitField, on_hit_field),
    Handler::on(EventId::Hit, on_hit),
]);
