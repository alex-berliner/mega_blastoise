//! Eruption: its power scales with the user's HP, kept as the fraction
//! PS's JS number holds (the `BP:` debug line prints it); `getDamage`
//! floors it (`clampIntRange(basePower, 1)`). Water Spout shares the body.
//!
//! PS: data/moves.ts eruption.basePowerCallback (no gen 3 or gen 4
//! override)

use ps_protocol::TextId;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon, target, move)`: `move.basePower *
/// pokemon.hp / pokemon.maxhp`, printed as `BP: ${bp}`.
// PS: data/moves.ts eruption.basePowerCallback
pub fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let base_power = b.active_move().expect("basePowerCallback without a move").base_power;
    let bp = f64::from(base_power) * f64::from(b.mon(pokemon).hp) / f64::from(b.mon(pokemon).maxhp);
    b.debug_num(TextId::BasePower, bp);
    Some(Relay::float(bp))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
