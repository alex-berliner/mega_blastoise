//! Low Kick: power from the target's weight.
//!
//! PS: data/moves.ts lowkick.basePowerCallback

use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon, target)`: `target.getWeight()` (in
/// hectograms) against 2000 / 1000 / 500 / 250 / 100 for 120 / 100 / 80 /
/// 60 / 40, else 20; printed as `BP: ${bp}`.
// PS: data/moves.ts lowkick.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.source.expect("basePowerCallback without a target");
    let target_weight = b.get_weight(target);
    let bp = if target_weight >= 2000 {
        120
    } else if target_weight >= 1000 {
        100
    } else if target_weight >= 500 {
        80
    } else if target_weight >= 250 {
        60
    } else if target_weight >= 100 {
        40
    } else {
        20
    };
    b.debug(Text::with1(TextId::BasePower, TextArg::int(bp)));
    Some(Relay::Int(bp))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
