//! `pursuit`: Pursuit's readiness for the turn (its `beforeTurnCallback`
//! adds it), `duration: 1`. When a foe switches out, the user, if it can
//! still move and targeted that slot, uses Pursuit on it first, in place
//! of its queued move.
//!
//! Effect state: `targetLoc` is `data[0]`; `source` and `sourceEffect` (a
//! Pursuit move object) are the state's own.
//!
//! PS: data/mods/gen3/moves.ts pursuit.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::actions::GenScripts;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{abilities, conditions, moves};

/// `this.effectState.targetLoc`.
pub const TARGET_LOC: usize = 0;

/// `onFoeBeforeSwitchOut(pokemon)`: Pursuit on the leaving foe, unless the
/// user targeted another slot, is frozen or asleep, loafs (Truant), is
/// not adjacent, has fainted, is encored into another move, or has no
/// move left in the queue to cancel.
// PS: data/mods/gen3/moves.ts pursuit.condition.onFoeBeforeSwitchOut
fn on_foe_before_switch_out(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let (source, target_loc, source_effect) = {
        let st = b.effect_state();
        (st.source.expect("pursuit without a source"), st.data[TARGET_LOC], st.source_effect)
    };
    b.debug(Text::new(TextId::PursuitStart));
    let status = b.mon(source).status;
    let loafing = b.has_ability(source, abilities::TRUANT)
        && b.mon(source).volatiles.contains(EffectId::Condition(conditions::TRUANT));
    let encored_elsewhere = b
        .mon(source)
        .volatiles
        .get(EffectId::Condition(conditions::ENCORE))
        .is_some_and(|st| st.move_ != moves::PURSUIT);
    if target_loc != i16::from(b.get_loc_of(source, pokemon))
        || status == conditions::FRZ
        || status == conditions::SLP
        || loafing
        || !b.is_adjacent(source, pokemon)
        || b.mon(source).hp == 0
        || encored_elsewhere
        || !b.queue.cancel_move(source)
    {
        return None;
    }
    if !b.mon(pokemon).pursuit_activated {
        let mon = b.ident(pokemon);
        b.add(Msg::Activate {
            mon: Some(mon),
            effect: Arg::new(ArgValue::MoveEffect(MoveNum(moves::PURSUIT.0))),
            a: None,
            b: None,
            kw: Kw::NONE,
        });
        b.mon_mut(pokemon).pursuit_activated = true;
    }
    // No Mega Evolution or Ultra Burst in gen 3.
    let mv = b.get_active_move(ActionMove::Move(moves::PURSUIT));
    Gen3Scripts::deduct_pp(b, source, moves::PURSUIT, 0);
    let position = b.mon(pokemon).position as i8;
    b.move_used(source, mv, position);
    if b.use_move(mv, source, Some(Some(pokemon)), source_effect) {
        let item = b.mon(source).item;
        if !item.is_none() && b.dex.item_def(item).is_choice {
            b.add_volatile(source, conditions::CHOICELOCK, None, EffectId::None);
        }
    }
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::BeforeSwitchOut, on_foe_before_switch_out).prefix(Prefix::Foe)]);
