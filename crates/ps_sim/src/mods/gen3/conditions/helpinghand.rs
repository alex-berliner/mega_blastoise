//! `helpinghand`: Helping Hand's volatile on the ally it helps. Singles has
//! no ally, so the move never finds a target (`-notarget`) and this
//! condition never starts; it is ported for completeness.
//!
//! Effect state: `multiplier` (1.5 per Helping Hand this turn) is kept as
//! the count of them in `counter`.
//!
//! PS: data/moves.ts helpinghand.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn add_line(b: &mut Battle, a: &EventArgs) -> Option<()> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mon = b.ident(target);
    let of = b.ident(source);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::HELPINGHAND.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw { of: Some(of), ..Kw::NONE } });
    Some(())
}

/// `onStart(target, source)`: multiplier 1.5 and
/// `-singleturn|MON|Helping Hand|[of] SOURCE`.
// PS: data/moves.ts helpinghand.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter = 1;
    add_line(b, a);
    None
}

/// `onRestart(target, source)`: the multiplier grows by 1.5 and the line
/// again.
// PS: data/moves.ts helpinghand.condition.onRestart
fn on_restart(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    b.effect_state().counter += 1;
    add_line(b, a);
    None
}

/// `onBasePower(basePower)`: `chainModify(multiplier)` with its debug
/// line. Only a doubles ally can carry the volatile.
// PS: data/moves.ts helpinghand.condition.onBasePower
fn on_base_power(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    unreachable!("Helping Hand boosts an ally: no allies in singles")
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::BasePower, on_base_power).priority(10),
]);
