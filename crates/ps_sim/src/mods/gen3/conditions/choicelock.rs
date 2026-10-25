//! `choicelock`: Choice Band's lock on the move it was first used with
//! (the item's `onModifyMove` adds it).
//!
//! Effect state: `move` is `move_`.
//!
//! PS: data/mods/gen4/conditions.ts choicelock (onStart),
//! data/conditions.ts choicelock (onDisableMove)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onStart(pokemon)`: refused before any move, else the last move.
// PS: data/mods/gen4/conditions.ts choicelock.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let last = b.mon(pokemon).last_move;
    if last.is_none() {
        return Some(Relay::FAIL);
    }
    b.effect_state().move_ = last;
    None
}

/// `onDisableMove(pokemon)`: the lock goes with the Choice item or the
/// move; otherwise every other move is disabled.
// PS: data/conditions.ts choicelock.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let locked = b.effect_state().move_;
    let item = b.mon(pokemon).item;
    let is_choice = !item.is_none() && b.dex.item_def(item).is_choice;
    if !is_choice || !b.has_move(pokemon, locked) {
        b.remove_volatile(pokemon, conditions::CHOICELOCK);
        return None;
    }
    // No Dynamax in gen 3.
    if b.ignoring_item(pokemon) {
        return None;
    }
    let source_effect = b.effect_state().source_effect;
    for i in 0..4 {
        let id = b.mon(pokemon).move_slots[i].id;
        if !id.is_none() && id != locked {
            b.disable_move(pokemon, id, false, source_effect);
        }
    }
    None
}


pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
