//! Mimic: a random move of the target replaces the move slot the user
//! selected, keeping that slot's PP, until the user switches out. No
//! effect state.
//!
//! The slot is `virtual`: gen 1's `deductPP` writes its PP through to the
//! real slot (`scripts.rs`), so the PP spent on the mimicked move is the
//! Mimic slot's.
//!
//! PS: data/mods/gen1/moves.ts mimic (lines 498-519)

use ps_dex::MoveId;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MoveSlot};

/// `onHit(target, source)`.
// PS: data/mods/gen1/moves.ts mimic.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Mimic hits a Pokemon");
    let source = a.source.expect("Mimic has a user");
    let moveslot = b.sides[source.side as usize].last_selected_move_slot as usize;
    // `target.moves`: the ids of its move slots.
    let mut moves = [MoveId::NONE; 4];
    let mut n = 0;
    for ms in b.mon(target).move_slots.iter().filter(|s| !s.id.is_none()) {
        moves[n] = ms.id;
        n += 1;
    }
    // `if (!moveid) return false`: every Pokemon has a move.
    let moveid = *b.prng.sample(&moves[..n]);
    let def = b.dex.move_def(moveid);
    let pp = b.mon(source).move_slots[moveslot].pp;
    let maxpp = crate::stats::calculate_pp(b.gen, def, b.pp_ups(source, moveslot));
    b.mon_mut(source).move_slots[moveslot] = MoveSlot { id: moveid, pp, maxpp, disabled: 0, used: false, virtual_: true, no_target: false };
    let mon = b.ident(source);
    let effect = Arg::new(ArgValue::Move(MoveNum(ps_dex::gen1::moves::MIMIC.0)));
    let name = Some(Arg::new(ArgValue::Move(MoveNum(moveid.0))));
    b.add(Msg::StartEffect { mon, effect, a: name, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
