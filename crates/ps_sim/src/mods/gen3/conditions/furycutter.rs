//! `furycutter`: Fury Cutter's power multiplier, doubled by each hit (up
//! to 16) and kept while the move is used every turn. `duration: 2`.
//!
//! Effect state: `multiplier` is `data[0]`.
//!
//! PS: data/moves.ts furycutter.condition.onStart,
//! data/mods/gen4/moves.ts furycutter.condition.onRestart

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `this.effectState.multiplier`.
pub const MULTIPLIER: usize = 0;

/// `onStart()`: `multiplier = 1`.
// PS: data/moves.ts furycutter.condition.onStart
fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().data[MULTIPLIER] = 1;
    None
}

/// `onRestart()`: the multiplier doubles up to 16; two more turns.
// PS: data/mods/gen4/moves.ts furycutter.condition.onRestart
fn on_restart(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    if st.data[MULTIPLIER] < 16 {
        st.data[MULTIPLIER] <<= 1;
    }
    st.duration = 2;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
]);
