//! Wonder Guard: only super-effective moves hit the holder; any other
//! damaging move (not a Status move, not typeless Struggle) shows
//! `-immune ... [from] ability: Wonder Guard` and stops (`null`). Fire
//! Fang's gen 4 exception names a move gen 3 does not have.
//!
//! No ability state.
//!
//! PS: data/mods/gen4/abilities.ts wonderguard

use ps_dex::{MoveCategory, Type};
use ps_protocol::{MoveNum, Text, TextArg, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, event_move};

/// `onTryHit(target, source, move)`.
// PS: data/mods/gen4/abilities.ts wonderguard.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mv = event_move(b, a)?;
    // `move.id === 'firefang'`: the hint is gen 4's, and gen 3 has no Fire
    // Fang.
    assert!(b.dex.move_id("firefang") != Some(mv.id), "unported: Fire Fang through Wonder Guard (gen 4)");
    if target == source || mv.category == MoveCategory::Status || mv.type_ == Type::None {
        return None;
    }
    b.debug(Text::with1(TextId::WonderGuardImmunity, TextArg::move_id(MoveNum(mv.id.0))));
    let mref = b.active_move?;
    if b.run_effectiveness(target, mref) <= 0 || !b.run_immunity(target, Some(mref), false) {
        add_immune(b, target);
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
