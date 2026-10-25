//! Assist: uses a random move of the user's teammates (in team order, the
//! user left out) without the `noassist` flag. No effect state.
//!
//! Gen 3 inherits gen 4's `onHit`. Gravity and Heal Block are gen 4, so
//! their checks never hold here. The called move runs through `useMove`
//! with Assist as its source effect.
//!
//! PS: data/mods/gen4/moves.ts assist.onHit

use ps_dex::MoveId;

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// Every teammate's four moves, for any team size.
const CAP: usize = 4 * 24;

/// `onHit(target)`.
// PS: data/mods/gen4/moves.ts assist.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Assist targets its user");
    let mut moves = [MoveId::NONE; CAP];
    let mut n = 0;
    // `target.side.pokemon`: the side's current order.
    let side = &b.sides[target.side as usize];
    for &slot in side.pokemon() {
        let pokemon = crate::state::MonRef::new(target.side, slot);
        if pokemon == target {
            continue;
        }
        for ms in b.mon(pokemon).move_slots.iter().filter(|s| !s.id.is_none()) {
            if b.dex.move_def(ms.id).flags.has("noassist") {
                continue;
            }
            moves[n] = ms.id;
            n += 1;
        }
    }
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let random_move = *b.prng.sample(&moves[..n]);
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, random_move));
    b.use_move(mv, target, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
