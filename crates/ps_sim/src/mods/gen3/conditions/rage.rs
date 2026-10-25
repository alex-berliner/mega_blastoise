//! `rage`: Rage's volatile (the move's `self.volatileStatus`). Each
//! damaging hit from another Pokemon raises the user's Attack, until the
//! user's next move removes it. No effect state.
//!
//! PS: data/moves.ts rage.condition

use ps_dex::MoveCategory;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::event_move_category;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(pokemon)`: `-singlemove|MON|Rage`.
// PS: data/moves.ts rage.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::SingleMove { mon, effect: Arg::new(ArgValue::Move(MoveNum(moves::RAGE.0))), kw: Kw::NONE });
    None
}

/// `onHit(target, source, move)`: `this.boost({atk: 1})` for another
/// Pokemon's damaging move (the event's target and source, and the
/// condition as the effect).
// PS: data/moves.ts rage.condition.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if Some(target) != a.source && event_move_category(b, a) != Some(MoveCategory::Status) {
        let mut boosts = [0i8; 7];
        boosts[0] = 1;
        b.boost(boosts, target, a.source, EffectId::Condition(conditions::RAGE));
    }
    None
}

/// `onBeforeMove(pokemon)`: the user's next move ends it.
// PS: data/moves.ts rage.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.debug(Text::new(TextId::RemovingRage));
    b.remove_volatile(pokemon, conditions::RAGE);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Hit, on_hit),
    Handler::on(EventId::BeforeMove, on_before_move).priority(100),
]);
