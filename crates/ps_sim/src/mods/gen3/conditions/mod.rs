//! Gen 3 conditions with handlers, one file per condition (statuses,
//! volatiles, slot conditions and weathers, as `dex.conditions` finds
//! them). Each file ports the definition `Dex.mod('gen3')` resolves:
//! `data/mods/gen3/conditions.ts` over `data/mods/gen4/conditions.ts` over
//! the gen 5 to 8 mods over base `data/conditions.ts`
//! (`node tools/ps-diff/show-effect.js gen3 condition <id>` prints it).
//!
//! PS: data/mods/gen3/conditions.ts, data/mods/gen4/conditions.ts,
//! data/conditions.ts.

pub mod attract;
pub mod beatup;
pub mod bide;
pub mod bounce;
pub mod brn;
pub mod charge;
pub mod choicelock;
pub mod confusion;
pub mod curse;
pub mod destinybond;
pub mod dig;
pub mod disable;
pub mod dive;
pub mod encore;
pub mod flashfire;
pub mod flinch;
pub mod fly;
pub mod focuspunch;
pub mod followme;
pub mod foresight;
pub mod frz;
pub mod furycutter;
pub mod futuremove;
pub mod grudge;
pub mod hail;
pub mod helpinghand;
pub mod iceball;
pub mod imprison;
pub mod leechseed;
pub mod lockedmove;
pub mod lockon;
pub mod magiccoat;
pub mod mustrecharge;
pub mod nightmare;
pub mod par;
pub mod partiallytrapped;
pub mod perishsong;
pub mod psn;
pub mod pursuit;
pub mod rage;
pub mod raindance;
pub mod rollout;
pub mod sandstorm;
pub mod semi_invulnerable;
pub mod slp;
pub mod snatch;
pub mod stall;
pub mod stockpile;
pub mod substitute;
pub mod sunnyday;
pub mod taunt;
pub mod torment;
pub mod tox;
pub mod trapped;
pub mod twoturnmove;
pub mod uproar;
pub mod yawn;

use ps_dex::{ConditionId, MoveCategory, Type};
use ps_protocol::{Arg, ArgValue, Kw, Msg, Status, Weather};

use crate::effect::{EffectId, EventArgs};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

/// The `-status` line of a status's base `onStart`:
/// `[from] ability: NAME` and `[of] SOURCE` when an ability set it,
/// otherwise plain.
// PS: data/conditions.ts brn/par/psn/tox/frz onStart
pub fn add_status_line(b: &mut Battle, a: &EventArgs, status: Status) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let kw = match (a.effect, a.source) {
        (EffectId::Ability(_), Some(source)) => Kw::from_of(b.effect_fullname(a.effect), b.ident(source)),
        _ => Kw::NONE,
    };
    b.add(Msg::SetStatus { mon, status, kw });
    None
}

/// `this.damage(pokemon.baseMaxhp / n)` from a residual handler, as an
/// integer: `spreadDamage`'s `clampIntRange(damage, 1)` floors it and makes
/// it at least 1. (`clampIntRange(baseMaxhp / n, 1)` is the same number.)
/// The fraction would reach a `Damage` handler first, but `clampIntRange`
/// runs before `runEvent('Damage')`, so the handler sees this number too.
pub fn fraction_of_base_maxhp(b: &Battle, pokemon: MonRef, n: i32) -> i32 {
    (i32::from(b.mon(pokemon).base_maxhp) / n).max(1)
}

/// The move an event runs for, as its handler sees `move.type`: the move
/// in flight (its type may differ from the dex's: Hidden Power), or a
/// typeless `???` move object that names no move (`getDamage`'s number
/// move, a future move's `moveData`).
pub fn event_move_type(b: &Battle, a: &EventArgs) -> Type {
    match a.effect {
        EffectId::Move(m) => match b.active_move() {
            Some(am) if am.id == m => am.type_,
            _ => b.dex.move_def(m).type_,
        },
        _ => Type::None,
    }
}

/// The move an event runs for, as its handler sees `move.category`: the
/// move in flight (Hidden Power and Beat Up change theirs), else the dex's.
pub fn event_move_category(b: &Battle, a: &EventArgs) -> Option<MoveCategory> {
    match a.effect {
        EffectId::Move(m) => Some(match b.active_move() {
            Some(am) if am.id == m => am.category,
            _ => b.dex.move_def(m).category,
        }),
        _ => None,
    }
}

/// The weather's name as a `-damage` line's `[from]` (`[from] Sandstorm`):
/// PS writes the condition's `fullname`, which for a weather is its name,
/// the name of the move that sets it.
pub fn weather_from(b: &Battle, weather: ConditionId) -> Arg {
    let key = b.dex.condition_def(weather).key.as_str();
    let m = b.dex.move_id(key).unwrap_or_else(|| panic!("no move named after the weather {key}"));
    Arg::new(ArgValue::Move(ps_protocol::MoveNum(m.0)))
}

/// The protocol name of a gen 3 weather condition (`-weather|RainDance`).
pub fn weather_word(b: &Battle, weather: ConditionId) -> Weather {
    let name = b.dex.condition_def(weather).name.as_str();
    Weather::parse(name).unwrap_or_else(|| panic!("no protocol weather named {name}"))
}

/// A `-weather` line's kwargs for a weather's `onFieldStart`: `[from]
/// ability: NAME` and `[of] SOURCE` when an ability set it.
pub fn weather_start_kw(b: &Battle, a: &EventArgs) -> Kw {
    match (a.effect, a.source) {
        (EffectId::Ability(_), Some(source)) => Kw::from_of(b.effect_fullname(a.effect), b.ident(source)),
        (EffectId::Ability(_), None) => Kw::from(b.effect_fullname(a.effect)),
        _ => Kw::NONE,
    }
}


#[cfg(test)]
mod tests;
#[cfg(test)]
mod volatile_tests;
