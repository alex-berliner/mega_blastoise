//! Paralysis: the `-status` line, the quartered speed and the
//! full-paralysis check before a move (a quarter of the time).
//!
//! The paralysis keeps no state of its own.
//!
//! PS: data/conditions.ts par (onStart), data/mods/gen4/conditions.ts par
//! (onModifySpe, onBeforeMove)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Arg, ArgValue, Msg, Status};

use super::add_status_line;

/// `onStart(target, source, sourceEffect)`: the `-status` line.
// PS: data/conditions.ts par.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    add_status_line(b, a, Status::Par)
}

/// `onModifySpe(spe, pokemon)`: `chainModify(0.25)` (Quick Feet, which
/// would keep the speed, is a gen 4 ability).
// PS: data/mods/gen4/conditions.ts par.onModifySpe
fn on_modify_spe(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let quickfeet = b.dex.ability_id("quickfeet");
    if quickfeet.is_some_and(|q| b.has_ability(pokemon, q)) {
        return a.relay;
    }
    b.chain_modify_f(0.25);
    None
}

/// `onBeforeMove(pokemon)`: the full-paralysis check (Magic Guard, which
/// would skip it, is a gen 4 ability).
// PS: data/mods/gen4/conditions.ts par.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let magicguard = b.dex.ability_id("magicguard");
    if magicguard.is_some_and(|m| b.has_ability(pokemon, m)) {
        return None;
    }
    if b.prng.random_chance(1, 4) {
        let mon = b.ident(pokemon);
        b.add(Msg::Cant { mon, reason: Arg::new(ArgValue::Status(Status::Par)), mv: None });
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::ModifySpe, on_modify_spe).priority(-101),
    Handler::on(EventId::BeforeMove, on_before_move).priority(1),
]);
