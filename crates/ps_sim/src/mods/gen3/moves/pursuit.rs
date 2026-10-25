//! Pursuit: before the turn the user readies itself (the `pursuit`
//! volatile, which strikes a foe switching out before it goes: see
//! `conditions/pursuit.rs`); against a Pokemon being called back or
//! asked to switch it never misses and has double power. No effect state.
//!
//! PS: data/moves.ts pursuit (`basePowerCallback`, `onModifyMove`),
//! data/mods/gen3/moves.ts pursuit.beforeTurnCallback

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::pursuit::TARGET_LOC;
use crate::relay::Relay;
use crate::state::{Battle, MonRef, SwitchFlag};
use ps_dex::gen3::{conditions, moves};

/// `target.beingCalledBack || target.switchFlag`.
fn leaving(b: &Battle, target: MonRef) -> bool {
    b.mon(target).being_called_back || b.mon(target).switch_flag != SwitchFlag::No
}

/// `beforeTurnCallback(pokemon, target)`: the volatile (its source effect
/// a Pursuit move object) and the target's location.
// PS: data/mods/gen3/moves.ts pursuit.beforeTurnCallback
fn before_turn_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = a.source.expect("Pursuit's beforeTurnCallback without a target");
    b.add_volatile(pokemon, conditions::PURSUIT, Some(pokemon), EffectId::Move(moves::PURSUIT));
    let loc = b.get_loc_of(pokemon, target);
    let st = b.mon_mut(pokemon).volatiles.get_mut(EffectId::Condition(conditions::PURSUIT)).expect("the pursuit volatile");
    st.data[TARGET_LOC] = i16::from(loc);
    None
}

/// `basePowerCallback(pokemon, target, move)`: double against a Pokemon on
/// its way out.
// PS: data/moves.ts pursuit.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.source.expect("Pursuit's basePowerCallback without a target");
    let base_power = i32::from(b.active_move().expect("basePowerCallback without a move").base_power);
    if leaving(b, target) {
        b.debug(Text::new(TextId::PursuitDamageBoost));
        return Some(Relay::Int(base_power * 2));
    }
    Some(Relay::Int(base_power))
}

/// `onModifyMove(move, source, target)`: `accuracy = true` against a
/// Pokemon on its way out (`tracksTarget` matters only with several
/// actives per side).
// PS: data/moves.ts pursuit.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(target) = a.source else { return None };
    if leaving(b, target) {
        b.active_move_mut().expect("ModifyMove without an active move").accuracy_x256 = None;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeTurnCallback, before_turn_callback),
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::ModifyMove, on_modify_move),
]);
