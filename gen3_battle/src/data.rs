//! Generated Gen 3 tables: the type chart, the species dex, and the move list.
//!
//! Everything in here is emitted by `build.rs` from the vendored data, merged
//! across the gen1, gen2 and gen3 layers. Nothing is hand-written, so a
//! correction belongs in the data or the generator, never here.

extern crate alloc;

use crate::types::{Type, TypeEffectiveness};

/// A species' six base stats. Special is two stats in Gen 3, which is the
/// whole reason this type exists rather than reusing Gen 1's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseStats {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

/// One dex entry.
#[derive(Clone, Copy, Debug)]
pub struct SpeciesEntry {
    /// Lowercase lookup id, e.g. `"blaziken"`.
    pub id: &'static str,
    pub name: &'static str,
    /// Primary type, then secondary or [`Type::None`].
    pub types: (Type, Type),
    pub base: BaseStats,
    /// Slice of [`LEARNSET`] holding this species' level-up moves.
    pub learn_start: u32,
    pub learn_len: u16,
}

impl SpeciesEntry {
    /// Level-up moves as `(move, level)`, sorted by move index.
    pub fn learnset(&self) -> &'static [(u16, u8)] {
        let start = self.learn_start as usize;
        &LEARNSET[start..start + self.learn_len as usize]
    }

    /// Everything this species knows by `level`, most recently learned first.
    pub fn moves_by_level(&self, level: u8) -> impl Iterator<Item = &'static MoveEntry> {
        let mut rows: alloc::vec::Vec<(u16, u8)> =
            self.learnset().iter().copied().filter(|(_, l)| *l <= level).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows.into_iter().map(|(i, _)| &MOVES[i as usize])
    }
}

/// One random-battle set: a species, the level a random battle drafts it at,
/// and one role's move pool.
///
/// A species appears once per role, so drawing a set is drawing a role.
#[derive(Clone, Copy, Debug)]
pub struct RandbatSet {
    pub species: &'static str,
    pub level: u8,
    pub move_start: u32,
    pub move_len: u16,
}

impl RandbatSet {
    /// The role's move pool, each with the type its set specifies when that
    /// differs from the move's own — which in Gen 3 is only Hidden Power.
    /// Showdown lists more moves than a set uses, and the drafter picks.
    pub fn moves(&self) -> impl Iterator<Item = (&'static MoveEntry, Option<Type>)> {
        let start = self.move_start as usize;
        RANDBAT_MOVES[start..start + self.move_len as usize].iter().map(|(i, ty)| {
            let over = if *ty as usize >= TYPE_COUNT { None } else { Some(TYPE_BY_INDEX[*ty as usize]) };
            (&MOVES[*i as usize], over)
        })
    }

    pub fn species_entry(&self) -> Option<&'static SpeciesEntry> {
        species_by_id(self.species)
    }
}

/// A status condition a move can inflict. Toxic is bad poison: its residual
/// grows each turn instead of holding at an eighth.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Burn,
    Paralysis,
    Poison,
    Toxic,
    Freeze,
    Sleep,
}

impl Status {
    /// The board's three-letter tag, matching what the display layer shows.
    pub fn abbr(self) -> &'static str {
        match self {
            Status::Burn => "brn",
            Status::Paralysis => "par",
            Status::Poison => "psn",
            Status::Toxic => "tox",
            Status::Freeze => "frz",
            Status::Sleep => "slp",
        }
    }
}

/// A stat a secondary effect can move, including the two battle-only stages
/// that live outside [`crate::stats::Stat`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boost {
    Atk,
    Def,
    Spe,
    SpAtk,
    SpDef,
    Acc,
    Eva,
}

impl Boost {
    /// The display name the battle log uses.
    pub fn label(self) -> &'static str {
        match self {
            Boost::Atk => "Attack",
            Boost::Def => "Defense",
            Boost::Spe => "Speed",
            Boost::SpAtk => "Sp. Atk",
            Boost::SpDef => "Sp. Def",
            Boost::Acc => "accuracy",
            Boost::Eva => "evasiveness",
        }
    }
}

/// What a secondary does when it procs: inflict a status, move the target's
/// stat stages (Mist Ball's Sp. Atk drop, Octazooka's accuracy cut), or make
/// the target flinch out of a move it has not used yet.
#[derive(Clone, Copy, Debug)]
pub enum SecondaryEffect {
    Status(Status),
    Boosts(&'static [(Boost, i8)]),
    Flinch,
    Confuse,
}

/// A move's secondary effect.
#[derive(Clone, Copy, Debug)]
pub struct Secondary {
    /// Percent chance.
    pub chance: u8,
    pub effect: SecondaryEffect,
}

/// A status move's whole effect: what a zero-power move does instead of
/// damage. Thunder Wave inflicts, Swords Dance raises the user, Growl drops
/// the target, Recover heals half.
#[derive(Clone, Copy, Debug)]
pub enum StatusAction {
    Inflict(Status),
    BoostSelf(&'static [(Boost, i8)]),
    BoostFoe(&'static [(Boost, i8)]),
    HealHalf,
    Confuse,
}

/// One move.
#[derive(Clone, Copy, Debug)]
pub struct MoveEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub move_type: Type,
    /// 0 for a status move.
    pub power: u16,
    /// 0 means it never misses.
    pub accuracy: u8,
    pub pp: u8,
    /// Move priority bracket; higher moves first regardless of Speed.
    pub priority: i8,
    pub secondary: Option<Secondary>,
    /// The user heals this fraction of the damage dealt (Giga Drain's 1/2).
    pub drain: Option<(u16, u16)>,
    /// The user takes this fraction of the damage dealt (Double-Edge's 1/3).
    pub recoil: Option<(u16, u16)>,
    /// Hits this many times: (2,2) fixed double, (2,5) the weighted
    /// 2-to-5 spread. None is the ordinary single hit.
    pub multihit: Option<(u16, u16)>,
    /// What a zero-power move does. None on damaging moves, and on status
    /// moves whose effect is not modelled yet (they no-op like Splash).
    pub status_action: Option<StatusAction>,
    /// A status move that type immunity blocks: Thunder Wave fails on
    /// Ground, Glare on Ghost. Almost every other status move ignores the
    /// chart in this era.
    pub respects_immunity: bool,
}

impl MoveEntry {
    /// Physical or special, which in Gen 3 follows the move's type.
    pub fn category(&self) -> crate::types::Category {
        if self.power == 0 {
            crate::types::Category::Status
        } else {
            crate::types::category_of(self.move_type)
        }
    }
}

/// Chart order, so a randbat slot's type index resolves back to a [`Type`].
static TYPE_BY_INDEX: [Type; 17] = [
    Type::Normal, Type::Fire, Type::Water, Type::Electric, Type::Grass, Type::Ice,
    Type::Fighting, Type::Poison, Type::Ground, Type::Flying, Type::Psychic, Type::Bug,
    Type::Rock, Type::Ghost, Type::Dragon, Type::Dark, Type::Steel,
];

include!(concat!(env!("OUT_DIR"), "/gen3_tables.rs"));

/// Find a species by lowercase id. O(log N): the table is emitted sorted.
pub fn species_by_id(id: &str) -> Option<&'static SpeciesEntry> {
    SPECIES.binary_search_by(|e| e.id.cmp(id)).ok().map(|i| &SPECIES[i])
}

/// Find a move by lowercase id. O(log N).
pub fn move_by_id(id: &str) -> Option<&'static MoveEntry> {
    MOVES.binary_search_by(|e| e.id.cmp(id)).ok().map(|i| &MOVES[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dex_covers_three_generations_of_layers() {
        // Gen 3's dex is everything up to Deoxys, so all three layers have to
        // have merged. Bulbasaur comes from the gen1 layer, Treecko from gen3.
        assert!(SPECIES.len() > 380, "only {} species merged", SPECIES.len());
        assert!(species_by_id("bulbasaur").is_some(), "the gen1 layer is missing");
        assert!(species_by_id("treecko").is_some(), "the gen3 layer is missing");
        assert!(species_by_id("notamon").is_none());
    }

    #[test]
    fn species_carry_split_stats_and_gen_three_typing() {
        let blaziken = species_by_id("blaziken").expect("blaziken");
        assert_eq!(blaziken.types, (Type::Fire, Type::Fighting));
        // Sp.Atk and Sp.Def are separate numbers, which is the point.
        assert_ne!(blaziken.base.spa, blaziken.base.spd);

        let bulbasaur = species_by_id("bulbasaur").expect("bulbasaur");
        assert_eq!(bulbasaur.types, (Type::Grass, Type::Poison));
        assert_eq!(bulbasaur.base.hp, 45);

        // A single-typed mon leaves the second slot empty rather than
        // repeating the first.
        let treecko = species_by_id("treecko").expect("treecko");
        assert_eq!(treecko.types.1, Type::None);
    }

    #[test]
    fn the_move_list_merges_and_categorises_by_type() {
        assert!(MOVES.len() > 340, "only {} moves merged", MOVES.len());
        let tackle = move_by_id("tackle").expect("tackle");
        assert_eq!(tackle.move_type, Type::Normal);
        assert_eq!(tackle.category(), crate::types::Category::Physical);

        // A Gen 3 addition, to prove the last layer landed.
        let aerial = move_by_id("aerialace").expect("aerialace");
        assert_eq!(aerial.move_type, Type::Flying);

        // Crunch is Dark, and Dark is special in Gen 3 however hard it bites.
        let crunch = move_by_id("crunch").expect("crunch");
        assert_eq!(crunch.category(), crate::types::Category::Special);
    }

    #[test]
    fn the_randbat_pool_is_showdowns_sets_not_a_learnset_dump() {
        assert!(RANDBAT.len() > 200, "only {} sets", RANDBAT.len());
        // Every set names a species the dex knows and carries real moves.
        for set in RANDBAT {
            assert!(set.species_entry().is_some(), "{} is not in the dex", set.species);
            assert!(set.move_len > 0);
            assert!(set.level > 0);
        }
        // Showdown's Gen 3 sets are high level, which is the giveaway that
        // these are its sets rather than anything derived here.
        assert!(RANDBAT.iter().all(|s| s.level >= 60));
    }

    #[test]
    fn moves_carry_priority_and_secondaries() {
        assert_eq!(move_by_id("quickattack").unwrap().priority, 1);
        assert_eq!(move_by_id("tackle").unwrap().priority, 0);
        let ember = move_by_id("ember").unwrap().secondary.unwrap();
        assert_eq!(ember.chance, 10);
        assert!(matches!(ember.effect, SecondaryEffect::Status(Status::Burn)));
        let bodyslam = move_by_id("bodyslam").unwrap().secondary.unwrap();
        assert_eq!(bodyslam.chance, 30);
        assert!(matches!(bodyslam.effect, SecondaryEffect::Status(Status::Paralysis)));
        assert!(move_by_id("tackle").unwrap().secondary.is_none());

        // Poison Fang badly poisons — tox, not plain psn. Found by the fuzzer:
        // the tick is a growing sixteenth, not a flat eighth.
        let fang = move_by_id("poisonfang").unwrap().secondary.unwrap();
        assert!(matches!(fang.effect, SecondaryEffect::Status(Status::Toxic)));

        // Boost secondaries carry the target's stage change. Crunch drops
        // Sp. Def in this era, not Defense.
        let crunch = move_by_id("crunch").unwrap().secondary.unwrap();
        assert_eq!(crunch.chance, 20);
        assert!(
            matches!(crunch.effect, SecondaryEffect::Boosts(&[(Boost::SpDef, -1)])),
            "crunch: {:?}",
            crunch.effect
        );
        let octazooka = move_by_id("octazooka").unwrap().secondary.unwrap();
        assert!(matches!(octazooka.effect, SecondaryEffect::Boosts(&[(Boost::Acc, -1)])));
    }

    #[test]
    fn tables_are_sorted_so_the_lookups_can_binary_search() {
        assert!(SPECIES.windows(2).all(|w| w[0].id < w[1].id));
        assert!(MOVES.windows(2).all(|w| w[0].id < w[1].id));
    }

    #[test]
    fn no_move_claims_a_type_this_generation_lacks() {
        // `build.rs` panics on an unknown type name, so reaching here at all
        // means every entry mapped. Curse is the era's one "???"-typed move
        // and maps to Type::None; nothing else may.
        assert!(MOVES
            .iter()
            .all(|m| m.move_type != Type::None || m.id == "curse"));
    }
}
