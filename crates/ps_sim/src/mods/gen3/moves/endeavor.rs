//! Endeavor: cuts the target's HP down to the user's.
//!
//! PS: data/moves.ts endeavor.damageCallback, endeavor.onTryImmunity;
//! data/mods/gen4/moves.ts endeavor.onTry

use ps_protocol::{Kw, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `damageCallback(pokemon, target)`: `target.getUndynamaxedHP() -
/// pokemon.hp`.
// PS: data/moves.ts endeavor.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = a.source.expect("damageCallback without a target");
    Some(Relay::Int(i32::from(b.mon(target).hp) - i32::from(b.mon(pokemon).hp)))
}

/// `onTryImmunity(target, pokemon)`: `pokemon.hp < target.hp`.
// PS: data/moves.ts endeavor.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let pokemon = a.source.expect("onTryImmunity without a user");
    Some(Relay::Bool(b.mon(pokemon).hp < b.mon(target).hp))
}

/// `onTry(pokemon, target)`: `-fail|USER` and `null` when `pokemon.hp >=
/// target.hp`.
// PS: data/mods/gen4/moves.ts endeavor.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = a.source.expect("endeavor.onTry without a target");
    if b.mon(pokemon).hp >= b.mon(target).hp {
        let mon = b.ident(pokemon);
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DamageCallback, damage_callback),
    Handler::on(EventId::TryImmunity, on_try_immunity),
    Handler::on(EventId::Try, on_try),
]);
