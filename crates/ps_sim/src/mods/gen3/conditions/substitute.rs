//! `substitute`: the doll that takes its holder's hits. A hit from a foe
//! (no `bypasssub`) damages the doll instead (`TryPrimaryHit` answers
//! `HIT_SUBSTITUTE`); status moves fail against it (their `getDamage` is
//! `undefined`). A broken doll leaves a handler-less `substitutebroken`
//! volatile (gen 4 clears it on switch-in; gen 3 keeps it).
//!
//! Effect state: `hp` in `data[HP]`.
//!
//! PS: data/moves.ts substitute.condition (onStart, onEnd);
//! data/mods/gen4/moves.ts substitute.condition.onTryPrimaryHit

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::actions::{GenScripts, MoveData, MoveResult};
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Pseudo};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::{conditions, moves};

/// `this.effectState.hp`.
pub const HP: usize = 0;

const SUBSTITUTE: EffectId = EffectId::Condition(conditions::SUBSTITUTE);

fn substitute_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::SUBSTITUTE.0)))
}

/// `onStart(target, source, effect)`: `-start|MON|Substitute` (no Shed
/// Tail in gen 3), `floor(maxhp / 4)` HP, and a partial trap on the
/// holder ends without its `onEnd`.
// PS: data/moves.ts substitute.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: substitute_arg(), a: None, b: None, kw: Kw::NONE });
    let hp = b.mon(target).maxhp / 4;
    b.effect_state().data[HP] = hp as i16;
    let trapped = EffectId::Condition(conditions::PARTIALLYTRAPPED);
    if let Some(st) = b.mon(target).volatiles.get(trapped) {
        // `-end|MON|<the trapping move>|[partiallytrapped]|[silent]`.
        let effect = match st.source_effect {
            EffectId::Move(m) => Arg::new(ArgValue::Move(MoveNum(m.0))),
            other => panic!("partiallytrapped from {other:?}"),
        };
        let mon = b.ident(target);
        let kw = Kw::flags(KwFlags::PARTIALLYTRAPPED.union(KwFlags::SILENT));
        b.add(Msg::EndEffect { mon, effect, kw });
        // `delete target.volatiles['partiallytrapped']`: no `onEnd`.
        if let Some(gone) = b.mon_mut(target).volatiles.remove(trapped) {
            b.bury(gone);
        }
    }
    None
}

/// `onTryPrimaryHit(target, source, move)`: the doll takes the damage
/// (capped at its HP) and the move's recoil and drain follow from it.
// PS: data/mods/gen4/moves.ts substitute.condition.onTryPrimaryHit
fn on_try_primary_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("substitute.onTryPrimaryHit without a source");
    let EffectId::Move(id) = a.effect else { panic!("substitute.onTryPrimaryHit from {:?}", a.effect) };
    let mv = b.active_move.filter(|&r| b.mv(r).id == id).expect("substitute.onTryPrimaryHit: the move in flight");
    let def = b.dex.move_def(id);
    if target == source || def.flags.has("bypasssub") {
        return None;
    }
    let damage = Gen3Scripts::get_damage(b, source, target, mv, MoveData::Move, false);
    // `if (!damage && damage !== 0)`.
    let MoveResult::Int(mut damage) = damage else {
        let mon = b.ident(source);
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        b.attr_last_move_still();
        return Some(Relay::Null);
    };
    let st = b.mon_mut(target).volatiles.get_mut(SUBSTITUTE).expect("the substitute");
    let hp = i32::from(st.data[HP]);
    if damage > hp {
        damage = hp;
    }
    st.data[HP] = (hp - damage) as i16;
    b.mon_mut(source).last_damage = damage as u16;
    if hp - damage <= 0 {
        b.remove_volatile(target, conditions::SUBSTITUTE);
        // `target.addVolatile('substitutebroken')` and its `move`: nothing
        // in gen 3 reads them.
        b.add_volatile(target, conditions::SUBSTITUTEBROKEN, None, EffectId::None);
        if let Some(st) = b.mon_mut(target).volatiles.get_mut(EffectId::Condition(conditions::SUBSTITUTEBROKEN)) {
            st.move_ = id;
        }
    } else {
        let mon = b.ident(target);
        b.add(Msg::Activate { mon: Some(mon), effect: substitute_arg(), a: None, b: None, kw: Kw::flags(KwFlags::DAMAGE) });
    }
    if b.mv(mv).ohko {
        b.add(Msg::Ohko);
    }
    if damage != 0 {
        b.apply_recoil_damage(damage, mv, source);
    }
    if let Some(drain) = def.drain() {
        // `Math.ceil(damage * drain[0] / drain[1])`.
        let amount = (damage * i32::from(drain.num)).div_euclid(i32::from(drain.den))
            + i32::from((damage * i32::from(drain.num)).rem_euclid(i32::from(drain.den)) != 0);
        b.heal(amount, Some(source), Some(target), EffectId::Pseudo(Pseudo::Drain));
    }
    b.run_event(EventId::AfterSubDamage, Holder::Mon(target), Some(source), EffectId::Move(id), Some(Relay::Int(damage)));
    Some(Relay::HIT_SUBSTITUTE)
}

/// `onEnd(target)`: `-end|MON|Substitute`.
// PS: data/moves.ts substitute.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: substitute_arg(), kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::TryPrimaryHit, on_try_primary_hit).priority(-1),
    Handler::on(EventId::End, on_end),
]);
