//! Stats: the set-to-stat formula, natures, Hidden Power, PP, and the
//! `getStat` / `getActionSpeed` the queue sorts by.
//!
//! P1-6 needs these to build Pokemon (`setSpecies`) and to order actions;
//! the move pipelines (P1-8, P1-9) own the rest of this file (gen 1's
//! `modifyStat` / `boostBy`, the full boost and modifier chain).
//!
//! PS: sim/battle.ts `spreadModify`, `statModify`, `calculatePP`;
//! sim/dex.ts `getHiddenPower`; sim/pokemon.ts `getStat`,
//! `getActionSpeed`, `updateSpeed`; data/mods/gen1/scripts.ts `getStat`;
//! data/mods/gen3/scripts.ts `getActionSpeed`; data/natures.ts.

use ps_dex::{MoveDef, Type};

use crate::effect::{EffectId, EventId};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, Pokemon};

/// A stat other than HP, PS's `StatIDExceptHP`, in `storedStats` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Atk = 0,
    Def,
    Spa,
    Spd,
    Spe,
}

/// PS's natures in `dex.natures.all()` order: name, `plus`, `minus`
/// (indices into hp, atk, def, spa, spd, spe; neutral natures have none).
pub const NATURES: [(&str, Option<u8>, Option<u8>); 25] = [
    ("Adamant", Some(1), Some(3)),
    ("Bashful", None, None),
    ("Bold", Some(2), Some(1)),
    ("Brave", Some(1), Some(5)),
    ("Calm", Some(4), Some(1)),
    ("Careful", Some(4), Some(3)),
    ("Docile", None, None),
    ("Gentle", Some(4), Some(2)),
    ("Hardy", None, None),
    ("Hasty", Some(5), Some(2)),
    ("Impish", Some(2), Some(3)),
    ("Jolly", Some(5), Some(3)),
    ("Lax", Some(2), Some(4)),
    ("Lonely", Some(1), Some(2)),
    ("Mild", Some(3), Some(2)),
    ("Modest", Some(3), Some(1)),
    ("Naive", Some(5), Some(4)),
    ("Naughty", Some(1), Some(4)),
    ("Quiet", Some(3), Some(5)),
    ("Quirky", None, None),
    ("Rash", Some(3), Some(4)),
    ("Relaxed", Some(2), Some(5)),
    ("Sassy", Some(4), Some(5)),
    ("Serious", None, None),
    ("Timid", Some(5), Some(1)),
];

/// The nature named `id` (PS `toID` form, `adamant`).
pub fn nature_from_id(id: &str) -> Option<u8> {
    NATURES.iter().position(|(n, _, _)| n.eq_ignore_ascii_case(id)).map(|i| i as u8)
}

/// Hidden Power's types in PS's table order.
const HP_TYPES: [Type; 16] = [
    Type::Fighting,
    Type::Flying,
    Type::Poison,
    Type::Ground,
    Type::Rock,
    Type::Bug,
    Type::Ghost,
    Type::Steel,
    Type::Fire,
    Type::Water,
    Type::Grass,
    Type::Electric,
    Type::Psychic,
    Type::Ice,
    Type::Dragon,
    Type::Dark,
];

/// PS `dex.getHiddenPower(ivs)`: type and base power. `ivs` in hp, atk,
/// def, spa, spd, spe order. `dex.trunc` is the cartridge truncation, a
/// plain floor for these non-negative values.
// PS: sim/dex.ts getHiddenPower
pub fn hidden_power(gen: u8, ivs: [u8; 6]) -> (Type, u8) {
    let [hp, atk, def, spa, spd, spe] = ivs.map(u32::from);
    if gen <= 2 {
        let (atk_dv, def_dv, spe_dv, spc_dv) = (atk / 2, def / 2, spe / 2, spa / 2);
        let ty = HP_TYPES[(4 * (atk_dv % 4) + (def_dv % 4)) as usize];
        let power = (5 * ((spc_dv >> 3) + 2 * (spe_dv >> 3) + 4 * (def_dv >> 3) + 8 * (atk_dv >> 3)) + spc_dv % 4) / 2 + 31;
        return (ty, power as u8);
    }
    // `for (const s in stats)` with `stats = {hp, atk, def, spe, spa, spd}`.
    let mut type_x = 0;
    let mut power_x = 0;
    for (i, iv) in [hp, atk, def, spe, spa, spd].into_iter().enumerate() {
        type_x += (1 << i) * (iv % 2);
        power_x += (1 << i) * ((iv / 2) % 2);
    }
    let power = if gen < 6 { power_x * 40 / 63 + 30 } else { 60 };
    (HP_TYPES[(type_x * 15 / 63) as usize], power as u8)
}

/// PS `calculatePP(move, ppUps)`.
// PS: sim/battle.ts calculatePP
pub fn calculate_pp(gen: u8, mv: &MoveDef, pp_ups: u32) -> u8 {
    if mv.bools.has("noPPBoosts") {
        return mv.pp;
    }
    let base = u32::from(mv.pp);
    // `pp * (5 + ppUps) / 5`: PS keeps the fraction, but every gen 1-3 PP
    // is a multiple of 5, so it is an integer.
    let mut pp = base * (5 + pp_ups) / 5;
    if gen <= 2 && base == 40 {
        pp -= pp_ups;
    }
    pp as u8
}

/// Two `JSON` numbers that PS's `modify(stat, 1.5)` and friends read: the
/// boost table `[1, 1.5, 2, 2.5, 3, 3.5, 4]` as halves.
const BOOST_HALVES: [i64; 7] = [2, 3, 4, 5, 6, 7, 8];

impl Battle {
    /// PS `statModify(baseStats, set, statName)`, `stat` 0 for HP to 5 for
    /// Speed, over the set fields stored on the Pokemon.
    // PS: sim/battle.ts statModify
    pub fn stat_modify(&self, base_stats: [u8; 6], mon: &Pokemon, stat: usize) -> u16 {
        let tr = |n: i64| self.trunc.int(n, 0);
        let base = i64::from(base_stats[stat]);
        let iv = i64::from(mon.ivs[stat]);
        let ev = i64::from(mon.evs[stat]);
        let level = i64::from(mon.level);
        if stat == 0 {
            return tr(tr(2 * base + iv + tr(ev / 4) + 100) * level / 100 + 10) as u16;
        }
        let mut s = tr(tr(2 * base + iv + tr(ev / 4)) * level / 100 + 5);
        if let Some(n) = mon.nature {
            let (_, plus, minus) = NATURES[n as usize];
            // No overflowstatmod in our formats.
            if plus == Some(stat as u8) {
                s = tr(self.trunc.int(s * 110, 16) / 100);
            } else if minus == Some(stat as u8) {
                s = tr(self.trunc.int(s * 90, 16) / 100);
            }
        }
        s as u16
    }

    /// PS `spreadModify(baseStats, set)`: hp, atk, def, spa, spd, spe.
    // PS: sim/battle.ts spreadModify
    pub fn spread_modify(&self, base_stats: [u8; 6], mon: &Pokemon) -> [u16; 6] {
        core::array::from_fn(|i| self.stat_modify(base_stats, mon, i))
    }

    /// PS `pokemon.getStat(statName, unboosted, unmodified)`, for the gen.
    /// Gen 1 reads its `modifiedStats` (its override takes only
    /// `(statName, unmodified)`: the second argument PS passes lands there).
    // PS: sim/pokemon.ts getStat, data/mods/gen1/scripts.ts getStat
    pub fn get_stat(&mut self, m: MonRef, stat: Stat, unboosted: bool, unmodified: bool) -> i32 {
        let s = stat as usize;
        if self.gen == 1 {
            let mon = self.mon(m);
            return i32::from(if unboosted { mon.base_stored_stats[s] } else { mon.modified_stats[s] });
        }
        let mut value = i64::from(self.mon(m).stored_stats[s]);
        // No Wonder Room in gens 1-3.
        if !unboosted {
            let mut boost = self.mon(m).boosts[s];
            if !unmodified {
                // `runEvent('ModifyBoost', this, null, null, {...boosts})`:
                // no gen 1-3 effect answers it (Unaware and Simple are gen
                // 4); the boosts table relay arrives with the first one.
                let r = self.run_event(EventId::ModifyBoost, Holder::Mon(m), None, EffectId::None, Some(Relay::TRUE));
                debug_assert_eq!(r, Relay::TRUE, "ModifyBoost handler: boosts relay not ported");
                boost = self.mon(m).boosts[s];
            }
            let b = boost.clamp(-6, 6);
            value = if b >= 0 {
                value * BOOST_HALVES[b as usize] / 2
            } else {
                value * 2 / BOOST_HALVES[(-b) as usize]
            };
        }
        if !unmodified {
            let event = match stat {
                Stat::Atk => EventId::ModifyAtk,
                Stat::Def => EventId::ModifyDef,
                Stat::Spa => EventId::ModifySpA,
                Stat::Spd => EventId::ModifySpD,
                Stat::Spe => EventId::ModifySpe,
            };
            match self.run_event(event, Holder::Mon(m), None, EffectId::None, Some(Relay::Int(value as i32))) {
                Relay::Int(n) => value = i64::from(n),
                other => panic!("Modify{stat:?} returned {other:?}"),
            }
        }
        // The 10000 Speed cap is off: both custom games set
        // `format.battle.trunc`.
        value as i32
    }

    /// PS `pokemon.getActionSpeed()`, gen 3's version (gen 1 inherits it).
    /// No Trick Room in gens 1-3.
    // PS: data/mods/gen3/scripts.ts getActionSpeed
    pub fn get_mon_action_speed(&mut self, m: MonRef) -> i32 {
        let mut speed = self.get_stat(m, Stat::Spe, false, false);
        if self.quick_claw_roll {
            if let Some(qc) = self.dex.item_id("quickclaw") {
                if self.has_item(m, qc) {
                    speed = 65535;
                }
            }
        }
        speed
    }

    /// PS `pokemon.updateSpeed()`.
    // PS: sim/pokemon.ts updateSpeed
    pub fn update_mon_speed(&mut self, m: MonRef) {
        let s = self.get_mon_action_speed(m);
        self.mon_mut(m).speed = s;
    }

    /// PS `battle.updateSpeed()`: every active Pokemon, p1 first.
    // PS: sim/battle.ts updateSpeed
    pub fn update_speed(&mut self) {
        let (actives, n) = self.all_active(false);
        for &m in &actives[..n] {
            self.update_mon_speed(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_power_matches_ps() {
        // All 31: Dark 70 (gen 3).
        assert_eq!(hidden_power(3, [31; 6]), (Type::Dark, 70));
        // Gen 1-2 DVs as even IVs: all 30 is Dark 70 too.
        assert_eq!(hidden_power(1, [30; 6]), (Type::Dark, 70));
        // Gen 3, the classic Fire 70: 31/30/31/30/31/30 (hp atk def spa spd spe).
        assert_eq!(hidden_power(3, [31, 30, 31, 30, 31, 30]), (Type::Fire, 70));
    }

    #[test]
    fn natures_are_ps_order() {
        assert_eq!(nature_from_id("adamant"), Some(0));
        assert_eq!(nature_from_id("timid"), Some(24));
        assert_eq!(nature_from_id("nope"), None);
    }
}
