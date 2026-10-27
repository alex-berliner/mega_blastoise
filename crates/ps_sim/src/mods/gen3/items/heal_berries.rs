//! Gen 3 heal berries: Oran, Sitrus, Figy, Wiki, Mago, Aguav, Iapapa.
//!
//! - Trigger: each berry's `onResidual` (gen 3 override) eats when
//!   `pokemon.hp <= pokemon.maxhp / 2`. The base `onUpdate` is dropped
//!   (`onUpdate: undefined`); Gluttony is a gen 6 ability so it is not
//!   reachable in gen 3.
//! - `onTryEatItem` from base runs `runEvent('TryHeal', pokemon, null,
//!   this.effect, N)` once with the planned amount (Oran: 10, Sitrus:
//!   maxhp/4, the flavor berries: maxhp/3); on failure the eat is
//!   aborted (`return false`).
//! - `onEat`: Oran +10, Sitrus +30 (gen 3 mod replaces base /4),
//!   Figy/Wiki/Mago/Aguav/Iapapa heal `baseMaxhp / 8` and add
//!   `confusion` when the holder's nature's minus stat matches the
//!   berry's slot.
//!
//! PS: data/items.ts (the base entries), data/mods/gen3/items.ts (the
//! `/2` `onResidual` and Sitrus's `onEat`).

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};

const ORDER: i8 = 10;
const SUB_ORDER: i8 = 4;

fn target(a: &EventArgs) -> MonRef {
    a.target.mon().expect("heal berry target")
}

// ============= onResidual /2 (shared) =================================

/// `if (pokemon.hp <= pokemon.maxhp / 2) pokemon.eatItem()` (integer HP:
/// the floor of the half compares the same).
// PS: data/mods/gen3/items.ts oranberry (and the others) onResidual
fn residual_half(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    if i32::from(b.mon(mon).hp) <= i32::from(b.mon(mon).maxhp) / 2 {
        b.eat_item(mon, false, None, EffectId::None);
    }
    None
}

// ============= onTryEatItem (per berry) ================================

/// `if (!this.runEvent('TryHeal', pokemon, null, this.effect, amount))
/// return false`. PS's amount is the float `baseMaxhp / N`; the relay
/// carries its floor, raised to 1 so a fraction stays truthy (a 1 HP
/// Shedinja's 0.25). No gen 3 handler reads the amount here: Liquid Ooze's
/// is `onSourceTryHeal`, and the source is null.
// PS: data/items.ts oranberry (and the others) onTryEatItem
fn try_heal_passes(b: &mut Battle, mon: MonRef, amount: i32) -> bool {
    b.run_event(EventId::TryHeal, Holder::Mon(mon), None, b.frame.effect, Some(Relay::Int(amount.max(1)))).truthy()
}

fn try_eat_10(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    (!try_heal_passes(b, mon, 10)).then_some(Relay::FAIL)
}
fn try_eat_qrt(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let amount = i32::from(b.mon(mon).base_maxhp) / 4;
    (!try_heal_passes(b, mon, amount)).then_some(Relay::FAIL)
}
fn try_eat_third(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let amount = i32::from(b.mon(mon).base_maxhp) / 3;
    (!try_heal_passes(b, mon, amount)).then_some(Relay::FAIL)
}

// ============= Heal helpers ============================================

/// `this.heal(amount)`: target, source and effect from the running event
/// (`battle.heal` fills them in), so the line is `[from] item: X`.
fn heal(b: &mut Battle, amount: i32) {
    b.heal(amount, None, None, EffectId::None);
}

/// `if (pokemon.getNature().minus === STAT) pokemon.addVolatile('confusion')`.
/// `stat_minus` indexes [`crate::stats::NATURES`] (1 atk .. 5 spe).
fn confusion_if_minus(mon: MonRef, stat_minus: u8, b: &mut Battle) {
    let Some(nature) = b.mon(mon).nature else { return };
    let (_name, _plus, maybe_minus) = crate::stats::NATURES[nature as usize];
    if maybe_minus != Some(stat_minus) {
        return;
    }
    let c = b.dex.condition_id("confusion").expect("confusion in the dex");
    b.add_volatile(mon, c, None, EffectId::None);
}

/// `this.heal(pokemon.baseMaxhp / 8)`: `battle.heal` truncates.
fn eighth(b: &Battle, mon: MonRef) -> i32 {
    i32::from(b.mon(mon).base_maxhp) / 8
}

// ============= Per-berry eat handlers ==================================

// PS: data/items.ts oranberry onEat
fn eat_oran(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    heal(b, 10);
    None
}
// PS: data/mods/gen3/items.ts sitrusberry onEat
fn eat_sitrus(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    heal(b, 30);
    None
}
// PS: data/mods/gen6/items.ts figyberry (wiki, mago, aguav, iapapa) onEat
fn eat_figy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    heal(b, eighth(b, mon));
    confusion_if_minus(mon, 1, b);
    None
}
fn eat_wiki(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    heal(b, eighth(b, mon));
    confusion_if_minus(mon, 3, b);
    None
}
fn eat_mago(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    heal(b, eighth(b, mon));
    confusion_if_minus(mon, 5, b);
    None
}
fn eat_aguav(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    heal(b, eighth(b, mon));
    confusion_if_minus(mon, 4, b);
    None
}
fn eat_iapapa(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    heal(b, eighth(b, mon));
    confusion_if_minus(mon, 2, b);
    None
}

pub static ORANBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_10),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_oran),
]);
pub static SITRUSBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_qrt),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_sitrus),
]);
pub static FIGYBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_third),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_figy),
]);
pub static WIKIBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_third),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_wiki),
]);
pub static MAGOBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_third),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_mago),
]);
pub static AGUAVBERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_third),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_aguav),
]);
pub static IAPAPABERRY: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryEatItem, try_eat_third),
    Handler::on(EventId::Residual, residual_half).order(ORDER).sub_order(SUB_ORDER),
    Handler::on(EventId::Eat, eat_iapapa),
]);
