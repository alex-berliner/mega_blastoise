//! Type and Stat enums mirroring battler-data's surface.
//!
//! Gen 1 mechanics only use a subset (no SpDef split, no Steel/Dark types),
//! but we expose the full Gen 2+ enum variants for API compatibility with
//! existing callers. Unused variants simply never appear in battle state.

/// Re-exported so `gen1_battle::Type` keeps working; the enum itself is
/// shared with every other generation's engine.
pub use battle_types::Type;

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
