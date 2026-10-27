//! `fakepartiallytrapped`: a partial trap ended this turn, but neither
//! player knows until they try to attack, so both Pokemon's requests stay
//! `maybeLocked` for a turn.
//!
//! Duration 2 (the dex). Gen 1's `endTurn` drops it once the counterpart
//! has fainted or lost its own (`Battle::gen1_partial_trapping_cleanup`).
//!
//! Effect state:
//! - `counterpart`: `data[0]` as a Pokemon (`EffectState::data_mon`),
//!   written by `partialtrappinglock.onAfterMove`.
//!
//! PS: data/mods/gen1/conditions.ts fakepartiallytrapped (lines 218-227)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onDisableMove(target)`.
// PS: data/mods/gen1/conditions.ts fakepartiallytrapped.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.mon_mut(target).maybe_locked = true;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DisableMove, on_disable_move).priority(1)]);
