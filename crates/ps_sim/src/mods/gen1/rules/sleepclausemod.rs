//! Sleep Clause Mod: a foe's second sleeping Pokemon is refused.
//!
//! `onBegin` (the `|rule|` line) runs in `Battle::start`. No effect state.
//!
//! PS: data/rulesets.ts sleepclausemod

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Msg, Text, TextId};

/// `onSetStatus(status, target, source)`.
// PS: data/rulesets.ts sleepclausemod.onSetStatus
fn on_set_status(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if a.source.is_some_and(|s| s.side == target.side) {
        return None;
    }
    let Some(Relay::Effect(EffectId::Condition(status))) = a.relay else { return None };
    if b.dex.condition_def(status).key.as_str() == "slp" {
        for i in 0..b.sides[target.side as usize].team_size as usize {
            let p = b.pokemon_at(target.side, i);
            let mon = b.mon(p);
            let slp = !mon.status.is_none() && b.dex.condition_def(mon.status).key.as_str() == "slp";
            if mon.hp != 0 && slp {
                // `!pokemon.statusState.source?.isAlly(pokemon)`
                if !mon.status_state.source.is_some_and(|s| s.side == p.side) {
                    b.add(Msg::Message(Text::new(TextId::SleepClauseActivated)));
                    b.hint(TextId::HintSleepClause, false, None);
                    return Some(Relay::FAIL);
                }
            }
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::SetStatus, on_set_status)]);
