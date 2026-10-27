//! Freeze Clause Mod: a foe's second frozen Pokemon is refused.
//!
//! `onBegin` (the `|rule|` line) runs in `Battle::start`. No effect state.
//!
//! PS: data/rulesets.ts freezeclausemod

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Msg, Text, TextId};

/// `onSetStatus(status, target, source)`.
// PS: data/rulesets.ts freezeclausemod.onSetStatus
fn on_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if a.source.is_some_and(|s| s.side == target.side) {
        return None;
    }
    let Some(Relay::Effect(EffectId::Condition(status))) = a.relay else { return None };
    if b.dex.condition_def(status).key.as_str() == "frz" {
        for i in 0..b.sides[target.side as usize].team_size as usize {
            let p = b.pokemon_at(target.side, i);
            let st = b.mon(p).status;
            if !st.is_none() && b.dex.condition_def(st).key.as_str() == "frz" {
                b.add(Msg::Message(Text::new(TextId::FreezeClauseActivated)));
                return Some(Relay::FAIL);
            }
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::SetStatus, on_set_status)]);
