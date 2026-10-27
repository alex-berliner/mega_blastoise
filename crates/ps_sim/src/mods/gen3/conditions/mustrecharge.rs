//! `mustrecharge`: Hyper Beam, Blast Burn, Frenzy Plant and Hydro Cannon's
//! recharge turn (their `self.volatileStatus` adds it). `duration: 2`; the
//! request offers only `recharge` (`onLockMove`).
//!
//! No effect state.
//!
//! PS: data/mods/gen4/conditions.ts mustrecharge (onBeforeMove),
//! data/conditions.ts mustrecharge (onStart, onLockMove)

use ps_protocol::{Arg, Msg, Word};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onBeforeMove(pokemon)`: the recharge turn (gen 4 keeps Truant's
/// volatile, unlike base).
// PS: data/mods/gen4/conditions.ts mustrecharge.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::word(Word::Recharge), mv: None, kw: ps_protocol::Kw::NONE });
    b.remove_volatile(pokemon, conditions::MUSTRECHARGE);
    Some(Relay::Null)
}

/// `onStart(pokemon)`: the `-mustrecharge` line.
// PS: data/conditions.ts mustrecharge.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::MustRecharge(mon));
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeMove, on_before_move).priority(11),
    Handler::value(EventId::LockMove, Relay::Recharge),
    Handler::on(EventId::Start, on_start),
]);
