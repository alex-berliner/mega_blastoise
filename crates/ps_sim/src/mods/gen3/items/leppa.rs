//! Gen 3 Leppa Berry: restores 10 PP to a 0-PP move on `Eat`. Triggered
//! each turn by `onUpdate` if any move is out of PP. No gen 3 override
//! (`data/mods/gen3/items.ts` has no `leppaberry` block, so base
//! inheritance carries unchanged).
//!
//! PS: data/items.ts leppaberry (onUpdate, onEat).

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Arg, ArgValue, ItemNum, Kw, KwFlags, MoveNum, Msg};

/// `onUpdate(pokemon)`: eat when any move slot is at 0 PP.
// PS: data/items.ts leppaberry onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("leppa onUpdate target");
    if b.mon(mon).hp == 0 {
        return None;
    }
    if b.mon(mon).move_slots.iter().any(|s| !s.id.is_none() && s.pp == 0) {
        b.eat_item(mon, false, None, crate::effect::EffectId::None);
    }
    None
}

/// `onEat(pokemon)`: +10 PP (Ripen is gen 8) to the first 0-PP slot, else
/// the first slot below its max; nothing when every slot is full. Then
/// `-activate|MON|item: Leppa Berry|MOVE|[consumed]`.
// PS: data/items.ts leppaberry onEat
fn on_eat(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = a.target.mon().expect("leppa onEat target");
    let slots = &b.mon(mon).move_slots;
    let live = |s: &crate::state::MoveSlot| !s.id.is_none();
    let slot = slots
        .iter()
        .position(|s| live(s) && s.pp == 0)
        .or_else(|| slots.iter().position(|s| live(s) && i32::from(s.pp) < i32::from(s.maxpp)))?;
    let item = b.mon(mon).item;
    let pokemon = b.mon_mut(mon);
    let slot_mut = &mut pokemon.move_slots[slot];
    slot_mut.pp = (i32::from(slot_mut.pp) + 10).min(i32::from(slot_mut.maxpp)) as i8;
    let pp = slot_mut.pp;
    // `moveSlots` share their objects with `baseMoveSlots` until Transform
    // or Mimic replaces a slot, so the PP survives a switch (as in
    // `deductPP`).
    if !pokemon.move_slots[slot].virtual_ && !pokemon.transformed {
        pokemon.base_move_slots[slot].pp = pp;
    }
    let mv = b.slot_effect_move(mon, &b.mon(mon).move_slots[slot]);
    let ident = b.ident(mon);
    b.add(Msg::Activate {
        mon: Some(ident),
        effect: Arg::new(ArgValue::ItemEffect(ItemNum(item.0))),
        a: Some(Arg::new(ArgValue::Move(MoveNum(mv.0)))),
        b: None,
        kw: Kw::flags(KwFlags::CONSUMED),
    });
    None
}

pub static LEPPA: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::Eat, on_eat),
]);
