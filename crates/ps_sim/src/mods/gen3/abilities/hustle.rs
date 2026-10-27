//! Hustle: the holder's Attack is 1.5 times as high, and the accuracy of
//! its physical moves is 0.8 times as high.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts hustle (onModifyAtk, onSourceModifyAccuracy)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

use super::super::conditions::event_move_type;
use super::relay_number;

/// `onModifyAtk(atk)`: `this.modify(atk, 1.5)`.
fn on_modify_atk(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let atk = a.relay?.as_int()?;
    Some(Relay::Int(b.modify_f(i64::from(atk), 1.5) as i32))
}

/// `onSourceModifyAccuracy(accuracy, target, source, move)`: the physical
/// types lose 0.8 of their accuracy.
fn on_source_modify_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_number(a)?;
    let physical = matches!(
        event_move_type(b, a),
        Type::Normal | Type::Fighting | Type::Flying | Type::Poison | Type::Ground | Type::Rock | Type::Bug | Type::Ghost | Type::Steel
    );
    if !physical {
        return None;
    }
    b.chain_modify(3277, 4096);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyAtk, on_modify_atk).priority(5),
    Handler::on(EventId::ModifyAccuracy, on_source_modify_accuracy).prefix(crate::effect::Prefix::Source).priority(7),
]);
