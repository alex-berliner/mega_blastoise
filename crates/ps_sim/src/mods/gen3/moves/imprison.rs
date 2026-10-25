//! Imprison: fails unless a foe knows one of the user's moves; the
//! volatile does the rest (`conditions::imprison`).
//!
//! PS: data/mods/gen4/moves.ts imprison.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryHit(pokemon)`: `undefined` once an active foe shares a move,
/// else `false`.
// PS: data/mods/gen4/moves.ts imprison.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // `pokemon.foes()`: the foe's actives with HP.
    let foe_side = 1 - pokemon.side;
    for pos in 0..b.sides[foe_side as usize].active.len() as u8 {
        let Some(target) = b.active_at(foe_side, pos) else { continue };
        if b.mon(target).hp == 0 || b.mon(target).fainted {
            continue;
        }
        let mine = b.mon(pokemon).move_slots;
        let theirs = b.mon(target).move_slots;
        for slot in mine.iter().filter(|s| !s.id.is_none()) {
            if theirs.iter().any(|t| t.id == slot.id) {
                return None;
            }
        }
    }
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
