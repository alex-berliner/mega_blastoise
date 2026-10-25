//! `spikes`: Spikes' side condition. A grounded Pokemon switching in to
//! the side loses an eighth, sixth or quarter of its maximum HP for one,
//! two or three layers. No duration.
//!
//! Effect state: `layers` is `layers`.
//!
//! PS: data/moves.ts spikes.condition (onSideStart, onSideRestart),
//! data/mods/gen4/moves.ts spikes.condition (onEntryHazard)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::moves;

fn add_side_start(b: &mut Battle, side: u8) {
    let side = b.sides[side as usize].id();
    b.add(Msg::SideStart { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::SPIKES.0))), kw: Kw::NONE });
}

/// `onSideStart(side)`: `-sidestart|SIDE|Spikes`, one layer.
// PS: data/moves.ts spikes.condition.onSideStart
fn on_side_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    add_side_start(b, side);
    b.effect_state().layers = 1;
    None
}

/// `onSideRestart(side)`: another layer, up to three.
// PS: data/moves.ts spikes.condition.onSideRestart
fn on_side_restart(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    if b.effect_state().layers >= 3 {
        return Some(Relay::FAIL);
    }
    add_side_start(b, side);
    b.effect_state().layers += 1;
    None
}

/// `onEntryHazard(pokemon)`: `damageAmounts[layers] * pokemon.maxhp / 24`
/// to a grounded Pokemon (Heavy-Duty Boots are gen 8). `damage` makes
/// anything in (0, 1] one and truncates the rest.
// PS: data/mods/gen4/moves.ts spikes.condition.onEntryHazard
fn on_entry_hazard(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let boots = b.dex.item_id("heavydutyboots");
    if b.is_grounded(pokemon, false) != Some(true) || boots.is_some_and(|i| b.has_item(pokemon, i)) {
        return None;
    }
    const DAMAGE_AMOUNTS: [i32; 4] = [0, 3, 4, 6];
    let layers = usize::from(b.effect_state().layers);
    let damage = DAMAGE_AMOUNTS[layers] * i32::from(b.mon(pokemon).maxhp);
    b.damage((damage / 24).max(1), None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::SideStart, on_side_start),
    Handler::on(EventId::SideRestart, on_side_restart),
    Handler::on(EventId::EntryHazard, on_entry_hazard).priority(2),
]);
