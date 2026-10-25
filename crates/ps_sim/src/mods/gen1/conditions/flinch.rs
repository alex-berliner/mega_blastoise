//! Flinch: the Pokemon loses its turn when a move startles it.
//!
//! The flinch is a volatile with `duration: 1`, so it lasts for the rest
//! of the turn it was added in. It keeps no state of its own.
//!
//! PS: data/mods/gen1/conditions.ts flinch (line 166)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen1::conditions;
use ps_protocol::{Arg, ArgValue, Msg, Word};

/// `onStart(target)`: `target.removeVolatile('mustrecharge')`.
// PS: data/mods/gen1/conditions.ts flinch.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.remove_volatile(target, conditions::MUSTRECHARGE);
    None
}

/// `onBeforeMove(pokemon)`: the flinched Pokemon cannot move.
// PS: data/mods/gen1/conditions.ts flinch.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // `if (!this.runEvent('Flinch', pokemon)) return;`: a handler that
    // stops the Flinch event lets the Pokemon move after all.
    if !b.run_event(EventId::Flinch, Holder::Mon(pokemon), None, crate::effect::EffectId::None, None).truthy() {
        return None;
    }
    let mon = b.ident(pokemon);
    let reason = Arg::new(ArgValue::Word(Word::Flinch));
    b.add(Msg::Cant { mon, reason, mv: None, kw: ps_protocol::Kw::NONE });
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move).priority(8),
]);
