//! Metronome: uses a random move with the `metronome` flag. No effect
//! state.
//!
//! Gen 3 inherits gen 4's `onHit`: the pool is every standard move with the
//! flag, sorted by `num` (the gen 3 dex holds only standard moves and
//! Hidden Power's placeholders, which have no flag). Only gens 2 and 4
//! leave out the user's own moves; Gravity and Heal Block are gen 4. The
//! called move runs through `useMove` with Metronome as its source effect
//! (`|move|...|[from] Metronome`), and its Pressure PP is Metronome's
//! (`callerMoveForPressure`, `scripts.rs`). A called move that is not
//! ported stops the battle as unported.
//!
//! PS: data/mods/gen4/moves.ts metronome.onHit

use ps_dex::MoveId;

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// Every gen 3 move fits.
const CAP: usize = 512;

/// The moves Metronome draws from, sorted by `num`; returns the count.
// PS: data/mods/gen4/moves.ts metronome.onHit (the filter)
fn pool(dex: &'static ps_dex::Dex, out: &mut [MoveId; CAP]) -> usize {
    let mut n = 0;
    for (i, def) in dex.moves.iter().enumerate() {
        if def.flags.has("metronome") {
            out[n] = MoveId(i as u16);
            n += 1;
        }
    }
    // `moves.sort((a, b) => a.num - b.num)`: each `num` is unique.
    out[..n].sort_unstable_by_key(|&m| dex.move_def(m).num);
    n
}

/// `onHit(pokemon)`.
// PS: data/mods/gen4/moves.ts metronome.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Metronome targets its user");
    let mut moves = [MoveId::NONE; CAP];
    let n = pool(b.dex, &mut moves);
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let random_move = *b.prng.sample(&moves[..n]);
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, random_move));
    b.use_move(mv, pokemon, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);

#[cfg(test)]
mod tests {
    use super::*;

    /// PS's pool at the pin (`node` over `Dex.mod('gen3').moves.all()` with
    /// the same filter and sort): 336 moves, Pound first, Psycho Boost
    /// last, Hidden Power in.
    #[test]
    fn pool_is_ps() {
        let dex = &ps_dex::GEN3;
        let mut moves = [MoveId::NONE; CAP];
        let n = pool(dex, &mut moves);
        assert_eq!(n, 336);
        assert_eq!(dex.move_def(moves[0]).name.as_str(), "Pound");
        assert_eq!(dex.move_def(moves[n - 1]).name.as_str(), "Psycho Boost");
        assert!(moves[..n].contains(&ps_dex::gen3::moves::HIDDENPOWER));
        assert!(!moves[..n].contains(&ps_dex::gen3::moves::METRONOME));
    }
}
