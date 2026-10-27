//! `counter`: Counter's volatile, readied before the turn (`duration: 1`,
//! `noCopy`). It records the last physical move (or Hidden Power) a foe
//! hit the holder with this turn: the attacker's slot and twice the
//! damage.
//!
//! Effect state: `slot` is `data[SLOT]` (`1 + side * 8 + position`, 0 for
//! `null`); `damage` is `data[DAMAGE]` and `data[DAMAGE + 1]`, the low and
//! high 16 bits.
//!
//! PS: data/moves.ts counter.condition (`onStart`), data/mods/gen3/moves.ts
//! counter.condition (`onRedirectTarget`, `onDamage`)

use ps_dex::MoveCategory;

use crate::effect::{EffectDef, EffectId, EffectType, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, EffectState};
use ps_dex::gen3::moves;

/// `this.effectState.slot`.
pub const SLOT: usize = 0;
/// `this.effectState.damage`, two words.
pub const DAMAGE: usize = 1;

/// `this.effectState.damage`.
pub fn damage(st: &EffectState) -> i32 {
    i32::from(st.data[DAMAGE] as u16) | (i32::from(st.data[DAMAGE + 1] as u16) << 16)
}

fn set_damage(st: &mut EffectState, damage: i32) {
    st.data[DAMAGE] = damage as u16 as i16;
    st.data[DAMAGE + 1] = (damage >> 16) as u16 as i16;
}

/// `onStart(target, source, move)`: `slot = null`, `damage = 0`.
// PS: data/moves.ts counter.condition.onStart (mirrorcoat's is the same)
pub fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    st.data[SLOT] = 0;
    set_damage(st, 0);
    None
}

/// `onRedirectTarget(target, source, source2)`: the recorded slot's
/// Pokemon. PS runs `RedirectTarget` only with `activePerHalf > 1`.
// PS: data/mods/gen3/moves.ts counter.condition.onRedirectTarget
pub fn on_redirect_target(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    unreachable!("RedirectTarget runs only in doubles")
}

/// The `onDamage` Counter and Mirror Coat share: a foe's move that `counts`
/// (by its category and id) records its slot and twice the damage.
pub fn record_damage(b: &mut Battle, a: &EventArgs, counts: fn(MoveCategory, bool) -> bool) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.effect_type(a.effect) != EffectType::Move {
        return None;
    }
    let source = a.source?;
    // `source.isAlly(target)`: the same side, in singles.
    if source.side == target.side {
        return None;
    }
    let EffectId::Move(id) = a.effect else { return None };
    // `effect.category`: the move in flight's (Hidden Power's and Weather
    // Ball's are set by their `onModifyMove`), else the data's (a plain
    // `moveData` object: Bide's strike, a future move's hit).
    let category = b.active_move().filter(|am| am.id == id).map_or(b.dex.move_def(id).category, |am| am.category);
    if !counts(category, id == moves::HIDDENPOWER) {
        return None;
    }
    let Some(Relay::Int(dealt)) = a.relay else { panic!("onDamage without a damage number: {:?}", a.relay) };
    let slot = 1 + i16::from(source.side) * 8 + i16::from(b.mon(source).position);
    let st = b.effect_state();
    st.data[SLOT] = slot;
    set_damage(st, 2 * dealt);
    None
}

/// `onDamage(damage, target, source, effect)`: a foe's physical move or
/// Hidden Power.
// PS: data/mods/gen3/moves.ts counter.condition.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    record_damage(b, a, |category, hidden_power| category == MoveCategory::Physical || hidden_power)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::RedirectTarget, on_redirect_target).priority(-1),
    Handler::on(EventId::Damage, on_damage).priority(-101),
]);
