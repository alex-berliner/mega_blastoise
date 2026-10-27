//! Metronome: uses a random move with the `metronome` flag, which becomes
//! the side's selected move. No effect state.
//!
//! The called move runs through `useMove` with Metronome as its source
//! effect (`|move|...|[from] Metronome`); `useMove` skips Metronome's own
//! `AfterMove` and handles a called two-turn move's PP (`scripts.rs`). A
//! called move that is not ported stops the battle as unported.
//!
//! PS: data/mods/gen1/moves.ts metronome (lines 481-497)

use ps_dex::MoveId;

use crate::actions::ActiveMove;
use crate::choice::SelectedMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// Every gen 1 move fits.
const CAP: usize = 256;

/// `onHit(pokemon)`.
// PS: data/mods/gen1/moves.ts metronome.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Metronome targets its user");
    // `dex.moves.all()` filtered by the flag (the gen 1 dex holds only
    // standard moves), sorted by `num` (each `num` is unique).
    let mut moves = [MoveId::NONE; CAP];
    let mut n = 0;
    for (i, def) in b.dex.moves.iter().enumerate() {
        if def.flags.has("metronome") {
            moves[n] = MoveId(i as u16);
            n += 1;
        }
    }
    let dex = b.dex;
    moves[..n].sort_unstable_by_key(|&m| dex.move_def(m).num);
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let random_move = *b.prng.sample(&moves[..n]);
    let side = &mut b.sides[pokemon.side as usize];
    side.last_selected_move = SelectedMove::Move(random_move);
    side.last_enemy_selected_move = side.last_selected_move;
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, random_move));
    b.use_move(mv, pokemon, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);

#[cfg(test)]
mod tests {
    use super::*;

    /// PS 0.11.11's pool (`node` over `Dex.mod('gen1').moves.all()` with
    /// the same filter and sort): 163 moves, Pound first, Substitute last.
    #[test]
    fn pool_is_ps() {
        let dex = &ps_dex::GEN1;
        let mut moves = [MoveId::NONE; CAP];
        let mut n = 0;
        for (i, def) in dex.moves.iter().enumerate() {
            if def.flags.has("metronome") {
                moves[n] = MoveId(i as u16);
                n += 1;
            }
        }
        moves[..n].sort_unstable_by_key(|&m| dex.move_def(m).num);
        assert_eq!(n, 163);
        assert_eq!(moves[0], ps_dex::gen1::moves::POUND);
        assert_eq!(moves[n - 1], ps_dex::gen1::moves::SUBSTITUTE);
        assert!(!moves[..n].contains(&ps_dex::gen1::moves::METRONOME));
        assert!(!moves[..n].contains(&ps_dex::gen1::moves::STRUGGLE));
    }
}
