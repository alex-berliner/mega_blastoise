//! SmellingSalts: double power against a paralyzed target, then cures it.
//!
//! PS: data/moves.ts smellingsalts.basePowerCallback, smellingsalts.onHit

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `basePowerCallback(pokemon, target, move)`: `move.basePower * 2` (with a
/// debug line) when `target.status === 'par'`.
// PS: data/moves.ts smellingsalts.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.source.expect("basePowerCallback without a target");
    let base_power = i32::from(b.active_move().expect("basePowerCallback without a move").base_power);
    if b.mon(target).status == conditions::PAR {
        b.debug(Text::new(TextId::BpDoubledOnParalyzedTarget));
        return Some(Relay::Int(base_power * 2));
    }
    Some(Relay::Int(base_power))
}

/// `onHit(target)`: `if (target.status === 'par') target.cureStatus()`.
// PS: data/moves.ts smellingsalts.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.mon(target).status == conditions::PAR {
        b.cure_status(target, false);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::Hit, on_hit),
]);
