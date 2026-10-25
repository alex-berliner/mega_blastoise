//! Gen 3 Lum Berry (`onAfterSetStatus` and `onUpdate`); `onEat` cures
//! status and removes confusion. `onAfterSetStatusPriority: -1`; the
//! `-status` line is already written when `AfterSetStatus` runs, so the
//! eat follows it.
//!
//! PS: data/items.ts lumberry.

use crate::effect::{EffectDef, EventArgs, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onAfterSetStatus(status, pokemon)`: eat whatever status just took.
// PS: data/items.ts lumberry onAfterSetStatus
fn on_after_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("lum onAfterSetStatus target");
    b.eat_item(mon, false, None, crate::effect::EffectId::None);
    None
}

/// `onUpdate(pokemon)`: eat on any status OR confusion.
// PS: data/items.ts lumberry onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("lum onUpdate target");
    let has_confusion = b
        .dex
        .condition_id("confusion")
        .is_some_and(|c| b.has_vol(mon, c));
    if !b.mon(mon).status.is_none() || has_confusion {
        b.eat_item(mon, false, None, crate::effect::EffectId::None);
    }
    None
}

/// `onEat(pokemon)`: PS `pokemon.cureStatus(); pokemon.removeVolatile('confusion')`.
// PS: data/items.ts lumberry onEat
fn on_eat(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("lum onEat target");
    if !b.mon(mon).status.is_none() {
        b.cure_status(mon, false);
    }
    if let Some(c) = b.dex.condition_id("confusion") {
        if b.has_vol(mon, c) {
            b.remove_volatile(mon, c);
        }
    }
    None
}

pub static LUM: EffectDef = EffectDef::new(&[
    Handler::on(crate::effect::EventId::AfterSetStatus, on_after_set_status).priority(-1),
    Handler::on(crate::effect::EventId::Update, on_update),
    Handler::on(crate::effect::EventId::Eat, on_eat),
]);
