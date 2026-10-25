//! Thick Fat: Ice and Fire moves against the holder have half their base
//! power.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts thickfat (onSourceBasePower), data/mods/gen4/
//! abilities.ts thickfat (onSourceBasePower)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

use super::super::conditions::event_move_type;

/// `onSourceBasePower(basePower, attacker, defender, move)`: the holder is
/// the event's source (the defender).
fn on_source_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if !matches!(event_move_type(b, a), Type::Ice | Type::Fire) {
        return None;
    }
    b.chain_modify_f(0.5);
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::BasePower, on_source_base_power).prefix(crate::effect::Prefix::Source).priority(1)]);
