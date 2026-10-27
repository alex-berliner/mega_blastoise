//! Flail: the less HP the user has left, the stronger. Gen 3's table (from
//! `floor(hp * 48 / maxhp)`); Reversal shares the body.
//!
//! PS: data/mods/gen3/moves.ts flail.basePowerCallback

use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon)`: `ratio = max(floor(hp * 48 / maxhp), 1)`,
/// then 200 / 150 / 100 / 80 / 40 / 20 below 2 / 5 / 10 / 17 / 33.
// PS: data/mods/gen3/moves.ts flail.basePowerCallback
pub fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let ratio = (i32::from(b.mon(pokemon).hp) * 48 / i32::from(b.mon(pokemon).maxhp)).max(1);
    let bp = if ratio < 2 {
        200
    } else if ratio < 5 {
        150
    } else if ratio < 10 {
        100
    } else if ratio < 17 {
        80
    } else if ratio < 33 {
        40
    } else {
        20
    };
    b.debug(Text::with1(TextId::BasePower, TextArg::int(bp)));
    Some(Relay::Int(bp))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
