//! Flinch: the Pokemon loses its turn. A `duration: 1` volatile, so it
//! lasts the rest of the turn it was added in, and keeps no state.
//!
//! PS: data/conditions.ts flinch

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_protocol::{Arg, ArgValue, Msg, Word};

/// `onBeforeMove(pokemon)`: the `cant` line, then the `Flinch` event
/// (Steadfast, gen 4, answers it), and the move does not happen.
// PS: data/conditions.ts flinch.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::new(ArgValue::Word(Word::Flinch)), mv: None });
    b.run_event(EventId::Flinch, Holder::Mon(pokemon), None, EffectId::None, None);
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BeforeMove, on_before_move).priority(8)]);
