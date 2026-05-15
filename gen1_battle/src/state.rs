//! Live battle state: `Mon`, `Side`, plus the per-battle bookkeeping.
//!
//! Memory budget per mon ≈ 64 B; per battle (2 sides × 6 mons + bookkeeping) ≈ 1 KB.

use crate::tables::{move_by_id, species_by_id, MoveEntry, SpeciesEntry};
use crate::types::Type;

/// Status (major) — exactly one at a time, plus sleep counter.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    None,
    Poison,
    Burn,
    Freeze,
    Paralysis,
    /// Sleep with `turns_remaining` 1..=7.
    Sleep(u8),
    /// Bad poison with `counter` 1..=15 (capped).
    BadPoison(u8),
}

/// Volatile status bitfield + small payloads.
#[derive(Clone, Copy, Debug, Default)]
pub struct Volatile {
    pub flags: u32,
    pub confused_turns: u8,
    pub substitute_hp: u8,
    pub bide_damage: u16,
    pub bide_turns: u8,
    pub disabled_slot: u8,
    pub disabled_turns: u8,
    pub multi_turn_move: u8,
    pub multi_turn_turns: u8,
    pub reflect_turns: u8,
    pub light_screen_turns: u8,
    pub trapping_user_side: u8,  // 0 = none, 1 = p1, 2 = p2
    pub trapping_turns: u8,
}

impl Volatile {
    pub const FLINCHED: u32         = 1 << 0;
    pub const CONFUSED: u32         = 1 << 1;
    pub const LEECH_SEEDED: u32     = 1 << 2;
    pub const SUBSTITUTED: u32      = 1 << 3;
    pub const MUST_RECHARGE: u32    = 1 << 4;
    pub const BIDING: u32           = 1 << 5;
    pub const DISABLED: u32         = 1 << 6;
    pub const REFLECT: u32          = 1 << 7;
    pub const LIGHT_SCREEN: u32     = 1 << 8;
    pub const FOCUS_ENERGY: u32     = 1 << 9;
    pub const MIST: u32             = 1 << 10;
    pub const CHARGING: u32         = 1 << 11;
    pub const INVULNERABLE: u32     = 1 << 12;
    pub const TRAPPED: u32          = 1 << 13;
    pub const TRANSFORMED: u32      = 1 << 14;

    pub fn set(&mut self, f: u32) {
        self.flags |= f;
    }
    pub fn clear(&mut self, f: u32) {
        self.flags &= !f;
    }
    pub fn has(&self, f: u32) -> bool {
        (self.flags & f) != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MoveSlot {
    pub move_id: &'static str,
    pub pp: u8,
    pub max_pp: u8,
}

/// One Pokémon's live battle state.
#[derive(Clone, Debug)]
pub struct Mon {
    pub species_id: &'static str,
    pub name: heapless::String<16>, // owned for display; capped 16 chars
    pub level: u8,
    pub hp_cur: u16,
    pub hp_max: u16,
    /// HP/Atk/Def/Spc/Spe — these are FINAL stats (computed from base+IV+EV+level).
    pub stats: [u16; 5],
    pub primary_type: Type,
    pub secondary_type: Type,
    pub status: Status,
    pub moves: [MoveSlot; 4],
    /// Stat stages for Atk/Def/Spc/Spe (HP doesn't stage). Range -6..=6.
    pub stages: [i8; 4],
    pub volatile: Volatile,
    /// Move id used last turn (for Mirror Move).
    pub last_move_used: &'static str,
    /// Last damage taken from a Normal/Fighting damaging move this turn (Counter).
    pub counter_source_dmg: u16,
}

impl Default for Mon {
    fn default() -> Self {
        Self {
            species_id: "",
            name: heapless::String::new(),
            level: 0,
            hp_cur: 0,
            hp_max: 0,
            stats: [0; 5],
            primary_type: Type::None,
            secondary_type: Type::None,
            status: Status::None,
            moves: [MoveSlot::default(); 4],
            stages: [0; 4],
            volatile: Volatile::default(),
            last_move_used: "",
            counter_source_dmg: 0,
        }
    }
}

impl Mon {
    /// True if mon slot is empty / no species set.
    pub fn empty(&self) -> bool {
        self.species_id.is_empty()
    }

    pub fn fainted(&self) -> bool {
        !self.empty() && self.hp_cur == 0
    }

    /// Initialize a mon from species id + level + chosen moves.
    /// All IVs and EVs assumed max (15 / 65535) — randbat-style.
    pub fn from_species(species_id: &'static str, level: u8, move_ids: &[&'static str]) -> Option<Self> {
        let sp: &SpeciesEntry = species_by_id(species_id)?;
        let hp = compute_hp(sp.base_stats[0], level);
        let atk = compute_stat(sp.base_stats[1], level);
        let def = compute_stat(sp.base_stats[2], level);
        let spc = compute_stat(sp.base_stats[3], level);
        let spe = compute_stat(sp.base_stats[4], level);

        let mut moves = [MoveSlot::default(); 4];
        for (i, mid) in move_ids.iter().take(4).enumerate() {
            if let Some(mv) = move_by_id(mid) {
                moves[i] = MoveSlot { move_id: mv.id, pp: mv.pp, max_pp: mv.pp };
            }
        }

        let mut name = heapless::String::new();
        let _ = name.push_str(sp.name);

        Some(Mon {
            species_id: sp.id,
            name,
            level,
            hp_cur: hp,
            hp_max: hp,
            stats: [hp, atk, def, spc, spe],
            primary_type: sp.primary_type,
            secondary_type: sp.secondary_type,
            status: Status::None,
            moves,
            stages: [0; 4],
            volatile: Volatile::default(),
            last_move_used: "",
            counter_source_dmg: 0,
        })
    }
}

/// Gen 1 stat computation: max IV=15, max EV=65535 (per-stat); we just assume max.
/// Formula: floor((((base + iv) * 2 + ceil(sqrt(ev))/4) * level) / 100) + 5
fn compute_stat(base: u16, level: u8) -> u16 {
    let iv = 15u32;
    let ev_term = 63u32; // ceil(sqrt(65535))/4 ≈ 64; using 63 matches gen1 ceiling
    let v = (((base as u32 + iv) * 2 + ev_term) * level as u32) / 100 + 5;
    v.min(999) as u16
}

fn compute_hp(base: u16, level: u8) -> u16 {
    let iv = 15u32;
    let ev_term = 63u32;
    let v = (((base as u32 + iv) * 2 + ev_term) * level as u32) / 100 + level as u32 + 10;
    v.min(999) as u16
}

/// One side (player) — 6 mons + active index.
#[derive(Clone, Debug, Default)]
pub struct Side {
    pub player_id: heapless::String<8>,
    pub name: heapless::String<16>,
    pub team: [Mon; 6],
    pub active_idx: u8,
    pub reflect_turns: u8,
    pub light_screen_turns: u8,
    pub last_move_used: &'static str,
    pub last_move_was_normal_or_fighting: bool,
    pub last_move_damage: u16,
}

impl Side {
    pub fn active(&self) -> &Mon {
        &self.team[self.active_idx as usize]
    }
    pub fn active_mut(&mut self) -> &mut Mon {
        &mut self.team[self.active_idx as usize]
    }
}
