//! Two-turn moves (Skull Bash, Solar Beam, ...): the volatile that holds
//! the user to the move for its second turn.
//!
//! `onStart` remembers the move that charged and marks the move line
//! `[still]` (a charging move does not show its target yet); `onLockMove`
//! hands the move back, so the request offers it as the only choice and
//! the pipeline knows which move the user is locked into.
//!
//! Effect state (`this.effectState`):
//! - `move`: the move that charged (`move` in PS);
//! - `sourceEffect`: the move's own `sourceEffect` (`sourceEffect` in PS).
//!
//! PS: data/mods/gen1/conditions.ts twoturnmove (lines 306-318)

use ps_dex::gen1::conditions;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(attacker, defender, effect)`.
// PS: data/mods/gen1/conditions.ts twoturnmove.onStart (lines 309-314)
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // `this.effectState.move = effect.id`: the move that charged. The
    // event's source effect is that move (the volatile is added by the
    // move's own `onTryMove`).
    let EffectId::Move(m) = a.effect else { return None };
    // `this.effectState.sourceEffect = effect.sourceEffect`: the move's
    // own `sourceEffect` (Metronome or Mirror Move when it called it).
    let source_effect = b.active_move().map_or(EffectId::None, |am| am.source_effect);
    {
        let st = b.effect_state();
        st.move_ = m;
        st.source_effect = source_effect;
    }
    // `this.attrLastMove('[still]')`: no animation, so the move's target
    // is not shown yet.
    b.attr_last_move_still();
    None
}

/// `onLockMove()`.
// PS: data/mods/gen1/conditions.ts twoturnmove.onLockMove (lines 315-317)
fn on_lock_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `return this.effectState.move`: the move id, as PS's string.
    let m = b.effect_state().move_;
    if m.is_none() {
        return None;
    }
    Some(Relay::Move(m))
}

/// The charge of a two-turn move, shared by the six moves that have it
/// (Dig, Fly, Razor Wind, Skull Bash, Sky Attack and Solar Beam): the
/// first turn charges, the second one hits.
///
/// `invulnerable` is the `attacker.addVolatile('invulnerability', defender)`
/// line, which only Dig and Fly have; the other four keep the
/// `removeVolatile('invulnerability')` line all six share.
///
/// PS: data/mods/gen1/moves.ts dig (lines 235-248), fly (329-341),
/// razorwind (625-637), skullbash (726-736), skyattack (738-748) and
/// solarbeam (761-771): their `onTryMove`.
pub fn charge(b: &mut Battle, a: &EventArgs, invulnerable: bool) -> Option<Relay> {
    let attacker = a.target.mon()?;
    // `if (attacker.removeVolatile('twoturnmove')) {`
    if b.remove_volatile(attacker, conditions::TWOTURNMOVE) {
        // `attacker.removeVolatile('invulnerability');`
        b.remove_volatile(attacker, conditions::INVULNERABILITY);
        // `return;`: the move goes on (the relay stays `true`).
        return None;
    }
    // `this.add('-prepare', attacker, move.name)`.
    let EffectId::Move(m) = a.effect else { return None };
    let mon = b.ident(attacker);
    b.add(ps_protocol::Msg::Prepare { mon, mv: ps_protocol::MoveNum(m.0), target: None });
    // `attacker.addVolatile('twoturnmove', defender)`.
    b.add_volatile(attacker, conditions::TWOTURNMOVE, a.source, EffectId::None);
    // `attacker.addVolatile('invulnerability', defender)`.
    if invulnerable {
        b.add_volatile(attacker, conditions::INVULNERABILITY, a.source, EffectId::None);
    }
    // `return null;`: the move does not happen this turn.
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::LockMove, on_lock_move),
]);
