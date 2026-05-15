//! Type and Stat enums mirroring battler-data's surface.
//!
//! Gen 1 mechanics only use a subset (no SpDef split, no Steel/Dark types),
//! but we expose the full Gen 2+ enum variants for API compatibility with
//! existing callers. Unused variants simply never appear in battle state.

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Normal = 0,
    Fire,
    Water,
    Electric,
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    // Gen 2+ — present for API compat, not used by Gen 1 mechanics.
    Dark,
    Steel,
    Fairy,
    None,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    Hp = 0,
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
}
