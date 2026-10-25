//! Sleep Talk: while asleep, uses a random move of the user's (not one
//! with the `nosleeptalk` or `charge` flag). No effect state.
//!
//! `onTry` is the base one (`moves/mod.rs`), `onTryHit` gen 4's (it fails
//! under Choice lock or Encore), `onHit` gen 3's: a slot with no PP left
//! is drawn too, and then the move fails with `cant|MON|nopp|ID`. The
//! called move runs through `useMove` with Sleep Talk as its source effect
//! (gen 3's `tryMoveHit` lets a called multi-hit move go on while the user
//! sleeps).
//!
//! PS: data/moves.ts sleeptalk.onTry; data/mods/gen4/moves.ts
//! sleeptalk.onTryHit; data/mods/gen3/moves.ts sleeptalk.onHit

use ps_dex::MoveId;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Word};

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTryHit(pokemon)`: `!choicelock && !encore`.
// PS: data/mods/gen4/moves.ts sleeptalk.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let ok = !b.has_vol(pokemon, conditions::CHOICELOCK) && !b.has_vol(pokemon, conditions::ENCORE);
    Some(Relay::Bool(ok))
}

/// `onHit(pokemon)`.
// PS: data/mods/gen3/moves.ts sleeptalk.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Sleep Talk targets its user");
    let mut moves = [(MoveId::NONE, 0i8); 4];
    let mut n = 0;
    for ms in b.mon(pokemon).move_slots.iter() {
        // `if (moveid && ...)`.
        if ms.id.is_none() {
            continue;
        }
        let flags = b.dex.move_def(ms.id).flags;
        if !flags.has("nosleeptalk") && !flags.has("charge") {
            moves[n] = (ms.id, ms.pp);
            n += 1;
        }
    }
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let (random_move, pp) = *b.prng.sample(&moves[..n]);
    if pp == 0 {
        // `randomMove.move` is the move's id.
        let mon = b.ident(pokemon);
        let reason = Arg::new(ArgValue::Word(Word::NoPp));
        let mv = Some(Arg::new(ArgValue::MoveId(MoveNum(random_move.0))));
        b.add(Msg::Cant { mon, reason, mv, kw: Kw::NONE });
        return None;
    }
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, random_move));
    b.use_move(mv, pokemon, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Try, super::try_asleep),
    Handler::on(EventId::Hit, on_hit),
    Handler::on(EventId::TryHit, on_try_hit),
]);
