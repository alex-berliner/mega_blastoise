//! The relay variable of PS's event system (design section 5.3).
//!
//! PS threads one loosely typed value through `runEvent` and `singleEvent`
//! (`relayVar`): a number, `true`/`false`, `null`, the empty string, an id,
//! a status object, a types array. Handlers tell those apart (`=== false`
//! vs `=== null` vs `=== 0` vs `''`), so [`Relay`] keeps every kind that
//! reaches gens 1 and 3 as its own variant. A handler that returns PS's
//! `undefined` ("change nothing") returns `None` instead of a `Relay`.
//!
//! A JS number is [`Relay::Int`], except where it may carry a fraction:
//! gen 3's accuracy (`100 * 4 / 3`) and the STAB (`1.5`) travel as
//! [`Relay::Float`], whole or not, since PS's final modifier only applies
//! to a whole number and the fraction has to survive.
//!
//! PS's named constants are JS values, and code compares against them with
//! `===`, so they are the plain values here too:
//! - `FAIL` is `false` ([`Relay::FAIL`] is `Bool(false)`);
//! - `SILENT_FAIL` is `null` ([`Relay::Null`]);
//! - `NOT_FAIL` is `''` ([`Relay::NotFail`]);
//! - `HIT_SUBSTITUTE` is `0` ([`Relay::HIT_SUBSTITUTE`] is `Int(0)`): PS
//!   cannot tell a substitute hit from zero damage, and neither do we.
//!
//! PS: sim/battle.ts (`runEvent`'s doc comment, `NOT_FAIL`, `HIT_SUBSTITUTE`,
//! `FAIL`, `SILENT_FAIL`).

use ps_dex::{MoveId, Type};

use crate::effect::EffectId;

/// A types array (`pokemon.getTypes()`, the `Type` event's relay): one or
/// two types, the second [`Type::None`] when absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeList(pub [Type; 2]);

impl TypeList {
    /// The types in order, without the empty second slot.
    pub fn as_slice(&self) -> &[Type] {
        if self.0[1] == Type::None {
            &self.0[..1]
        } else {
            &self.0[..]
        }
    }
}

/// One relay value. See the module doc for how PS's constants map here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relay {
    /// `true` / `false`. `false` is PS's `FAIL`.
    Bool(bool),
    /// A number. PS's numbers here are integers (damage, base power, a
    /// stat, a crit ratio, a priority); `0` is also `HIT_SUBSTITUTE`.
    Int(i32),
    /// `null`, PS's `SILENT_FAIL`.
    Null,
    /// `''`, PS's `NOT_FAIL`. Falsy, like every empty string.
    NotFail,
    /// A move: a move id string (`onLockMove: 'recharge'`) or the move
    /// object a move event passes along.
    Move(MoveId),
    /// An effect object: the status of `SetStatus`, the condition of
    /// `TryAddVolatile`.
    Effect(EffectId),
    /// A types array (`Type` event).
    Types(TypeList),
    /// The id string `'recharge'`, which names no move: mustrecharge's
    /// `onLockMove` (the request's Recharge button, PS's
    /// `getActiveMove('recharge')`).
    Recharge,
    /// A boosts table (the relay of `TryBoost`, `AfterEachBoost` and
    /// `AfterBoost`): atk, def, spa, spd, spe, accuracy, evasion, in the
    /// order of [`ps_protocol::Stat::ALL`]. PS passes `TryBoost` a copy
    /// (`{...boost}`) that a handler may drop entries from (Mist drops the
    /// drops); a dropped entry is 0 here.
    Boosts([i8; 7]),
    /// A JS number that may carry a fraction (gen 3's accuracy, the STAB).
    /// The bits of the `f64`, so `Relay` stays `Eq`.
    Float(u64),
    /// The relay of `ModifySecondaries`, PS's `secondaries.slice()`: which
    /// of the move's dex secondaries are left, bit `i` for
    /// `secondaries[i]` (Shield Dust filters the list; a handler returns
    /// a subset of the same objects).
    Secondaries(u8),
    /// The species object of `ModifySpecies`, as far as a handler changes
    /// it: the base stats (hp, atk, def, spa, spd, spe) and `bst`. A rule
    /// (Scalemons Mod and the like) returns a deep clone with new numbers;
    /// the next handler reads that one.
    Species { base_stats: [u8; 6], bst: u16 },
}

impl Relay {
    /// PS's `true`, also the relay `runEvent` uses when given none.
    pub const TRUE: Relay = Relay::Bool(true);
    /// PS's `FAIL` (`false`).
    pub const FAIL: Relay = Relay::Bool(false);
    /// PS's `SILENT_FAIL` (`null`).
    pub const SILENT_FAIL: Relay = Relay::Null;
    /// PS's `NOT_FAIL` (`''`).
    pub const NOT_FAIL: Relay = Relay::NotFail;
    /// PS's `HIT_SUBSTITUTE` (`0`).
    pub const HIT_SUBSTITUTE: Relay = Relay::Int(0);

    /// JS truthiness: `false`, `0`, `null` and `''` are falsy; ids, objects
    /// and arrays are truthy.
    pub fn truthy(self) -> bool {
        match self {
            Relay::Bool(b) => b,
            Relay::Int(n) => n != 0,
            Relay::Null | Relay::NotFail => false,
            Relay::Move(_) | Relay::Effect(_) | Relay::Types(_) | Relay::Recharge | Relay::Boosts(_) => true,
            Relay::Float(bits) => {
                let v = f64::from_bits(bits);
                v != 0.0 && !v.is_nan()
            }
            Relay::Secondaries(_) | Relay::Species { .. } => true,
        }
    }

    /// The number, if this is one.
    pub fn as_int(self) -> Option<i32> {
        match self {
            Relay::Int(n) => Some(n),
            _ => None,
        }
    }

    /// A JS number relay (the accuracy, the STAB).
    pub fn float(v: f64) -> Relay {
        Relay::Float(v.to_bits())
    }
}

impl From<bool> for Relay {
    fn from(b: bool) -> Relay {
        Relay::Bool(b)
    }
}

impl From<i32> for Relay {
    fn from(n: i32) -> Relay {
        Relay::Int(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthiness_matches_js() {
        assert!(Relay::TRUE.truthy());
        assert!(!Relay::FAIL.truthy());
        assert!(!Relay::SILENT_FAIL.truthy());
        assert!(!Relay::NOT_FAIL.truthy());
        assert!(!Relay::HIT_SUBSTITUTE.truthy());
        assert!(Relay::Int(-1).truthy());
        assert!(Relay::Move(MoveId(0)).truthy());
        assert!(Relay::Types(TypeList([Type::Fire, Type::None])).truthy());
        // `false !== null`, `0 !== ''`: the kinds stay apart.
        assert_ne!(Relay::FAIL, Relay::SILENT_FAIL);
        assert_ne!(Relay::HIT_SUBSTITUTE, Relay::NOT_FAIL);
        // ...but HIT_SUBSTITUTE is the number 0, as in PS.
        assert_eq!(Relay::HIT_SUBSTITUTE, Relay::Int(0));
    }

    #[test]
    fn type_list_slice() {
        assert_eq!(TypeList([Type::Fire, Type::None]).as_slice(), &[Type::Fire]);
        assert_eq!(
            TypeList([Type::Fire, Type::Flying]).as_slice(),
            &[Type::Fire, Type::Flying]
        );
    }
}
