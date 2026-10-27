//! Sleep (gen 3): the `-status` line, the turn counter, the "cannot move"
//! line, and the turns a Sleep Talk or Snore user skipped, given back when
//! it switches out.
//!
//! `this.effectState` is the status state: `time` (the turns of sleep left)
//! is `data[0]`, `skippedTime` (turns spent using a `sleepUsable` move just
//! before switching out) is `data[1]`.
//!
//! PS: data/mods/gen3/conditions.ts slp

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Status};

/// `this.effectState.time`.
pub const TIME: usize = 0;
/// `this.effectState.skippedTime`.
const SKIPPED_TIME: usize = 1;

/// `onStart(target, source, sourceEffect)`: the `-status` line, 1-4 turns
/// of sleep, and the Nightmare's end.
// PS: data/mods/gen3/conditions.ts slp.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let kw = match a.effect {
        // `[from] move: ${sourceEffect.name}`.
        EffectId::Move(m) => Kw::from(Arg::new(ArgValue::MoveEffect(MoveNum(m.0)))),
        _ => Kw::NONE,
    };
    b.add(Msg::SetStatus { mon, status: Status::Slp, kw });
    let time = b.prng.random_range(2, 6) as i16;
    {
        let st = b.effect_state();
        st.data[TIME] = time;
        st.data[SKIPPED_TIME] = 0;
    }
    // `if (target.removeVolatile('nightmare')) this.add('-end', target,
    // 'Nightmare', '[silent]')`.
    b.remove_nightmare(target);
    None
}

/// `onSwitchOut(target)`: the skipped turns come back.
// PS: data/mods/gen3/conditions.ts slp.onSwitchOut
fn on_switch_out(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    st.data[TIME] += st.data[SKIPPED_TIME];
    st.data[SKIPPED_TIME] = 0;
    None
}

/// `onBeforeMove(pokemon, target, move)`: the counter (twice as fast with
/// Early Bird), waking up, or the "cannot move" line.
// PS: data/mods/gen3/conditions.ts slp.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let earlybird = b.dex.ability_id("earlybird");
    if earlybird.is_some_and(|e| b.has_ability(pokemon, e)) {
        b.mon_mut(pokemon).status_state.data[TIME] -= 1;
    }
    let time = {
        let st = &mut b.mon_mut(pokemon).status_state;
        st.data[TIME] -= 1;
        st.data[TIME]
    };
    if time <= 0 {
        b.cure_status(pokemon, false);
        return None;
    }
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::new(ArgValue::Status(Status::Slp)), mv: None, kw: Kw::NONE });
    let sleep_usable = match a.effect {
        EffectId::Move(m) => b.dex.move_def(m).bools.has("sleepUsable"),
        _ => false,
    };
    if sleep_usable {
        b.effect_state().data[SKIPPED_TIME] += 1;
        return None;
    }
    b.effect_state().data[SKIPPED_TIME] = 0;
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::SwitchOut, on_switch_out),
    Handler::on(EventId::BeforeMove, on_before_move).priority(10),
]);
