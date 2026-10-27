//! Facade: double power while the user has a status other than sleep.
//!
//! PS: data/moves.ts facade.onBasePower

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onBasePower(basePower, pokemon)`: `if (pokemon.status &&
/// pokemon.status !== 'slp') return this.chainModify(2)`.
// PS: data/moves.ts facade.onBasePower
fn on_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let status = b.mon(pokemon).status;
    if !status.is_none() && status != conditions::SLP {
        // `chainModify` returns nothing.
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePower, on_base_power)]);
