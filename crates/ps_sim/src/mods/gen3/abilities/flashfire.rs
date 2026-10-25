//! Flash Fire: a Fire move aimed at the holder is absorbed (the move stops,
//! `null`) and the holder's own Fire moves deal 1.5x from then on (the
//! `flashfire` volatile, see `conditions/flashfire.rs`). A second Fire hit
//! only shows `-immune`. A frozen holder does not absorb, and Will-O-Wisp is
//! not absorbed by a holder it would fail on anyway.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts flashfire (onTryHit), data/abilities.ts
//! flashfire (onEnd)

use ps_dex::gen3::conditions as c;
use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, event_move};

/// `onTryHit(target, source, move)`.
// PS: data/mods/gen3/abilities.ts flashfire.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mv = event_move(b, a)?;
    if target == source || mv.type_ != Type::Fire {
        return None;
    }
    if b.dex.move_id("willowisp") == Some(mv.id)
        && (b.has_type(target, Type::Fire) || !b.mon(target).status.is_none() || b.has_vol(target, c::SUBSTITUTE))
    {
        return None;
    }
    if b.mon(target).status == c::FRZ {
        return None;
    }
    if !b.add_volatile(target, c::FLASHFIRE, None, crate::effect::EffectId::None).truthy() {
        add_immune(b, target);
    }
    Some(Relay::Null)
}

/// `onEnd(pokemon)`: `pokemon.removeVolatile('flashfire')`.
// PS: data/abilities.ts flashfire.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, c::FLASHFIRE);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::End, on_end),
]);
