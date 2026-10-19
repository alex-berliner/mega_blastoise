//! Random team drafting for Gen 3.
//!
//! Sets come from Showdown's Gen 3 random battle data, the same source the
//! Gen 1 side already draws from. That matters: a species' learnset is
//! everything it *can* know, which drafts nonsense like a sweeper carrying
//! three status moves. A random battle set is a curated role, and drawing a
//! set is drawing a role.
//!
//! Not yet drafted, because the engine has nowhere to put them: abilities,
//! held items, and per-stat EV and IV spreads. Every drafted mon gets the
//! same even investment instead.

extern crate alloc;

use alloc::vec::Vec;

use crate::battle::{Mon, MoveSlot, Rng};
use crate::data::{RandbatSet, SpeciesEntry, RANDBAT};
use crate::stats::{Invest, Nature};
use crate::types::Category;

/// How many moves a drafted mon carries.
const MOVE_SLOTS: usize = 4;

/// Investment every drafted mon gets: perfect IVs, no EVs. Even spreads read
/// as fairer across a table than random ones, and EVs need a trainer to
/// allocate them.
const DRAFT_INVEST: Invest = Invest { iv: 31, ev: 0 };

/// Draft the mon a random-battle set describes.
///
/// The set lists more moves than fit, so damaging ones are taken first and the
/// rest fill in, which keeps a role's identity without needing Showdown's own
/// weighting.
pub fn draft_set(rng: &mut Rng, set: &'static RandbatSet) -> Option<Mon> {
    let species = set.species_entry()?;
    let pool: Vec<(&'static crate::data::MoveEntry, Option<crate::types::Type>)> =
        set.moves().collect();
    if pool.is_empty() {
        return None;
    }

    let mut picked: Vec<MoveSlot> = Vec::with_capacity(MOVE_SLOTS);
    let take = |want_damaging: bool, picked: &mut Vec<MoveSlot>| {
        for (entry, ty) in &pool {
            if picked.len() == MOVE_SLOTS {
                return;
            }
            let damaging = entry.category() != Category::Status;
            if damaging != want_damaging {
                continue;
            }
            if picked.iter().any(|s| s.entry.id == entry.id) {
                continue;
            }
            picked.push(MoveSlot::typed(entry, *ty));
        }
    };
    take(true, &mut picked);
    take(false, &mut picked);

    let nature = Nature::from_index(rng.below(25) as u8);
    Mon::with_moves(species.id, set.level, nature, DRAFT_INVEST, picked)
}

/// Draft one mon of `species` at `level` from its level-up learnset.
///
/// The fallback for a species with no random-battle set, and for callers that
/// want a specific mon rather than a random one.
pub fn draft_mon(rng: &mut Rng, species: &'static SpeciesEntry, level: u8) -> Option<Mon> {
    let known: Vec<&'static crate::data::MoveEntry> = species.moves_by_level(level).collect();
    if known.is_empty() {
        return None;
    }
    let mut picked: Vec<&str> = Vec::with_capacity(MOVE_SLOTS);
    for want_damaging in [true, false] {
        for m in known.iter() {
            if picked.len() == MOVE_SLOTS {
                break;
            }
            if (m.category() != Category::Status) != want_damaging {
                continue;
            }
            if !picked.contains(&m.id) {
                picked.push(m.id);
            }
        }
    }
    let nature = Nature::from_index(rng.below(25) as u8);
    Mon::new(species.id, level, nature, DRAFT_INVEST, &picked)
}

/// Draft a team of `size` distinct species from the random-battle pool.
///
/// Deterministic for a given seed, so both seats of a battle can be drawn from
/// one seed and replayed.
pub fn draft_team(rng: &mut Rng, size: usize) -> Vec<Mon> {
    let mut team: Vec<Mon> = Vec::with_capacity(size);
    // Bounded rather than looping until success: a set that cannot be built
    // must not be able to hang the draft.
    let mut attempts = 0;
    while team.len() < size && attempts < size * 64 {
        attempts += 1;
        let set = &RANDBAT[rng.below(RANDBAT.len() as u32) as usize];
        if team.iter().any(|m| m.species.id == set.species) {
            continue;
        }
        if let Some(mon) = draft_set(rng, set) {
            team.push(mon);
        }
    }
    team
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::species_by_id;

    #[test]
    fn a_drafted_team_is_the_size_asked_for() {
        let mut rng = Rng::new(1);
        let team = draft_team(&mut rng, 3);
        assert_eq!(team.len(), 3);
        // Levels come from the sets, which are Showdown's, not ours.
        assert!(team.iter().all(|m| m.level >= 60));
    }

    #[test]
    fn every_drafted_mon_can_actually_fight() {
        let mut rng = Rng::new(9);
        for mon in draft_team(&mut rng, 6) {
            assert!(
                !mon.moves.is_empty(),
                "{} was drafted with no moves",
                mon.species.name
            );
            assert!(mon.moves.len() <= 4);
            assert!(mon.hp > 0);
        }
    }

    #[test]
    fn a_team_has_no_duplicates() {
        let mut rng = Rng::new(4);
        let team = draft_team(&mut rng, 6);
        for (i, a) in team.iter().enumerate() {
            for b in team.iter().skip(i + 1) {
                assert_ne!(a.species.id, b.species.id);
            }
        }
    }

    #[test]
    fn the_same_seed_drafts_the_same_team() {
        let names = |seed: u64| {
            let mut rng = Rng::new(seed);
            draft_team(&mut rng, 4)
                .iter()
                .map(|m| m.species.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(names(77), names(77));
        assert_ne!(names(77), names(78), "different seeds should differ");
    }

    #[test]
    fn a_drafted_set_uses_showdowns_moves_for_that_role() {
        let mut rng = Rng::new(11);
        let set = RANDBAT
            .iter()
            .find(|s| s.species == "absol")
            .expect("absol has sets");
        let mon = draft_set(&mut rng, set).expect("absol drafts");
        let allowed: Vec<&str> = set.moves().map(|(m, _)| m.id).collect();
        for slot in &mon.moves {
            assert!(
                allowed.contains(&slot.entry.id),
                "{} is not in the role",
                slot.entry.name
            );
        }
        assert_eq!(mon.level, set.level);
    }

    #[test]
    fn hidden_power_keeps_the_type_its_set_gives_it() {
        // The move table has one Hidden Power, typed Normal. A set that asks
        // for a typed one has to survive into the slot, or a third of the
        // pool's coverage moves quietly become Normal.
        let typed = RANDBAT
            .iter()
            .flat_map(|s| s.moves())
            .find(|(m, ty)| m.id == "hiddenpower" && ty.is_some());
        let (entry, ty) = typed.expect("some set runs a typed Hidden Power");
        let slot = MoveSlot::typed(entry, ty);
        assert_ne!(slot.move_type(), entry.move_type);
        assert_eq!(slot.move_type(), ty.unwrap());
    }

    #[test]
    fn moves_come_from_the_species_own_learnset() {
        let mut rng = Rng::new(5);
        let blaziken = species_by_id("blaziken").expect("blaziken");
        let mon = draft_mon(&mut rng, blaziken, 50).expect("blaziken can fight at 50");
        let learnable: Vec<&str> = blaziken.moves_by_level(50).map(|m| m.id).collect();
        for slot in &mon.moves {
            assert!(
                learnable.contains(&slot.entry.id),
                "{} is not in blaziken's level-50 learnset",
                slot.entry.name,
            );
        }
    }

    #[test]
    fn a_low_level_mon_only_knows_early_moves() {
        let mut rng = Rng::new(6);
        let blaziken = species_by_id("blaziken").expect("blaziken");
        let low = draft_mon(&mut rng, blaziken, 5).expect("even at 5 it knows something");
        let early: Vec<&str> = blaziken.moves_by_level(5).map(|m| m.id).collect();
        assert!(low.moves.iter().all(|s| early.contains(&s.entry.id)));
    }
}
