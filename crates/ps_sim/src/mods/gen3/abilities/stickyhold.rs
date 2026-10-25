//! Sticky Hold: another Pokemon cannot take the holder's item.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/mods/gen4/abilities.ts stickyhold (onTakeItem)

use ps_dex::gen3::moves;
use ps_protocol::{Kw, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::this_ability_arg;

/// `onTakeItem(item, pokemon, source)`: `if (source && source !== pokemon
/// || this.activeMove?.id === "knockoff")`, `-activate|MON|ability: Sticky
/// Hold` and `false`.
fn on_take_item(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let by_other = a.source.is_some_and(|source| source != pokemon);
    let knockoff = b.active_move().is_some_and(|mv| mv.id == moves::KNOCKOFF);
    if !by_other && !knockoff {
        return None;
    }
    let mon = b.ident(pokemon);
    let effect = this_ability_arg(b);
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TakeItem, on_take_item)]);
