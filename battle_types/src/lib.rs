//! The one `Type` enum, shared by every generation's engine.
//!
//! Each engine used to declare its own, and to Rust those are unrelated types
//! however identical their variants — so a Gen 3 species' type could not be
//! handed to anything typed against Gen 1's, and shared UI code that wanted
//! "a type" had to pick a generation or go without. The party card went
//! without, and drew no type chips for Gen 3 at all.
//!
//! A generation that lacks a type simply never produces it. Both engines
//! index their chart by `Type as usize` behind a `< TYPE_COUNT` guard, so a
//! variant past the end of a given generation's chart reads as neutral, which
//! is exactly right: Gen 3 has no Fairy, and asking about one gets 1x back
//! rather than a panic or a wrong row.
//!
//! The order is load-bearing. Both engines' `build.rs` emit their chart rows
//! in this order, so a new variant belongs at the END of the real types and
//! before `None`, never in the middle.

#![no_std]

/// A Pokemon type. `None` is the empty second slot of a single-typed mon.
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
    Dark,
    Steel,
    Fairy,
    None,
}

impl Type {
    /// The type's printed name, as the dex spells it.
    pub fn name(self) -> &'static str {
        match self {
            Type::Normal => "Normal",
            Type::Fire => "Fire",
            Type::Water => "Water",
            Type::Electric => "Electric",
            Type::Grass => "Grass",
            Type::Ice => "Ice",
            Type::Fighting => "Fighting",
            Type::Poison => "Poison",
            Type::Ground => "Ground",
            Type::Flying => "Flying",
            Type::Psychic => "Psychic",
            Type::Bug => "Bug",
            Type::Rock => "Rock",
            Type::Ghost => "Ghost",
            Type::Dragon => "Dragon",
            Type::Dark => "Dark",
            Type::Steel => "Steel",
            Type::Fairy => "Fairy",
            Type::None => "",
        }
    }

    /// Three-letter label for the badges and chips both renderers draw.
    pub fn abbr(self) -> &'static str {
        match self {
            Type::Normal => "NRM",
            Type::Fire => "FIR",
            Type::Water => "WAT",
            Type::Electric => "ELC",
            Type::Grass => "GRS",
            Type::Ice => "ICE",
            Type::Fighting => "FGT",
            Type::Poison => "PSN",
            Type::Ground => "GND",
            Type::Flying => "FLY",
            Type::Psychic => "PSY",
            Type::Bug => "BUG",
            Type::Rock => "RCK",
            Type::Ghost => "GHO",
            Type::Dragon => "DRG",
            Type::Dark => "DRK",
            Type::Steel => "STL",
            Type::Fairy => "FAI",
            Type::None => "",
        }
    }
}
