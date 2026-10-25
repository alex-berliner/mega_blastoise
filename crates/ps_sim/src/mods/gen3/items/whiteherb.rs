//! Gen 3 White Herb: clears any negative stat-boost stages on the holder,
//! consuming the herb. PS runs `onStart` (and `onAnySwitchIn`,
//! `onAnyAfterMega`, `onAnyAfterMove`, `onResidual`) to scan for
//! negative boosts; if any are present, the item is consumed via
//! `target.useItem()`, which writes `-enditem` and fires the `onUse`
//! handler that emits `-clearnegativeboost`. `onResidualOrder: 29` so
//! the herb runs later than the standard berry order 10.
//!
//! PS: data/items.ts whiteherb.

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{KwFlags, Msg};

/// Five handlers share this body: `onStart`, `onAnySwitchIn`,
/// `onAnyAfterMega`, `onAnyAfterMove`, `onResidual` (the `onAny*` ones call
/// `onStart` with `this.effectState.target`, the residual one with its
/// target: always the holder, never the event's target). If any negative
/// boost exists, consume the herb (`useItem` fires the `onUse`
/// single_event, which writes `-clearnegativeboost`).
// PS: data/items.ts whiteherb onStart / onAnySwitchIn / onAnyAfterMega / onAnyAfterMove / onResidual
fn setup_and_consume(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let mon = b.effect_state().target.mon().expect("whiteherb holder");
    if !b.mon(mon).boosts.iter().any(|&s| s < 0) {
        return None;
    }
    b.use_item(mon, None, EffectId::None);
    None
}

/// `onUse(pokemon)`: PS `pokemon.setBoost(this.effectState.boosts);
/// this.add('-clearnegativeboost', pokemon, '[silent]')`. We bypass
/// `Battle::boost` (which would write per-stat lines) and emit
/// `-clearnegativeboost` once. `effectState.boosts` holds a 0 for each
/// stage that was negative when `onStart` ran, just before this
/// `useItem`, so the set is the clearing of the negative stages.
// PS: data/items.ts whiteherb onUse
fn on_use(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("whiteherb onUse target");
    for s in b.mon_mut(mon).boosts.iter_mut() {
        if *s < 0 {
            *s = 0;
        }
    }
    let ident = b.ident(mon);
    b.add(Msg::ClearNegativeBoost { mon: ident, kw: ps_protocol::Kw::flags(KwFlags::SILENT) });
    None
}

pub static WHITEHERB: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, setup_and_consume),
    Handler::on(EventId::SwitchIn, setup_and_consume).prefix(Prefix::Any),
    Handler::on(EventId::AfterMega, setup_and_consume).prefix(Prefix::Any),
    Handler::on(EventId::AfterMove, setup_and_consume).prefix(Prefix::Any),
    Handler::on(EventId::Residual, setup_and_consume).order(29),
    Handler::on(EventId::Use, on_use),
]);
