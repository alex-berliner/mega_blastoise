//! `bide`: Bide's volatile (the move's `volatileStatus`), `duration: 3`,
//! locking the user into Bide. It adds up the damage moves deal to the
//! user and remembers who dealt the last of it; on its last turn
//! (`duration` 1, before the residual takes it) the user hits that
//! Pokemon, or a random foe if it left, with a plain `moveData` object of
//! twice the total as fixed damage through gen 3's `tryMoveHit`.
//!
//! Effect state:
//! - `totalDamage`: `data[0]` and `data[1]`, the low and high 16 bits;
//! - `lastDamageSource`: `data[2]`, `1 + side * 8 + index` (0 for none).
//!
//! PS: data/moves.ts bide.condition (`onBeforeMove` from
//! data/mods/gen3/moves.ts)

use ps_dex::{MoveCategory, MoveTarget, Type};
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::actions::{ActiveMove, FakeMove, GenScripts};
use crate::effect::{EffectDef, EffectType, EventArgs, EventId, Handler};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, EffectState, MonRef};
use ps_dex::gen3::{conditions, moves};

/// `this.effectState.totalDamage`.
pub fn total_damage(st: &EffectState) -> i32 {
    i32::from(st.data[0] as u16) | (i32::from(st.data[1] as u16) << 16)
}

fn set_total_damage(st: &mut EffectState, damage: i32) {
    st.data[0] = damage as u16 as i16;
    st.data[1] = (damage >> 16) as u16 as i16;
}

/// `this.effectState.lastDamageSource`.
const LAST_DAMAGE_SOURCE: usize = 2;

fn bide_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::BIDE.0)))
}

/// `onStart(pokemon)`: nothing stored yet.
// PS: data/moves.ts bide.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    set_total_damage(b.effect_state(), 0);
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: bide_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onDamage(damage, target, source, move)`: a move's damage with a source
/// is stored (confusion's self-hit counts: its effect is a `Move`).
// PS: data/moves.ts bide.condition.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if b.effect_type(a.effect) != EffectType::Move {
        return None;
    }
    let source = a.source?;
    let Some(Relay::Int(damage)) = a.relay else { panic!("Bide's onDamage without a damage number: {:?}", a.relay) };
    let st = b.effect_state();
    let total = total_damage(st) + damage;
    set_total_damage(st, total);
    st.data[LAST_DAMAGE_SOURCE] = 1 + i16::from(source.side) * 8 + i16::from(source.index);
    None
}

/// `onBeforeMove(pokemon, target, move)`: on the last turn, the strike
/// (or `-fail` with nothing stored, `-miss` with no foe to hit); before
/// that, `-activate`.
// PS: data/mods/gen3/moves.ts bide.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let (duration, total, last) = {
        let st = b.effect_state();
        (st.duration, total_damage(st), st.data[LAST_DAMAGE_SOURCE])
    };
    if duration != 1 {
        b.add(Msg::Activate { mon: Some(mon), effect: bide_arg(), a: None, b: None, kw: Kw::NONE });
        return None;
    }
    b.add(Msg::EndEffect { mon, effect: bide_arg(), kw: Kw::NONE });
    if total == 0 {
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        return Some(Relay::FAIL);
    }
    if last == 0 {
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        return Some(Relay::FAIL);
    }
    let mut target = MonRef::new(((last - 1) / 8) as u8, ((last - 1) % 8) as u8);
    if !b.mon(target).is_active {
        match b.get_random_target(pokemon, ActionMove::Move(moves::POUND)) {
            Some(t) => target = t,
            None => {
                b.add(Msg::Miss { source: mon, target: None });
                return Some(Relay::FAIL);
            }
        }
    }
    // `{id: 'bide', name: 'Bide', accuracy: 100, damage: totalDamage * 2,
    // category: 'Physical', priority: 0, flags: {contact: 1, protect: 1},
    // effectType: 'Move', type: 'Normal'}`: no `ignoreImmunity` (gen 3's
    // `tryMoveHit` makes it `false`), no `target`, no handlers.
    let damage = u16::try_from(total * 2).expect("Bide's damage fits a fixed-damage field");
    let mv = b.new_active_move(ActiveMove {
        id: moves::BIDE,
        fake: FakeMove::Data,
        type_: Type::Normal,
        category: MoveCategory::Physical,
        target: MoveTarget::Normal,
        accuracy_x256: Some(100 * 256),
        ignore_immunity: false,
        damage: Some(ps_dex::FixedDamage::Hp(damage)),
        ..ActiveMove::EMPTY
    });
    Gen3Scripts::try_move_hit(b, target, pokemon, mv);
    b.remove_volatile(pokemon, conditions::BIDE);
    Some(Relay::FAIL)
}

/// `onMoveAborted(pokemon)`: the volatile goes.
// PS: data/moves.ts bide.condition.onMoveAborted
fn on_move_aborted(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, conditions::BIDE);
    None
}

/// `onEnd(pokemon)`: `-end|MON|move: Bide|[silent]`.
// PS: data/moves.ts bide.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: bide_arg(), kw: Kw::flags(KwFlags::SILENT) });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::value(EventId::LockMove, Relay::Move(moves::BIDE)),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Damage, on_damage).priority(-101),
    Handler::on(EventId::BeforeMove, on_before_move),
    Handler::on(EventId::MoveAborted, on_move_aborted),
    Handler::on(EventId::End, on_end),
]);
