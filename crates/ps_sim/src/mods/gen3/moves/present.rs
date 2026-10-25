//! Present: `onModifyMove` rolls a heal (`move.heal`, kept on the move in
//! flight, [`crate::actions::ActiveMove::heal`]) or a power. The heal case
//! keeps base power 0, so `getDamage` deals nothing and `runMoveEffects`
//! heals. Its `move.infiltrates = true` is not kept: in gen 3 only the
//! screens, Mist and Safeguard read it, from damage, boosts and statuses
//! this move never deals in that case.
//!
//! PS: data/moves.ts present.onModifyMove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move, pokemon, target)`: `rand = this.random(10)`; below 2
/// heals a quarter, else power 40 / 80 / 120.
// PS: data/moves.ts present.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let rand = b.prng.random_n(10);
    let am = b.active_move_mut().expect("ModifyMove without an active move");
    if rand < 2 {
        am.heal = Some(ps_dex::Fraction { num: 1, den: 4 });
    } else if rand < 6 {
        am.base_power = 40;
    } else if rand < 9 {
        am.base_power = 80;
    } else {
        am.base_power = 120;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
