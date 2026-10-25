//! `stall`: Protect, Detect and Endure's falling success chance when used
//! in a row (their `onHit` adds or restarts it, their `onPrepareHit` asks
//! `StallMove`). `duration: 2`, `counterMax: 8` (gen 4's).
//!
//! Effect state: `counter` is `counter`.
//!
//! PS: data/mods/gen5/conditions.ts stall, data/mods/gen4/conditions.ts
//! stall (counterMax)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Text, TextArg, TextArgValue, TextId};

/// `onStart()`: `this.effectState.counter = 2`.
// PS: data/mods/gen5/conditions.ts stall.onStart
fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter = 2;
    None
}

/// `onStallMove()`: a one in `counter` chance.
// PS: data/mods/gen5/conditions.ts stall.onStallMove
fn on_stall_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let counter = match b.effect_state().counter {
        0 => 1,
        c => u32::from(c),
    };
    // `if (counter >= 256) return this.randomChance(1, 2 ** 32)`: the
    // counter never passes gen 4's `counterMax` of 8.
    assert!(counter < 256, "stall counter {counter} past counterMax");
    // `Math.round(100 / counter)`, counter >= 1.
    // `Math.round(x)` is `floor(x + 0.5)`: `floor((200 + c) / 2c)`.
    let chance = ((200 + counter) / (2 * counter)) as i32;
    b.debug(Text::with1(TextId::SuccessChance, TextArg::new(TextArgValue::Int(chance))));
    Some(Relay::Bool(b.prng.random_chance(1, counter)))
}

/// `onRestart()`: the counter doubles up to `counterMax`; two more turns.
// PS: data/mods/gen5/conditions.ts stall.onRestart
fn on_restart(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let counter_max = b.dex.condition_def(conditions::STALL).counter_max.map_or(0, u16::from);
    let st = b.effect_state();
    if st.counter < counter_max {
        st.counter *= 2;
    }
    st.duration = 2;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::StallMove, on_stall_move),
    Handler::on(EventId::Restart, on_restart),
]);
