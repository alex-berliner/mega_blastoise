//! Gen 3 status-curing berries: Cheri (par), Chesto (slp), Pecha (psn
//! or tox), Rawst (brn), Aspear (frz). Each fires on `Update` if the
//! holder has the right status, then cures it on `Eat`. No gen 3
//! override (so the base handlers carry through unchanged).
//!
//! PS: data/items.ts (the berry entries).

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

/// True when the holder has the status that matches `want`, where
/// `want = "psn"` also matches `"tox"`.
fn status_match(b: &Battle, mon: MonRef, want: &str) -> bool {
    let status = b.mon(mon).status;
    if status.is_none() { return false; }
    let held = b.dex.condition_def(status).key.as_str();
    held == want || (want == "psn" && held == "tox")
}

fn update_cheri(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_update(b, a, "par") }
fn update_chesto(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_update(b, a, "slp") }
fn update_pecha(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_update(b, a, "psn") }
fn update_rawst(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_update(b, a, "brn") }
fn update_aspear(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_update(b, a, "frz") }

// PS: data/items.ts cheriberry (chesto, pecha, rawst, aspear) onUpdate
fn do_update(b: &mut Battle, a: &EventArgs, want: &str) -> Option<Relay> {
    let mon = a.target.mon().expect("status berry onUpdate target");
    if !status_match(b, mon, want) { return None; }
    b.eat_item(mon, false, None, crate::effect::EffectId::None);
    None
}

fn eat_cheri(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_eat(b, a, "par") }
fn eat_chesto(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_eat(b, a, "slp") }
fn eat_pecha(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_eat(b, a, "psn") }
fn eat_rawst(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_eat(b, a, "brn") }
fn eat_aspear(b: &mut Battle, a: &EventArgs) -> Option<Relay> { do_eat(b, a, "frz") }

// PS: data/items.ts cheriberry (chesto, pecha, rawst, aspear) onEat
fn do_eat(b: &mut Battle, a: &EventArgs, want: &str) -> Option<Relay> {
    let mon = a.target.mon().expect("status berry onEat target");
    if !status_match(b, mon, want) { return None; }
    b.cure_status(mon, false);
    None
}

pub static CHERI: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, update_cheri),
    Handler::on(EventId::Eat, eat_cheri),
]);
pub static CHESTO: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, update_chesto),
    Handler::on(EventId::Eat, eat_chesto),
]);
pub static PECHA: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, update_pecha),
    Handler::on(EventId::Eat, eat_pecha),
]);
pub static RAWST: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, update_rawst),
    Handler::on(EventId::Eat, eat_rawst),
]);
pub static ASPEAR: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Update, update_aspear),
    Handler::on(EventId::Eat, eat_aspear),
]);
