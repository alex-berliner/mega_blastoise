//! `focusband`: Focus Band, a one in ten chance to survive a move's hit
//! at 1 HP.
//!
//! PS: data/items.ts focusband

use ps_dex::gen3::items;
use ps_protocol::{Arg, ArgValue, ItemNum, Kw, Msg};

use crate::effect::{EffectDef, EffectType, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onDamage(damage, target, source, effect)`: `if (this.randomChance(1,
/// 10) && damage >= target.hp && effect?.effectType === "Move") {
/// this.add("-activate", target, "item: Focus Band"); return target.hp -
/// 1; }`. The roll comes first: every damage the holder takes draws it.
/// Confusion's self-hit passes `{id: 'confused', effectType: 'Move'}`, so
/// it counts as a move here (`EffectType::of` maps `Pseudo::Confused`).
// PS: data/items.ts focusband.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let damage = match a.relay {
        Some(Relay::Int(n)) => n,
        other => panic!("focusband: Damage relay {other:?}"),
    };
    if !b.prng.random_chance(1, 10) {
        return None;
    }
    let hp = i32::from(b.mon(target).hp);
    if damage < hp || b.effect_type(a.effect) != EffectType::Move {
        return None;
    }
    let mon = b.ident(target);
    b.add(Msg::Activate { mon: Some(mon), effect: Arg::new(ArgValue::ItemEffect(ItemNum(items::FOCUSBAND.0))), a: None, b: None, kw: Kw::NONE });
    Some(Relay::Int(hp - 1))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Damage, on_damage).priority(-40)]);
