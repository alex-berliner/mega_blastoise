//! Own Tempo: the holder cannot be confused (`TryAddVolatile` refuses
//! `confusion` with `null`); a move whose own volatile is confusion
//! (Confuse Ray, Swagger) shows `-immune|MON|confusion|[from] ability: Own
//! Tempo` (`onHit`). A confusion it has anyway ends at the next `Update`.
//!
//! No ability state.
//!
//! PS: data/abilities.ts owntempo

use ps_dex::gen3::conditions as c;
use ps_protocol::{Arg, Kw, Msg, Word};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{event_move, relay_is, this_ability_arg};

/// `onUpdate(pokemon)`: `-activate|MON|ability: Own Tempo`, then
/// `removeVolatile('confusion')`.
// PS: data/abilities.ts owntempo.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.has_vol(pokemon, c::CONFUSION) {
        let mon = b.ident(pokemon);
        let effect = this_ability_arg(b);
        b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
        b.remove_volatile(pokemon, c::CONFUSION);
    }
    None
}

/// `onTryAddVolatile(status, pokemon)`: `null` for confusion.
// PS: data/abilities.ts owntempo.onTryAddVolatile
fn on_try_add_volatile(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    relay_is(a, c::CONFUSION).then_some(Relay::Null)
}

/// `onHit(target, source, move)`: `move?.volatileStatus === 'confusion'`.
// PS: data/abilities.ts owntempo.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mv = event_move(b, a)?;
    if b.dex.move_def(mv.id).volatile_status != c::CONFUSION {
        return None;
    }
    let mon = b.ident(target);
    let kw = Kw::from(this_ability_arg(b));
    b.add(Msg::Immune { mon, what: Some(Arg::word(Word::Confusion)), kw });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::TryAddVolatile, on_try_add_volatile),
    Handler::on(EventId::Hit, on_hit),
]);
