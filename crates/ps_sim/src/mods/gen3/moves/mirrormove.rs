//! Mirror Move: uses the move that last hit the user, when its attacker
//! still knows it and it is not on gen 3's list of moves Mirror Move cannot
//! copy. No effect state.
//!
//! `attackedBy` holds the move id `gotAttacked` recorded (a move another
//! move called is recorded as itself, which its user may not know). The
//! called move runs through `useMove` with Mirror Move as its source
//! effect.
//!
//! PS: data/mods/gen3/moves.ts mirrormove.onHit

use ps_dex::MoveId;

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `noMirror`: the moves Mirror Move fails on.
const NO_MIRROR: [&str; 19] = [
    "assist",
    "curse",
    "doomdesire",
    "focuspunch",
    "futuresight",
    "magiccoat",
    "metronome",
    "mimic",
    "mirrormove",
    "naturepower",
    "psychup",
    "roleplay",
    "sketch",
    "sleeptalk",
    "spikes",
    "spitup",
    "taunt",
    "teeterdance",
    "transform",
];

/// `noMirror.includes(id)`.
fn no_mirror(b: &Battle, id: MoveId) -> bool {
    let name = b.dex.move_def(id).name.as_str();
    NO_MIRROR.iter().any(|&n| ps_dex::text::name_is_id(name, n))
}

/// `onHit(pokemon)`.
// PS: data/mods/gen3/moves.ts mirrormove.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Mirror Move targets its user");
    // `pokemon.getLastAttackedBy()`.
    let Some(last) = b.mon(pokemon).attacked_by.iter().flatten().last().copied() else {
        return Some(Relay::FAIL);
    };
    if b.mon(last.source).last_move.is_none() || last.move_.is_none() {
        return Some(Relay::FAIL);
    }
    if no_mirror(b, last.move_) || !b.has_move(last.source, last.move_) {
        return Some(Relay::FAIL);
    }
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, last.move_));
    b.use_move(mv, pokemon, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
