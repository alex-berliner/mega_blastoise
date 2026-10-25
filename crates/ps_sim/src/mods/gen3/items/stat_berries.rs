//! Gen 3 stat-boosting berries: Liechi (atk), Ganlon (def), Salac (spe),
//! Petaya (spa), Apicot (spd), Lansat (focus energy), Starf (random).
//!
//! - The base `onUpdate` (heal at /4 or Gluttony /2, no gen 3 Gluttony)
//!   is dropped in `data/mods/gen3/items.ts`; only `onResidual` fires
//!   the eat at `<= maxhp / 4`.
//! - `onEat` from base boosts the matching stat by one (Lansat adds
//!   `focusenergy`; Starf samples among the boost-cap-not-reached
//!   stats and +2).
//!
//! PS: data/items.ts (each berry's `onEat`), data/mods/gen3/items.ts
//! (the /4 `onResidual` and the dropped `onUpdate`).

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

const RESIDUAL_ORDER: i8 = 10;
const RESIDUAL_SUB_ORDER: i8 = 4;

// `Battle::boost`'s `[i8; 7]` order: atk, def, spa, spd, spe, accuracy, evasion.
const ATK: usize = 0;
const DEF: usize = 1;
const SPA: usize = 2;
const SPD: usize = 3;
const SPE: usize = 4;

fn target(a: &EventArgs) -> MonRef {
    a.target.mon().expect("item handler target")
}

// `onResidual(pokemon)`: eat at hp <= maxhp/4. Shared by every berry here.
// PS: data/mods/gen3/items.ts liechiberry (and the others) onResidual
fn on_residual_quarter(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    if i32::from(b.mon(mon).hp) <= i32::from(b.mon(mon).maxhp) / 4 {
        b.eat_item(mon, false, None, EffectId::None);
    }
    None
}

fn boost_in_place(boosts: &mut [i8; 7], stat: usize, by: i8) {
    boosts[stat] = by;
}

// =================== per-berry onEat handlers =========================

// PS: data/items.ts liechiberry onEat
fn eat_liechi(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, ATK, 1);
    b.boost(ar, mon, None, EffectId::None);
    None
}
// PS: data/items.ts ganlonberry onEat
fn eat_ganlon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, DEF, 1);
    b.boost(ar, mon, None, EffectId::None);
    None
}
// PS: data/items.ts salacberry onEat
fn eat_salac(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, SPE, 1);
    b.boost(ar, mon, None, EffectId::None);
    None
}
// PS: data/items.ts petayaberry onEat
fn eat_petaya(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, SPA, 1);
    b.boost(ar, mon, None, EffectId::None);
    None
}
// PS: data/items.ts apicotberry onEat
fn eat_apicot(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, SPD, 1);
    b.boost(ar, mon, None, EffectId::None);
    None
}
// PS: data/items.ts lansatberry onEat
fn eat_lansat(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    if let Some(c) = b.dex.condition_id("focusenergy") {
        b.add_volatile(mon, c, None, EffectId::None);
    }
    None
}
// PS: data/items.ts starfberry onEat
fn eat_starf(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let mon = target(a);
    let boosts = b.mon(mon).boosts;
    let mut candidates: [usize; 5] = [ATK, DEF, SPA, SPD, SPE];
    let mut n = 0;
    for s in 0..5 {
        let stat = match s {
            0 => ATK,
            1 => DEF,
            2 => SPA,
            3 => SPD,
            _ => SPE,
        };
        if boosts[stat] < 6 {
            candidates[n] = stat;
            n += 1;
        }
    }
    if n == 0 {
        return None;
    }
    let pick = *b.prng.sample(&candidates[..n]);
    let mut ar = [0i8; 7];
    boost_in_place(&mut ar, pick, 2);
    b.boost(ar, mon, None, EffectId::None);
    None
}

// =================== per-berry EffectDefs ============================

pub static LIECHI: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_liechi),
]);
pub static GANLON: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_ganlon),
]);
pub static SALAC: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_salac),
]);
pub static PETAYA: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_petaya),
]);
pub static APICOT: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_apicot),
]);
pub static LANSAT: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_lansat),
]);
pub static STARF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual_quarter).order(RESIDUAL_ORDER).sub_order(RESIDUAL_SUB_ORDER),
    Handler::on(EventId::Eat, eat_starf),
]);
