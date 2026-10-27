//! `substitute`: the doll that takes its holder's hits. Gen 1's version
//! blocks only some status moves, counts the uncapped damage of a hit as
//! `lastDamage` (Counter reads it), lets recoil, drain and a secondary
//! confusion through while the doll stands, and marks `subFainted` when it
//! breaks (gen 1's `tryMoveHit` reads that).
//!
//! Effect state: `hp` in `data[0]` (gen 1's `directDamage` also edits it).
//!
//! PS: data/mods/gen1/moves.ts substitute.condition (lines 823-895:
//! `onStart` 825-832, `onTryHit` 835-894); data/moves.ts
//! substitute.condition.onEnd (lines 18366-18368)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, TextId};

use crate::actions::{GenScripts, MoveData, MoveResult};
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen1::scripts::Gen1Scripts;
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen1::{conditions, moves};
use ps_dex::MoveCategory;

const SUBSTITUTE: EffectId = EffectId::Condition(conditions::SUBSTITUTE);

fn substitute_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::SUBSTITUTE.0)))
}

/// `onStart(target)`: the doll gets `floor(maxhp / 4) + 1` HP, and a
/// partial trap on the holder ends without its `onEnd`.
// PS: data/mods/gen1/moves.ts substitute.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: substitute_arg(), a: None, b: None, kw: Kw::NONE });
    let hp = b.mon(target).maxhp / 4 + 1;
    b.effect_state().data[0] = hp as i16;
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
        b.mon_mut(target).volatiles.remove(trapped);
    }
    None
}

/// `onEnd(target)`: `-end|MON|Substitute`.
// PS: data/moves.ts substitute.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: substitute_arg(), kw: Kw::NONE });
    None
}

/// `onTryHit(target, source, move)`: the doll takes a damaging hit
/// (`return 0`, PS's `HIT_SUBSTITUTE`), or blocks a status move
/// (`return false`), or lets it through (`undefined`).
// PS: data/mods/gen1/moves.ts substitute.condition.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("substitute.onTryHit without a source");
    let EffectId::Move(id) = a.effect else { panic!("substitute.onTryHit from {:?}", a.effect) };
    let mv = b.active_move.filter(|&r| b.mv(r).id == id).expect("substitute.onTryHit: the move in flight");
    let def = b.dex.move_def(id);
    if b.mv(mv).category == MoveCategory::Status {
        // `SubBlocked`: ['lockon', 'meanlook', 'mindreader', 'nightmare'],
        // none of them a gen 1 move.
        let sub_blocked = matches!(def.name.as_str(), "Lock-On" | "Mean Look" | "Mind Reader" | "Nightmare");
        let status = if def.status.is_none() { "" } else { b.effect_key(EffectId::Condition(def.status)) };
        if status == "psn"
            || status == "tox"
            || (def.boosts().is_some() && target != source)
            || def.volatile_status == conditions::CONFUSION
            || sub_blocked
        {
            return Some(Relay::FAIL);
        }
        return None;
    }
    if !def.volatile_status.is_none() && target == source {
        return None;
    }
    // `move.hit > 1 ? this.lastDamage : this.actions.getDamage(source, target, move)`.
    let mut uncapped = if b.mv(mv).hit > 1 {
        MoveResult::Int(i32::from(b.last_damage))
    } else {
        Gen1Scripts::get_damage(b, source, target, mv, MoveData::Move, false)
    };
    if id == moves::BIDE {
        let st = b.mon(source).volatiles.get(EffectId::Condition(conditions::BIDE)).expect("bide's volatile");
        uncapped = MoveResult::Int(crate::mods::gen1::conditions::bide::stored(st) * 2);
    }
    // `if (!uncappedDamage && uncappedDamage !== 0) return null;`
    let MoveResult::Int(uncapped) = uncapped else { return Some(Relay::Null) };
    b.last_damage = uncapped as u16;
    let st = b.mon_mut(target).volatiles.get_mut(SUBSTITUTE).expect("the substitute");
    let hp = i32::from(st.data[0]);
    let hp = hp - uncapped.min(hp);
    st.data[0] = hp as i16;
    if hp <= 0 {
        b.remove_volatile(target, conditions::SUBSTITUTE);
        b.mon_mut(target).sub_fainted = true;
    } else {
        let mon = b.ident(target);
        b.add(Msg::Activate { mon: Some(mon), effect: substitute_arg(), a: None, b: None, kw: Kw::flags(KwFlags::DAMAGE) });
    }
    // Recoil, drain and a secondary confusion only while the doll stands.
    if b.has_vol(target, conditions::SUBSTITUTE) {
        if uncapped != 0 {
            b.apply_recoil_damage(uncapped, mv, source);
        }
        if let Some(drain) = def.drain() {
            let amount = (uncapped * i32::from(drain.num) / i32::from(drain.den)).max(1);
            b.last_damage = amount as u16;
            b.heal(amount, Some(source), Some(target), EffectId::Pseudo(crate::effect::Pseudo::Drain));
        }
        // `move.secondary?.volatileStatus === 'confusion'`.
        if let Some(secondary) = def.secondaries().first().filter(|s| s.volatile_status == conditions::CONFUSION) {
            let hit = match secondary.chance {
                None => true,
                Some(chance) => {
                    let n = (u32::from(chance) * 256).div_ceil(100) - 1;
                    b.prng.random_chance(n, 256)
                }
            };
            if hit {
                b.require_ported(EffectId::Condition(conditions::CONFUSION));
                b.add_volatile(target, conditions::CONFUSION, Some(source), EffectId::Move(id));
                b.hint(TextId::HintGen1SubConfusion, false, None);
            }
        }
    }
    b.run_event(EventId::AfterSubDamage, Holder::Mon(target), Some(source), EffectId::Move(id), Some(Relay::Int(uncapped)));
    // `target.getLastAttackedBy()`: update the last entry, or push one.
    let last = b.mon(target).attacked_by.iter().rposition(|e| e.is_some());
    match last {
        Some(i) => {
            let e = b.mon_mut(target).attacked_by[i].as_mut().expect("an attackedBy entry");
            e.move_ = id;
            e.damage = uncapped.max(0) as u16;
        }
        None => b.got_attacked(target, id, MoveResult::Int(uncapped), source),
    }
    Some(Relay::HIT_SUBSTITUTE)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::TryHit, on_try_hit).priority(-1),
]);
