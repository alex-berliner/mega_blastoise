//! Conversion 2: the user becomes a random type that resists or is immune
//! to the type of the target's last used move (Struggle counts as Normal).
//! No effect state.
//!
//! `lastMoveUsed.type` is the move object's type after its `ModifyMove`
//! handlers ([`crate::state::Pokemon::last_move_used_type`]); the types are
//! `dex.types.names()` in its order.
//!
//! PS: data/mods/gen3/moves.ts conversion2.onHit

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target, source)`.
// PS: data/mods/gen3/moves.ts conversion2.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Conversion 2 hits a Pokemon");
    let source = a.source.expect("Conversion 2 has a user");
    let last = b.mon(target).last_move_used;
    if last.is_none() {
        return Some(Relay::FAIL);
    }
    let attack_type = if last == moves::STRUGGLE { Type::Normal } else { b.mon(target).last_move_used_type };
    let mut possible = [Type::None; 18];
    let mut n = 0;
    for &t in b.dex.typechart.order {
        // `damageTaken[attackType]`: `???` has no entry.
        let code = if attack_type == Type::None { 0 } else { b.dex.typechart.code(attack_type, t) };
        if code == 2 || code == 3 {
            possible[n] = t;
            n += 1;
        }
    }
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let t = *b.prng.sample(&possible[..n]);
    if !b.set_type(source, t, false) {
        return Some(Relay::FAIL);
    }
    super::add_typechange(b, source, t);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
