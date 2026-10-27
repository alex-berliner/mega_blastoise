//! `focuspunch`: Focus Punch's turn of focus (its `beforeTurnCallback`
//! adds it). A damaging hit breaks the focus; flinching cannot happen.
//! `duration: 1`.
//!
//! Effect state: `lostFocus` is `data[0]` (1 for true).
//!
//! PS: data/moves.ts focuspunch.condition

use ps_dex::MoveCategory;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::event_move_category;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `this.effectState.lostFocus`.
pub const LOST_FOCUS: usize = 0;

/// `onStart(pokemon)`: `-singleturn|MON|move: Focus Punch`.
// PS: data/moves.ts focuspunch.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::FOCUSPUNCH.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw::NONE });
    None
}

/// `onHit(pokemon, source, move)`: a damaging move breaks the focus.
// PS: data/moves.ts focuspunch.condition.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if event_move_category(b, a) != Some(MoveCategory::Status) {
        b.effect_state().data[LOST_FOCUS] = 1;
    }
    None
}

/// `onTryAddVolatile(status, pokemon)`: no flinching.
// PS: data/moves.ts focuspunch.condition.onTryAddVolatile
fn on_try_add_volatile(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if a.relay == Some(Relay::Effect(EffectId::Condition(conditions::FLINCH))) {
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Hit, on_hit),
    Handler::on(EventId::TryAddVolatile, on_try_add_volatile),
]);
