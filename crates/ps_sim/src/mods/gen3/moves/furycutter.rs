//! Fury Cutter: its power is 10 times the `furycutter` multiplier (at
//! most 160), which each hit doubles. No effect state.
//!
//! PS: data/moves.ts furycutter.basePowerCallback,
//! data/mods/gen3/moves.ts furycutter.onHit

use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::furycutter::MULTIPLIER;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `basePowerCallback(pokemon, target, move)`: adds the volatile on the
/// first use (or the first hit of a multi-hit use), then
/// `clampIntRange(basePower * multiplier, 1, 160)`.
// PS: data/moves.ts furycutter.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let key = EffectId::Condition(conditions::FURYCUTTER);
    let first_hit = b.active_move().is_some_and(|am| am.hit == 1);
    if !b.mon(pokemon).volatiles.contains(key) || first_hit {
        b.add_volatile(pokemon, conditions::FURYCUTTER, None, EffectId::None);
    }
    let base_power = i32::from(b.active_move().expect("basePowerCallback without a move").base_power);
    let multiplier = b.mon(pokemon).volatiles.get(key).map_or(0, |st| i32::from(st.data[MULTIPLIER]));
    let bp = (base_power * multiplier).clamp(1, 160);
    b.debug(Text::with1(TextId::BasePower, TextArg::int(bp)));
    Some(Relay::Int(bp))
}

/// `onHit(target, source)`: `source.addVolatile('furycutter')` (a restart
/// doubles the multiplier).
// PS: data/mods/gen3/moves.ts furycutter.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source?;
    b.add_volatile(source, conditions::FURYCUTTER, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::Hit, on_hit),
]);
