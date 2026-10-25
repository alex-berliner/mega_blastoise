//! Pressure: a foe's move aimed at the holder costs one more PP
//! (`DeductPP`), and its switch-in line is a silent `-ability`, sent to the
//! holder's own side only (`addSplit`). `onSwitchInPriority: 3`.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts pressure (onStart),
//! data/mods/gen4/abilities.ts pressure (onDeductPP)

use ps_protocol::{AbilityNum, Kw, KwFlags, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(pokemon)`: `this.addSplit(pokemon.side.id, ['-ability',
/// pokemon, 'Pressure', '[silent]'])`.
// PS: data/mods/gen3/abilities.ts pressure.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let EffectId::Ability(id) = b.frame.effect else { unreachable!("Pressure's own handler") };
    let mon = b.ident(pokemon);
    let split = Some(mon.side);
    b.add(Msg::Ability { mon, ability: AbilityNum(id.0), old: None, boost: false, kw: Kw::flags(KwFlags::SILENT), split });
    None
}

/// `onDeductPP(target, source)`: one more PP, unless the holder targets
/// itself.
// PS: data/mods/gen4/abilities.ts pressure.onDeductPP
fn on_deduct_pp(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if Some(target) == a.source {
        return None;
    }
    Some(Relay::Int(1))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::DeductPP, on_deduct_pp),
]);
