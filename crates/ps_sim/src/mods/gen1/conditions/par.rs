//! Paralysis: the `-status` line and the full-paralysis check before a
//! move (a quarter of the time the Pokemon cannot move).
//!
//! The paralysis keeps no state of its own.
//!
//! PS: data/mods/gen1/conditions.ts par (line 30)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;
use ps_protocol::{Arg, ArgValue, Kw, Msg, Status, TextId};

/// `onStart(target)`: `this.add('-status', target, 'par')`.
// PS: data/mods/gen1/conditions.ts par.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::SetStatus { mon, status: Status::Par, kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon)`: the full-paralysis check.
// PS: data/mods/gen1/conditions.ts par.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if !b.prng.random_chance(63, 256) {
        return None;
    }
    let mon = b.ident(pokemon);
    let reason = Arg::new(ArgValue::Status(Status::Par));
    b.add(Msg::Cant { mon, reason, mv: None, kw: Kw::NONE });
    b.remove_volatile(pokemon, conditions::BIDE);
    if b.remove_volatile(pokemon, conditions::TWOTURNMOVE)
        && b.has_volatile_key(pokemon, "invulnerability")
    {
        b.hint(TextId::HintGen1ParSemiInvulnerable, true, None);
    }
    b.remove_volatile(pokemon, conditions::PARTIALTRAPPINGLOCK);
    b.remove_volatile(pokemon, conditions::LOCKEDMOVE);
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move).priority(2),
]);
