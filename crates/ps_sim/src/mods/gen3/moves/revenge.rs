//! Revenge: double power after taking damage from the target this turn.
//!
//! PS: data/moves.ts revenge.basePowerCallback

use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon, target, move)`: `move.basePower * 2` when an
/// `attackedBy` entry from `target` dealt damage this turn (with the
/// `BP doubled for getting hit by ${target}` debug line), else
/// `move.basePower`.
// PS: data/moves.ts revenge.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = a.source.expect("basePowerCallback without a target");
    let damaged_by_target =
        b.mon(pokemon).attacked_by.iter().flatten().any(|p| p.source == target && p.damage > 0 && p.this_turn);
    let base_power = i32::from(b.active_move().expect("basePowerCallback without a move").base_power);
    if damaged_by_target {
        let mon = b.ident(target);
        b.debug(Text::with1(TextId::BpDoubledForGettingHitBy, TextArg::mon(mon)));
        return Some(Relay::Int(base_power * 2));
    }
    Some(Relay::Int(base_power))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
