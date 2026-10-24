//! `mist`: Mist's volatile (a volatile since gen 2's mod; gen 1 keeps
//! that). It drops the stat drops a foe's status move aims at its holder;
//! gen 1 lets damaging moves' drops through.
//!
//! Effect state: none.
//!
//! PS: data/mods/gen2/moves.ts mist.condition.onStart (lines 391-393);
//! data/mods/gen1/moves.ts mist.condition.onTryBoost (lines 543-558)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;
use ps_dex::MoveCategory;

/// `onStart(pokemon)`: `-start|MON|Mist`.
// PS: data/mods/gen2/moves.ts mist.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::MIST.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onTryBoost(boost, target, source, effect)`: `delete boost[i]` for each
/// drop, which edits the relayed table in place (the handler returns
/// `undefined`); here the edited table is the return value.
// PS: data/mods/gen1/moves.ts mist.condition.onTryBoost
fn on_try_boost(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let Some(Relay::Boosts(mut boost)) = a.relay else { panic!("TryBoost without a boosts table") };
    // `if (effect.effectType === 'Move' && effect.category !== 'Status') return;`
    let mut secondaries = false;
    if let EffectId::Move(m) = a.effect {
        let def = b.dex.move_def(m);
        let category = b.active_move().filter(|am| am.id == m).map_or(def.category, |am| am.category);
        if category != MoveCategory::Status {
            return None;
        }
        // `effect.secondaries`: the move's (`null` when it has none).
        secondaries = !def.secondaries().is_empty();
    }
    let source = a.source?;
    if target == source {
        return None;
    }
    let mut show_msg = false;
    for v in boost.iter_mut() {
        if *v < 0 {
            *v = 0;
            show_msg = true;
        }
    }
    if show_msg && !secondaries {
        let mon = b.ident(target);
        let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::MIST.0)));
        b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    }
    Some(Relay::Boosts(boost))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::TryBoost, on_try_boost),
]);
