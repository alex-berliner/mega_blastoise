//! Trick: the user and the target swap held items. It fails against
//! Sticky Hold (`onTryImmunity`), when either item cannot be taken, or
//! when neither holds one. No effect state.
//!
//! PS: data/moves.ts trick (`onTryImmunity`, `onHit`)

use ps_protocol::{Arg, ArgValue, ItemNum, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectId, EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef};
use ps_dex::gen3::{abilities, moves};
use ps_dex::ItemId;

fn trick_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::TRICK.0)))
}

/// `onTryImmunity(target)`: `!target.hasAbility('stickyhold')`.
// PS: data/moves.ts trick.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    Some(Relay::Bool(!b.has_ability(target, abilities::STICKYHOLD)))
}

/// `this.singleEvent('TakeItem', item, holder.itemState, target, source,
/// move, item)`: the item's own `onTakeItem`, truthy when it lets go.
pub fn item_take_event(b: &mut Battle, item: ItemId, holder: MonRef, target: MonRef, source: MonRef, effect: EffectId) -> bool {
    let serial = b.mon(holder).item_state.serial;
    let relay = Some(Relay::Effect(EffectId::Item(item)));
    b.single_event(EventId::TakeItem, EffectId::Item(item), StateRef::Item(holder), serial, Holder::Mon(target), Some(source), effect, relay)
        .truthy()
}

/// `target.item = item.id`: the item goes back without its state.
pub fn put_back(b: &mut Battle, m: MonRef, item: Option<ItemId>) {
    if let Some(item) = item {
        b.mon_mut(m).item = item;
    }
}

/// `onHit(target, source, move)`: both items are taken, then each holder
/// gets the other's, with `-activate|SOURCE|move: Trick|[of] TARGET` and an
/// `-item` line per item received (`-enditem ... [silent]` for nothing).
// PS: data/moves.ts trick.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("Trick's onHit without a source");
    let effect = a.effect;
    let your_item = b.take_item(target, Some(source));
    let my_item = b.take_item(source, None);
    // `false` is `Err(FAIL)`, `undefined` is `Err(Null)`.
    let refused = your_item == Err(Relay::FAIL) || my_item == Err(Relay::FAIL);
    let (yours, mine) = (your_item.ok(), my_item.ok());
    if refused || (yours.is_none() && mine.is_none()) {
        put_back(b, target, yours);
        put_back(b, source, mine);
        return Some(Relay::FAIL);
    }
    let mine_refused = mine.is_some_and(|i| !item_take_event(b, i, source, target, source, effect));
    if mine_refused || yours.is_some_and(|i| !item_take_event(b, i, target, source, target, effect)) {
        put_back(b, target, yours);
        put_back(b, source, mine);
        return Some(Relay::FAIL);
    }
    let src = b.ident(source);
    let tgt = b.ident(target);
    b.add(Msg::Activate { mon: Some(src), effect: trick_arg(), a: None, b: None, kw: Kw { of: Some(tgt), ..Kw::NONE } });
    let silent = Kw::from(trick_arg()).with(KwFlags::SILENT.union(KwFlags::FLAGS_FIRST));
    match (mine, yours) {
        (Some(m), _) => {
            b.set_item(target, m, None, EffectId::None);
            b.add(Msg::Item { mon: tgt, item: ItemNum(m.0), kw: Kw::from(trick_arg()) });
        }
        (None, Some(y)) => b.add(Msg::EndItem { mon: tgt, item: ItemNum(y.0), kw: silent }),
        (None, None) => unreachable!("Trick with no item on either side"),
    }
    match (yours, mine) {
        (Some(y), _) => {
            b.set_item(source, y, None, EffectId::None);
            b.add(Msg::Item { mon: src, item: ItemNum(y.0), kw: Kw::from(trick_arg()) });
        }
        (None, Some(m)) => b.add(Msg::EndItem { mon: src, item: ItemNum(m.0), kw: silent }),
        (None, None) => unreachable!("Trick with no item on either side"),
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryImmunity, on_try_immunity),
    Handler::on(EventId::Hit, on_hit),
]);
