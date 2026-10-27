//! `mail`: Mail, which only Knock Off, Thief and Covet can take.
//!
//! PS: data/items.ts mail

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTakeItem(item, source)`: `if (!this.activeMove) return false; if
/// (this.activeMove.id !== "knockoff" && this.activeMove.id !== "thief" &&
/// this.activeMove.id !== "covet") return false`.
// PS: data/items.ts mail.onTakeItem
fn on_take_item(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let Some(mv) = b.active_move() else { return Some(Relay::FAIL) };
    if mv.id.is_none() {
        return Some(Relay::FAIL);
    }
    match b.dex.move_def(mv.id).name.as_str() {
        "Knock Off" | "Thief" | "Covet" => None,
        _ => Some(Relay::FAIL),
    }
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TakeItem, on_take_item)]);
