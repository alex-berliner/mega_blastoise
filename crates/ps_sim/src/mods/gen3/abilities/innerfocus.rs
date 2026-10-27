//! Inner Focus: the holder cannot flinch (`TryAddVolatile` refuses `flinch`
//! with `null`, which writes only the `add volatile [flinch] interrupted`
//! debug line).
//!
//! No ability state.
//!
//! PS: data/abilities.ts innerfocus

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::relay_is;

/// `onTryAddVolatile(status, pokemon)`: `null` for flinch.
// PS: data/abilities.ts innerfocus.onTryAddVolatile
fn on_try_add_volatile(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_is(a, c::FLINCH).then_some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryAddVolatile, on_try_add_volatile)]);
