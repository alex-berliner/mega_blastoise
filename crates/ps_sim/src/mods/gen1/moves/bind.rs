//! Bind: a partial trapping move. The data does the work (`self:
//! partialtrappinglock`, `volatileStatus: partiallytrapped`, both in
//! `conditions/`); gen 1 adds `onTryMove`, which ends the target's
//! Hyper Beam recharge whether or not the move hits. No effect state.
//!
//! PS: data/mods/gen1/moves.ts bind (lines 69-81)

use ps_protocol::TextId;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onTryMove(source, target)`.
// PS: data/mods/gen1/moves.ts bind.onTryMove
fn on_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.source?;
    if b.mon(target).volatiles.contains(EffectId::Condition(conditions::MUSTRECHARGE)) {
        b.remove_volatile(target, conditions::MUSTRECHARGE);
        b.hint(TextId::HintGen1PartialTrapRecharge, true, None);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryMove, on_try_move)]);
