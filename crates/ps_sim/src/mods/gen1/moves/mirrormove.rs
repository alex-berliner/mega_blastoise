//! Mirror Move: uses the foe's last move (not Mirror Move), which becomes
//! the side's selected move. No effect state.
//!
//! The called move runs through `useMove` with Mirror Move as its source
//! effect; `useMove` skips Mirror Move's own `AfterMove` when it called a
//! move and handles a called two-turn move's PP (`scripts.rs`). A called
//! move that is not ported stops the battle as unported.
//!
//! PS: data/mods/gen1/moves.ts mirrormove (lines 527-538)

use crate::actions::ActiveMove;
use crate::choice::SelectedMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;

/// `onHit(pokemon)`.
// PS: data/mods/gen1/moves.ts mirrormove.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Mirror Move targets its user");
    // `pokemon.side.foe.active[0]?.lastMove` (the move's id, see
    // `moveUsed`).
    let last = b.active_at(1 - pokemon.side, 0).map_or(ps_dex::MoveId::NONE, |f| b.mon(f).last_move);
    if last.is_none() || last == moves::MIRRORMOVE {
        return Some(Relay::FAIL);
    }
    let side = &mut b.sides[pokemon.side as usize];
    side.last_selected_move = SelectedMove::Move(last);
    side.last_enemy_selected_move = side.last_selected_move;
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, last));
    b.use_move(mv, pokemon, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
