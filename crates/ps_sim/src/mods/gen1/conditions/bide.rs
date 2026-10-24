//! `bide`: Bide's condition (the move's `volatileStatus`). For 2 or 3
//! turns the user stores `lastDamage`, the battle-wide damage register,
//! whoever dealt it; then it deals twice the total to the foe as a bare
//! `{damage}` hit, or fails if nothing was stored.
//!
//! `onSemiLockMove: 'bide'` locks the request (unless `maybeLocked`), and
//! `onDisableMove` clears `maybeLocked`: the player knows it is locked.
//!
//! Effect state:
//! - `time`: `data[0]`;
//! - `damage`: `data[1]` and `data[2]`, the low and high 16 bits.
//!
//! PS: data/mods/gen1/moves.ts bide.condition (lines 36-67)

use ps_protocol::{Arg, ArgValue, Kw, Msg, MoveNum, Text, TextId};

use crate::actions::{ActiveMove, GenScripts, MoveData};
use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen1::scripts::Gen1Scripts;
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, EffectState};
use ps_dex::gen1::{conditions, moves};

fn stored(st: &EffectState) -> i32 {
    i32::from(st.data[1] as u16) | (i32::from(st.data[2] as u16) << 16)
}

fn set_stored(st: &mut EffectState, damage: i32) {
    st.data[1] = damage as u16 as i16;
    st.data[2] = (damage >> 16) as u16 as i16;
}

fn bide_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::BIDE.0)))
}

/// `onStart(pokemon)`.
// PS: data/mods/gen1/moves.ts bide.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let time = b.prng.random_range(2, 4) as i16;
    let st = b.effect_state();
    set_stored(st, 0);
    st.data[0] = time;
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: bide_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon, t, move)`: store, or unleash.
// PS: data/mods/gen1/moves.ts bide.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let current_move = b.new_active_move(ActiveMove::from_dex(b.dex, moves::BIDE));
    let last_damage = i32::from(b.last_damage);
    let st = b.effect_state();
    let damage = stored(st) + last_damage;
    set_stored(st, damage);
    st.data[0] -= 1;
    let time = st.data[0];
    let mon = b.ident(pokemon);
    if time == 0 {
        b.add(Msg::EndEffect { mon, effect: bide_arg(), kw: Kw::NONE });
        if damage == 0 {
            b.debug(Text::new(TextId::BideNoDamage));
            b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
            b.remove_volatile(pokemon, conditions::BIDE);
            return Some(Relay::FAIL);
        }
        let target = b.get_random_target(pokemon, ActionMove::Move(moves::POUND));
        Gen1Scripts::move_hit(b, target, pokemon, current_move, MoveData::Damage(damage * 2), false, false);
        b.remove_volatile(pokemon, conditions::BIDE);
        return Some(Relay::FAIL);
    }
    b.add(Msg::Activate { mon: Some(mon), effect: bide_arg(), a: None, b: None, kw: Kw::NONE });
    Some(Relay::FAIL)
}

/// `onDisableMove(target)`.
// PS: data/mods/gen1/moves.ts bide.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.mon_mut(target).maybe_locked = false;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move),
    Handler::value(EventId::SemiLockMove, Relay::Move(moves::BIDE)),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
