//! Gen 3 Mental Herb: gen 4's `onUpdate`, which cures Attract only (the
//! base entry's Taunt / Encore / Torment / Disable / Heal Block list is
//! gen 5). The herb is `useItem`'d, not eaten (`isBerry: false`).
//!
//! PS: data/mods/gen4/items.ts mentalherb (gen 3 inherits it).

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Arg, ArgValue, ItemNum, Kw, MoveNum, Msg};

/// `onUpdate(pokemon)`: `if (pokemon.volatiles['attract'] &&
/// pokemon.useItem()) { pokemon.removeVolatile('attract');
/// this.add('-end', pokemon, 'move: Attract', '[from] item: Mental Herb'); }`.
// PS: data/mods/gen4/items.ts mentalherb onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("mentalherb onUpdate target");
    let attract = b.dex.condition_id("attract").expect("attract in the dex");
    if !b.has_vol(mon, attract) {
        return None;
    }
    let item = b.mon(mon).item;
    if !b.use_item(mon, None, EffectId::None) {
        return None;
    }
    b.remove_volatile(mon, attract);
    let mv = b.dex.move_id("attract").expect("attract in the dex");
    let ident = b.ident(mon);
    let from = Arg::new(ArgValue::ItemEffect(ItemNum(item.0)));
    b.add(Msg::EndEffect { mon: ident, effect: Arg::new(ArgValue::MoveEffect(MoveNum(mv.0))), kw: Kw::from(from) });
    None
}

pub static MENTALHERB: EffectDef = EffectDef::new(&[Handler::on(EventId::Update, on_update)]);
