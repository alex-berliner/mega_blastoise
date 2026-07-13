//! Live battle state: `Mon`, `Side`, plus the per-battle bookkeeping.
//!
//! Memory budget per mon ≈ 80 B; per battle (2 sides × 6 mons + bookkeeping) ≈ 1 KB.

use crate::tables::{move_by_id, species_by_id, SpeciesEntry};
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
    /// Sleep with `turns_remaining` 1..=7 (the last turn is the lost wake turn).
    Sleep(u8),
    /// Bad poison. The escalating counter lives in `Volatile::toxic_counter`
    /// so it survives Rest (Gen 1 Toxic counter glitch).
    BadPoison,
}

/// Volatile status bitfield + small payloads.
#[derive(Clone, Copy, Debug, Default)]
pub struct Volatile {
    pub flags: u32,
    pub confused_turns: u8,
    pub substitute_hp: u8,
    /// Bide accumulator, or a partial-trap lock's repeated per-turn damage.
    pub stored_damage: u16,
    pub bide_turns: u8,
    pub disabled_slot: u8,
    pub disabled_turns: u8,
    /// Move id for a locked-in multi-turn action (TwoTurn charge, Wrap, Bide,
    /// Thrash/Petal Dance, Rage). Empty means none.
    pub multi_turn_move: &'static str,
    pub multi_turn_turns: u8,
    /// Stored effective accuracy for the Thrash/Rage accuracy bug: stage
    /// multipliers compound onto LAST turn's effective accuracy each turn.
    pub locked_acc: u8,
    /// Gen 1 Toxic counter ("residualdmg"): starts at 0 when badly poisoned,
    /// increments on tox and Leech Seed residuals, multiplies psn/brn/seed
    /// damage, and survives Rest. Active while `TOX_COUNTER` flag is set.
    pub toxic_counter: u8,
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
    pub const RAGE: u32             = 1 << 15;
    /// Lose the next action (Haze cured this mon's sleep/freeze mid-turn
    /// before it acted — Gen 1 leaves it unable to move that turn).
    pub const SKIP_TURN: u32        = 1 << 16;
    /// Toxic counter is live (see `toxic_counter`).
    pub const TOX_COUNTER: u32      = 1 << 17;

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

/// Battle-global registers (Gen 1 keeps these in overworked WRAM slots, which
/// is where half its glitches come from).
#[derive(Clone, Copy, Debug, Default)]
pub struct Field {
    /// Last damage dealt in the battle by anyone, to anyone: move damage,
    /// residual poison/burn/seed, recoil, confusion self-hits, crash damage.
    /// Persists across turns; only reset when a move outside the Gen 1
    /// skip-list starts executing. Counter deals 2× this; Bide accumulates it.
    pub last_damage: u16,
    /// True while the side currently acting moves SECOND this turn (its foe
    /// already acted). Haze's cure-slp/frz-lose-turn quirk needs to know.
    pub foe_acted: bool,
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
    /// HP/Atk/Def/Spc/Spe — FINAL stats (base+IV+EV+level), never modified
    /// in battle (except by Transform). Crits read these directly.
    pub stats: [u16; 5],
    /// Gen 1 "modified" stats: stats with stat stages and the sticky
    /// paralysis/burn drops applied. Index 0 (HP) is unused. Recalculated
    /// from `stats` whenever a stage changes (which ERASES par/brn drops —
    /// the Gen 1 stat modification glitch), quartered/halved in place when
    /// paralysis/burn lands or re-stacks.
    pub modified: [u16; 5],
    /// Species base Speed at team-build time — crit rate uses this, and it
    /// survives Transform (the cartridge reads the original species).
    pub base_spe: u16,
    pub primary_type: Type,
    pub secondary_type: Type,
    pub status: Status,
    pub moves: [MoveSlot; 4],
    /// Stat stages for Atk/Def/Spc/Spe/Acc/Eva (HP doesn't stage). Range -6..=6.
    pub stages: [i8; 6],
    pub volatile: Volatile,
    /// Move id used last (for Mirror Move). Cleared while asleep/frozen.
    pub last_move_used: &'static str,
    /// Current sleep came from the mon's own Rest — exempt from Sleep Clause
    /// Mod. Persists across switches (sleep does too), ignored when awake.
    pub rest_sleep: bool,
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
            modified: [0; 5],
            base_spe: 0,
            primary_type: Type::None,
            secondary_type: Type::None,
            status: Status::None,
            moves: [MoveSlot::default(); 4],
            stages: [0; 6],
            volatile: Volatile::default(),
            last_move_used: "",
            rest_sleep: false,
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

    pub fn is_type(&self, t: Type) -> bool {
        self.primary_type == t || self.secondary_type == t
    }

    /// Find the 0-based move slot containing the given move id, if any.
    pub fn find_move_slot(&self, move_id: &str) -> Option<u8> {
        self.moves.iter().enumerate()
            .find(|(_, s)| s.move_id == move_id)
            .map(|(i, _)| i as u8)
    }

    /// True when every filled move slot is out of PP (Struggle time).
    pub fn out_of_pp(&self) -> bool {
        self.moves.iter().all(|s| s.move_id.is_empty() || s.pp == 0)
            && self.moves.iter().any(|s| !s.move_id.is_empty())
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
            modified: [hp, atk, def, spc, spe],
            base_spe: sp.base_stats[4],
            primary_type: sp.primary_type,
            secondary_type: sp.secondary_type,
            status: Status::None,
            moves,
            stages: [0; 6],
            volatile: Volatile::default(),
            last_move_used: "",
            rest_sleep: false,
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

/// Pre-Transform snapshot of the transforming mon, restored when it leaves
/// the field (Gen 1: Transform reverts on switch-out / faint).
#[derive(Clone, Copy, Debug)]
pub struct TransformBackup {
    pub species_id: &'static str,
    pub primary_type: Type,
    pub secondary_type: Type,
    pub stats: [u16; 5],
    pub moves: [MoveSlot; 4],
}

/// One side (player) — 6 mons + active index.
#[derive(Clone, Debug, Default)]
pub struct Side {
    pub player_id: heapless::String<8>,
    pub name: heapless::String<16>,
    pub team: [Mon; 6],
    pub active_idx: u8,
    /// Last move this side actually USED (announced). Not cleared by sleep —
    /// Counter reads this, which is why stale Counters work.
    pub last_move_used: &'static str,
    /// Last move this side SELECTED. Counter's desync-clause check reads the
    /// opponent's selection.
    pub last_selected_move: &'static str,
    /// Set while the active mon is TRANSFORMED (only the active mon can be).
    pub transform_backup: Option<TransformBackup>,
}

impl Side {
    pub fn active(&self) -> &Mon {
        &self.team[self.active_idx as usize]
    }
    pub fn active_mut(&mut self) -> &mut Mon {
        &mut self.team[self.active_idx as usize]
    }
}
