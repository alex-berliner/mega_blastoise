//! Stockpile: adds a layer of the `stockpile` volatile; fails at three.
//! No effect state.
//!
//! PS: data/moves.ts stockpile.onTry

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// The user's `stockpile` layers, 0 without the volatile.
pub fn layers(b: &Battle, pokemon: crate::state::MonRef) -> u8 {
    b.mon(pokemon).volatiles.get(EffectId::Condition(conditions::STOCKPILE)).map_or(0, |st| st.layers)
}

/// `onTry(source)`: three layers is the most.
// PS: data/moves.ts stockpile.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    if b.mon(source).volatiles.contains(EffectId::Condition(conditions::STOCKPILE)) && layers(b, source) >= 3 {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Try, on_try)]);
