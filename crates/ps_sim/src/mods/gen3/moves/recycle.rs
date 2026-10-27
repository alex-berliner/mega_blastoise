//! Recycle: the user gets back the item it last used, if it holds none.
//! No effect state.
//!
//! PS: data/moves.ts recycle.onHit

use ps_protocol::{Arg, ArgValue, ItemNum, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;
use ps_dex::ItemId;

/// `onHit(pokemon, source, move)`: `-item|MON|ITEM|[from] move: Recycle`,
/// then `setItem(item, source, move)`; `false` while it holds an item or
/// has none to recycle.
// PS: data/moves.ts recycle.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let (item, last_item) = (b.mon(pokemon).item, b.mon(pokemon).last_item);
    if !item.is_none() || last_item.is_none() {
        return Some(Relay::FAIL);
    }
    b.mon_mut(pokemon).last_item = ItemId::NONE;
    let mon = b.ident(pokemon);
    let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::RECYCLE.0)));
    b.add(Msg::Item { mon, item: ItemNum(last_item.0), kw: Kw::from(from) });
    b.set_item(pokemon, last_item, a.source, a.effect);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
