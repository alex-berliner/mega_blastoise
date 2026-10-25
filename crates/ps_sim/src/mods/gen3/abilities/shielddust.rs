//! Shield Dust: the holder is not hit by the secondary effects of other
//! Pokemon's moves (the ones that hit the user itself still happen).
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts shielddust (onModifySecondaries)

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::event_move;

/// `onModifySecondaries(secondaries)`: `secondaries.filter((effect) =>
/// !!effect.self)`, the list a mask over the move's secondaries (the
/// dex's, then King's Rock's added flinch: [`crate::actions::ActiveMove::secondaries`]).
fn on_modify_secondaries(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(Relay::Secondaries(mask)) = a.relay else { panic!("ModifySecondaries relay {:?}", a.relay) };
    b.debug(Text::new(TextId::ShieldDustPrevent));
    let mv = event_move(b, a).unwrap_or_else(|| panic!("ModifySecondaries of {:?}, not the active move", a.effect));
    let mut kept = 0u8;
    for (i, secondary) in mv.secondaries(b.dex).enumerate() {
        if mask & (1 << i) != 0 && secondary.self_effect.is_some() {
            kept |= 1 << i;
        }
    }
    Some(Relay::Secondaries(kept))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySecondaries, on_modify_secondaries)]);
