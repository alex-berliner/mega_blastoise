//! The base dex's `ModifySpecies` rules that gen 1 overrides: Scalemons
//! Mod (every base stat but HP scaled toward a BST of 600) and 350 Cup Mod
//! (a species of BST 350 or lower has its base stats doubled). Gen 3's mod
//! overrides neither. Bad 'n Boosted Mod, the same in every gen, is gen 1's
//! port (`crate::mods::gen1::rules::species_mods`), as are the helpers.
//!
//! PS: data/rulesets.ts scalemonsmod, 350cupmod

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen1::rules::species_mods::{clamp_stat, species, HP};
use crate::relay::Relay;
use crate::state::Battle;

/// Scalemons Mod, `onModifySpecies(species)`: every base stat but HP times
/// `(600 - hp) / (bst - hp)`.
// PS: data/rulesets.ts scalemonsmod.onModifySpecies
fn scalemons(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (mut stats, bst) = species(a);
    let hp = u32::from(stats[HP]);
    let bst_without_hp = u32::from(bst) - hp;
    let scale = 600 - hp;
    let mut new_bst = hp;
    for (i, stat) in stats.iter_mut().enumerate() {
        if i == HP {
            continue;
        }
        *stat = clamp_stat(u32::from(*stat) * scale / bst_without_hp);
        new_bst += u32::from(*stat);
    }
    Some(Relay::Species { base_stats: stats, bst: new_bst as u16 })
}

/// 350 Cup Mod, `onModifySpecies(species)`: a `bst` of 350 or lower has
/// every base stat doubled.
// PS: data/rulesets.ts 350cupmod.onModifySpecies
fn three_fifty_cup(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (mut stats, mut bst) = species(a);
    if bst <= 350 {
        bst = 0;
        for stat in stats.iter_mut() {
            *stat = clamp_stat(u32::from(*stat) * 2);
            bst += u16::from(*stat);
        }
    }
    Some(Relay::Species { base_stats: stats, bst })
}

/// `onModifySpeciesPriority: 1`.
pub static SCALEMONS: EffectDef =
    EffectDef::new(&[Handler::on(EventId::ModifySpecies, scalemons).priority(1)]);
/// `onModifySpeciesPriority: 2`.
pub static THREE_FIFTY_CUP: EffectDef =
    EffectDef::new(&[Handler::on(EventId::ModifySpecies, three_fifty_cup).priority(2)]);
