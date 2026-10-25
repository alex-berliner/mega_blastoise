//! Oblivious: the holder cannot be infatuated (`Immunity` to `attract`);
//! an infatuation it has anyway ends at the next `Update`. Captivate (gen
//! 4) is the move it would also block (`onTryHit`), so that handler never
//! answers in gen 3.
//!
//! No ability state.
//!
//! PS: data/mods/gen5/abilities.ts oblivious (onUpdate, onTryHit),
//! data/abilities.ts oblivious (onImmunity)

use ps_dex::gen3::conditions as c;
use ps_protocol::{AbilityNum, Arg, Kw, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{event_move, relay_is, this_ability_arg};

/// `onUpdate(pokemon)`: `removeVolatile('attract')`, then `-end|MON|move:
/// Attract|[from] ability: Oblivious`.
// PS: data/mods/gen5/abilities.ts oblivious.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.has_vol(pokemon, c::ATTRACT) {
        b.remove_volatile(pokemon, c::ATTRACT);
        let mon = b.ident(pokemon);
        let attract = b.dex.move_id("attract").expect("Attract in the dex");
        let effect = Arg::move_effect(ps_protocol::MoveNum(attract.0));
        let kw = Kw::from(this_ability_arg(b));
        b.add(Msg::EndEffect { mon, effect, kw });
    }
    None
}

/// `onImmunity(type, pokemon)`: `false` for `attract`.
// PS: data/abilities.ts oblivious.onImmunity
fn on_immunity(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_is(a, c::ATTRACT).then_some(Relay::FAIL)
}

/// `onTryHit(pokemon, target, move)`: Captivate shows `-immune|MON|[from]
/// Oblivious` (the plain name) and stops.
// PS: data/mods/gen5/abilities.ts oblivious.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mv = event_move(b, a)?;
    if b.dex.move_id("captivate") != Some(mv.id) {
        return None;
    }
    let EffectId::Ability(id) = b.frame.effect else { return None };
    let mon = b.ident(pokemon);
    b.add(Msg::Immune { mon, what: None, kw: Kw::from(Arg::ability(AbilityNum(id.0))) });
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::Immunity, on_immunity),
    Handler::on(EventId::TryHit, on_try_hit),
]);
