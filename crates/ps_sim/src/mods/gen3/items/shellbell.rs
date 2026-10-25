//! `shellbell`: Shell Bell, an eighth of the damage the holder's move
//! dealt back after the move.
//!
//! PS: data/items.ts shellbell

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::abilities::event_move;
use crate::relay::Relay;
use crate::state::Battle;

/// `onAfterMoveSecondarySelf(pokemon, target, move)`: `if
/// (move.totalDamage && !pokemon.forceSwitchFlag) this.heal(move.totalDamage
/// / 8, pokemon)`. The source is the event's (the move's target), so the
/// line names it with `[of]`. PS's amount is a float; `heal` raises a
/// positive amount at most 1 to 1 and truncates the rest.
///
/// `move` is the event's own move object, not `battle.activeMove`: after
/// a calling move (Nature Power, Metronome, Sleep Talk...) `useMove` has
/// left `activeMove` on the called move, whose own event already healed.
/// The calling move dealt no damage (`totalDamage` falsy), so its event
/// finds no move object of its id here and heals nothing.
// PS: data/items.ts shellbell.onAfterMoveSecondarySelf
fn on_after_move_secondary_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let total_damage = event_move(b, a).map_or(0, |m| m.total_damage);
    if total_damage != 0 && !b.mon(pokemon).force_switch_flag {
        b.heal((total_damage / 8).max(1), Some(pokemon), None, EffectId::None);
    }
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::AfterMoveSecondarySelf, on_after_move_secondary_self).priority(-1)]);
