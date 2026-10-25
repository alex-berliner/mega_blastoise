//! Sketch: the target's last move replaces Sketch for good (in the move
//! slots and the base move slots, with the move's full PP). No effect
//! state.
//!
//! Gen 3 inherits gen 4's `onHit`: it fails like Mimic (transformed user,
//! no last move, a Substitute, a `nosketch` move or one the user knows).
//! PS puts the same slot object in both lists; here the two copies agree,
//! and `deductPP` writes a non-virtual slot through to the base slot. The
//! line names Mimic, as PS writes it.
//!
//! PS: data/mods/gen4/moves.ts sketch.onHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MoveSlot};
use ps_dex::gen3::{conditions, moves};

/// `onHit(target, source)`.
// PS: data/mods/gen4/moves.ts sketch.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Sketch hits a Pokemon");
    let source = a.source.expect("Sketch has a user");
    let last = b.mon(target).last_move;
    if b.mon(source).transformed || last.is_none() || b.has_vol(target, conditions::SUBSTITUTE) {
        return Some(Relay::FAIL);
    }
    let knows = |b: &Battle, id| b.mon(source).move_slots.iter().any(|s| !s.id.is_none() && s.id == id);
    if b.dex.move_def(last).flags.has("nosketch") || knows(b, last) {
        return Some(Relay::FAIL);
    }
    let Some(sketch_index) = b.mon(source).move_slots.iter().position(|s| s.id == moves::SKETCH) else {
        return Some(Relay::FAIL);
    };
    let def = b.dex.move_def(last);
    let maxpp = crate::stats::calculate_pp(b.gen, def, b.pp_ups(source, sketch_index));
    let sketched = MoveSlot { id: last, pp: def.pp as i8, maxpp, disabled: 0, used: false, virtual_: false, no_target: true };
    let mon = b.mon_mut(source);
    mon.move_slots[sketch_index] = sketched;
    mon.base_move_slots[sketch_index] = sketched;
    let mon = b.ident(source);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::MIMIC.0)));
    let name = Some(Arg::new(ArgValue::Move(MoveNum(last.0))));
    b.add(Msg::Activate { mon: Some(mon), effect, a: name, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
