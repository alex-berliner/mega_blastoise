//! `trapped`: Mean Look's hard trap, kept only while its source is active.
//! No gen 1 move adds it; it is ported so the condition table is whole.
//!
//! Effect state:
//! - `source`: the trapper.
//!
//! PS: data/mods/gen1/conditions.ts trapped (lines 181-191)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Trapped};
use ps_dex::gen1::conditions;

/// `onTrapPokemon(pokemon)`.
// PS: data/mods/gen1/conditions.ts trapped.onTrapPokemon
fn on_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = b.effect_state().source;
    if !source.is_some_and(|s| b.mon(s).is_active) {
        // A plain `delete`: no End event.
        b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(conditions::TRAPPED));
        return None;
    }
    b.mon_mut(pokemon).trapped = Trapped::Yes;
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TrapPokemon, on_trap_pokemon)]);
