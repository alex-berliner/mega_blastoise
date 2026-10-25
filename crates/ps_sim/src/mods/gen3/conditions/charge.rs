//! `charge`: Charge's volatile. The user's next Electric move has double
//! power; any move but Charge itself (used, or aborted) then ends it. No
//! effect state.
//!
//! PS: data/moves.ts charge.condition (`onMoveAborted` and `onAfterMove`
//! from data/mods/gen8/moves.ts)

use ps_dex::Type;
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::event_move_type;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

fn charge_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::CHARGE.0)))
}

/// `-start|MON|Charge` (Electromorphosis and Wind Power, gen 9, write
/// their own).
fn start_line(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: charge_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onStart(pokemon, source, effect)`.
// PS: data/moves.ts charge.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    start_line(b, a)
}

/// `onRestart(pokemon, source, effect)`: the same line again.
// PS: data/moves.ts charge.condition.onRestart
fn on_restart(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    start_line(b, a)
}

/// `onBasePower(basePower, attacker, defender, move)`: Electric moves
/// double.
// PS: data/moves.ts charge.condition.onBasePower
fn on_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if event_move_type(b, a) == Type::Electric {
        b.debug(Text::new(TextId::ChargeBoost));
        b.chain_modify(2, 1);
    }
    None
}

/// `onMoveAborted` and `onAfterMove(pokemon, target, move)`: any move but
/// Charge ends it.
// PS: data/mods/gen8/moves.ts charge.condition.onMoveAborted, onAfterMove
fn on_move_done(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if a.effect != EffectId::Move(moves::CHARGE) {
        b.remove_volatile(pokemon, conditions::CHARGE);
    }
    None
}

/// `onEnd(pokemon)`: `-end|MON|Charge|[silent]`.
// PS: data/moves.ts charge.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: charge_arg(), kw: Kw::flags(KwFlags::SILENT) });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::BasePower, on_base_power).priority(9),
    Handler::on(EventId::MoveAborted, on_move_done),
    Handler::on(EventId::AfterMove, on_move_done),
    Handler::on(EventId::End, on_end),
]);
