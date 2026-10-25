//! Mimic: the target's last move replaces Mimic in the user's move slots
//! (5 PP at most) until the user switches out. No effect state.
//!
//! Gen 3 inherits gen 4's `onHit`: it fails for a transformed user, a
//! target with no last move or behind a Substitute, a `failmimic` move or
//! one the user knows. The slot is `virtual`, so its PP is not written
//! through to the base slot (`deductPP`).
//!
//! PS: data/mods/gen4/moves.ts mimic.onHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MoveSlot};
use ps_dex::gen3::{conditions, moves};

/// `onHit(target, source)`.
// PS: data/mods/gen4/moves.ts mimic.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Mimic hits a Pokemon");
    let source = a.source.expect("Mimic has a user");
    let last = b.mon(target).last_move;
    if b.mon(source).transformed || last.is_none() || b.has_vol(target, conditions::SUBSTITUTE) {
        return Some(Relay::FAIL);
    }
    // `source.moves.includes(id)`: the ids of the move slots.
    let knows = |b: &Battle, id| b.mon(source).move_slots.iter().any(|s| !s.id.is_none() && s.id == id);
    if b.dex.move_def(last).flags.has("failmimic") || knows(b, last) {
        return Some(Relay::FAIL);
    }
    let Some(mimic_index) = b.mon(source).move_slots.iter().position(|s| s.id == moves::MIMIC) else {
        return Some(Relay::FAIL);
    };
    let def = b.dex.move_def(last);
    let pp = def.pp.min(5) as i8;
    let maxpp = crate::stats::calculate_pp(b.gen, def, b.pp_ups(source, mimic_index));
    b.mon_mut(source).move_slots[mimic_index] = MoveSlot { id: last, pp, maxpp, disabled: 0, used: false, virtual_: true, no_target: true };
    let mon = b.ident(source);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::MIMIC.0)));
    let name = Some(Arg::new(ArgValue::Move(MoveNum(last.0))));
    b.add(Msg::Activate { mon: Some(mon), effect, a: name, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
