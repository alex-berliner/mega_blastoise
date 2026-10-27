//! Blaze: Fire moves get 1.5 times their base power while the user is at
//! a third of its HP or less.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts blaze (onBasePower), data/mods/gen4/abilities.ts
//! blaze (onBasePower)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;
use ps_protocol::{Text, TextId};

use super::super::conditions::event_move_type;

/// `onBasePower(basePower, attacker, defender, move)`.
fn on_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let attacker = a.target.mon()?;
    if event_move_type(b, a) != Type::Fire {
        return None;
    }
    // `attacker.hp <= attacker.maxhp / 3`.
    if i32::from(b.mon(attacker).hp) * 3 > i32::from(b.mon(attacker).maxhp) {
        return None;
    }
    b.debug(Text::new(TextId::BlazeBoost));
    b.chain_modify_f(1.5);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePower, on_base_power).priority(2)]);
