//! Rollout: five turns locked in, the power doubling with each hit (and
//! again after Defense Curl). Ice Ball (`iceball.rs`) is the same move
//! with its own volatile and shares these functions. No effect state of
//! its own (the `rollout` volatile's is in `conditions/rollout.rs`).
//!
//! PS: data/moves.ts rollout

use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::rollout::CONTACT_HIT_COUNT;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_dex::ConditionId;

/// `basePowerCallback(pokemon, target, move)` with the move's volatile
/// `id`.
// PS: data/moves.ts rollout.basePowerCallback
pub fn base_power_callback_of(b: &mut Battle, a: &EventArgs, id: ConditionId) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mut bp = i32::from(b.active_move().expect("basePowerCallback without a move").base_power);
    let asleep = b.mon(pokemon).status == conditions::SLP;
    let key = EffectId::Condition(id);
    if let Some(st) = b.mon_mut(pokemon).volatiles.get_mut(key) {
        if st.hit_count != 0 {
            bp *= 1 << st.data[CONTACT_HIT_COUNT];
        }
        if !asleep {
            st.hit_count += 1;
            st.data[CONTACT_HIT_COUNT] += 1;
            if st.hit_count < 5 {
                st.duration = 2;
            }
        }
    }
    if b.mon(pokemon).volatiles.contains(EffectId::Condition(conditions::DEFENSECURL)) {
        bp *= 2;
    }
    b.debug(Text::with1(TextId::BasePower, TextArg::int(bp)));
    Some(Relay::Int(bp))
}

/// `onModifyMove(move, pokemon, target)`: the volatile, unless the user
/// already rolls, sleeps or has no target; a called use also sets the
/// target location.
// PS: data/moves.ts rollout.onModifyMove
pub fn on_modify_move_of(b: &mut Battle, a: &EventArgs, id: ConditionId) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let Some(target) = a.source else { return None };
    if b.mon(pokemon).volatiles.contains(EffectId::Condition(id)) || b.mon(pokemon).status == conditions::SLP {
        return None;
    }
    b.add_volatile(pokemon, id, None, EffectId::None);
    if b.active_move().is_some_and(|am| !am.source_effect.is_none()) {
        let loc = b.get_loc_of(pokemon, target);
        b.mon_mut(pokemon).last_move_target_loc = loc;
    }
    None
}

/// `onAfterMove(source, target, move)`: Rollout Storage (Instruct and
/// Dancer, gen 7) needs fewer contact hits than hits, which gen 3 cannot
/// make: both counts rise together.
// PS: data/moves.ts rollout.onAfterMove
pub fn on_after_move_of(b: &mut Battle, a: &EventArgs, id: ConditionId) -> Option<Relay> {
    let source = a.target.mon()?;
    if let Some(st) = b.mon(source).volatiles.get(EffectId::Condition(id)) {
        assert!(
            !(st.hit_count == 5 && st.data[CONTACT_HIT_COUNT] < 5),
            "unported: rolloutstorage (fewer contact hits than hits)"
        );
    }
    None
}

fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    base_power_callback_of(b, a, conditions::ROLLOUT)
}

fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    on_modify_move_of(b, a, conditions::ROLLOUT)
}

fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    on_after_move_of(b, a, conditions::ROLLOUT)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::ModifyMove, on_modify_move),
    Handler::on(EventId::AfterMove, on_after_move),
]);
