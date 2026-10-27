//! Gen 3 Persim Berry: eats on `Update` if confused, then removes
//! confusion on `Eat`. No `onResidual`, no priority data; base inheritance
//! carries unchanged in gen 3.
//!
//! PS: data/items.ts persimberry.

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onUpdate(pokemon)`: eat if confused.
// PS: data/items.ts persimberry onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("persim onUpdate target");
    let Some(c) = b.dex.condition_id("confusion") else { return None };
    if b.has_vol(mon, c) {
        b.eat_item(mon, false, None, crate::effect::EffectId::None);
    }
    None
}

/// `onEat(pokemon)`: PS `pokemon.removeVolatile('confusion')`.
// PS: data/items.ts persimberry onEat
fn on_eat(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("persim onEat target");
    if let Some(c) = b.dex.condition_id("confusion") {
        if b.has_vol(mon, c) {
            b.remove_volatile(mon, c);
        }
    }
    None
}

pub static PERSIM: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::Eat, on_eat),
]);
