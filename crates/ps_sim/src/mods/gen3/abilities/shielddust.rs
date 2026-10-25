//! Shield Dust: the holder is not hit by the secondary effects of other
//! Pokemon's moves (the ones that hit the user itself still happen).
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts shielddust (onModifySecondaries)

use ps_protocol::{Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySecondaries(secondaries)`: `secondaries.filter((effect) =>
/// !!effect.self)`, the list a mask over the move's dex secondaries.
fn on_modify_secondaries(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(Relay::Secondaries(mask)) = a.relay else { panic!("ModifySecondaries relay {:?}", a.relay) };
    b.debug(Text::new(TextId::ShieldDustPrevent));
    let EffectId::Move(m) = a.effect else { panic!("ModifySecondaries of {:?}", a.effect) };
    let mut kept = 0u8;
    for (i, secondary) in b.dex.move_def(m).secondaries().iter().enumerate() {
        if mask & (1 << i) != 0 && secondary.self_effect.is_some() {
            kept |= 1 << i;
        }
    }
    Some(Relay::Secondaries(kept))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySecondaries, on_modify_secondaries)]);
