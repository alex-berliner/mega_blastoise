//! Rest: the user falls asleep and its HP is restored. Gen 1's Rest has
//! two failure cases: at full HP, and when the missing HP is 255 or 511
//! (unless the current HP is a multiple of 256).
//!
//! `this.effectState` is the sleep status's own state (`statusState`):
//! `time` and `startTime` are set to 2 after the status is set, so the
//! user sleeps for exactly one turn.
//!
//! PS: data/mods/gen1/moves.ts rest (line 674)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;
use ps_protocol::TextId;

/// `this.effectState.time`: the turns of sleep left.
const TIME: usize = 0;
/// `this.effectState.startTime`: the turns of sleep it started with.
const START_TIME: usize = 1;

/// `onHit(target, source, move)`: the sleep, the two turns and the heal.
// PS: data/mods/gen1/moves.ts rest.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let hp = b.mon(target).hp;
    let maxhp = b.mon(target).maxhp;
    if hp == maxhp {
        return Some(Relay::FAIL);
    }
    // Fail when health is 255 or 511 less than max, unless it is divisible
    // by 256.
    let short = hp == maxhp.wrapping_sub(255) || hp == maxhp.wrapping_sub(511);
    if short && hp % 256 != 0 {
        b.hint(TextId::HintGen1Recovery, false, None);
        return Some(Relay::FAIL);
    }
    // `target.setStatus('slp', source, move)`.
    if !b.set_status(target, conditions::SLP, a.source, a.effect, false).truthy() {
        return Some(Relay::FAIL);
    }
    // `target.statusState.time = 2; target.statusState.startTime = 2;`
    {
        let st = &mut b.mon_mut(target).status_state;
        st.data[TIME] = 2;
        st.data[START_TIME] = 2;
    }
    // `this.heal(target.maxhp)`: the target and the effect are the running
    // event's.
    b.heal(i32::from(maxhp), None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
