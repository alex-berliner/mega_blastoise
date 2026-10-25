//! The `ModifySpecies` rules of the variant formats: gen 1's own Scalemons
//! Mod and 350 Cup Mod (`data/mods/gen1/rulesets.ts`: the Special is one
//! stat there, so both leave `spd` out and copy `spa` into it), and Bad 'n
//! Boosted Mod, which PS defines once for every gen (gen 3 registers it
//! too). The base Scalemons and 350 Cup Mods are gen 3's
//! (`crate::mods::gen3::rules::species_mods`). Each returns a deep clone of
//! the species with new base stats and `bst` (the [`Relay::Species`]
//! relay); the next handler, and `setSpecies`, read that clone. PS's
//! `clampIntRange` floors first; the base stats are non-negative, so
//! integer division floors the same way.
//!
//! PS: data/mods/gen1/rulesets.ts scalemonsmod, 350cupmod;
//! data/rulesets.ts badnboostedmod

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

pub(crate) const HP: usize = 0;
const SPA: usize = 3;
const SPD: usize = 4;

/// `this.clampIntRange(Math.floor(num), 1, 255)` of a non-negative number.
pub(crate) fn clamp_stat(num: u32) -> u8 {
    num.clamp(1, 255) as u8
}

/// The species object of a `ModifySpecies` relay: base stats and `bst`.
pub(crate) fn species(a: &EventArgs) -> ([u8; 6], u16) {
    match a.relay {
        Some(Relay::Species { base_stats, bst }) => (base_stats, bst),
        other => panic!("ModifySpecies relay {other:?}"),
    }
}

/// Gen 1's Scalemons Mod: toward 500, `spd` skipped (and left out of the
/// new `bst`), then `spd = spa`.
// PS: data/mods/gen1/rulesets.ts scalemonsmod.onModifySpecies
fn scalemons_gen1(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (mut stats, bst) = species(a);
    let hp = u32::from(stats[HP]);
    let pst = u32::from(bst) - hp;
    let scale = 500 - hp;
    let mut new_bst = hp;
    for (i, stat) in stats.iter_mut().enumerate() {
        if i == HP || i == SPD {
            continue;
        }
        *stat = clamp_stat(u32::from(*stat) * scale / pst);
        new_bst += u32::from(*stat);
    }
    stats[SPD] = stats[SPA];
    Some(Relay::Species { base_stats: stats, bst: new_bst as u16 })
}

/// Gen 1's 350 Cup Mod: `spd` skipped, then `spd = spa`.
// PS: data/mods/gen1/rulesets.ts 350cupmod.onModifySpecies
fn three_fifty_cup_gen1(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (mut stats, mut bst) = species(a);
    if bst <= 350 {
        bst = 0;
        for (i, stat) in stats.iter_mut().enumerate() {
            if i == SPD {
                continue;
            }
            *stat = clamp_stat(u32::from(*stat) * 2);
            bst += u16::from(*stat);
        }
        stats[SPD] = stats[SPA];
    }
    Some(Relay::Species { base_stats: stats, bst })
}

/// Bad 'n Boosted Mod, `onModifySpecies(species)`: every base stat of 70
/// or lower doubled.
// PS: data/rulesets.ts badnboostedmod.onModifySpecies
fn bad_n_boosted(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (mut stats, _) = species(a);
    let mut bst = 0;
    for stat in stats.iter_mut() {
        if *stat <= 70 {
            *stat = clamp_stat(u32::from(*stat) * 2);
        }
        bst += u16::from(*stat);
    }
    Some(Relay::Species { base_stats: stats, bst })
}

pub static SCALEMONS_GEN1: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpecies, scalemons_gen1)]);
pub static THREE_FIFTY_CUP_GEN1: EffectDef =
    EffectDef::new(&[Handler::on(EventId::ModifySpecies, three_fifty_cup_gen1)]);
/// `onModifySpeciesPriority: 2`.
pub static BAD_N_BOOSTED: EffectDef =
    EffectDef::new(&[Handler::on(EventId::ModifySpecies, bad_n_boosted).priority(2)]);
