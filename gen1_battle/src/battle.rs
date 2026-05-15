//! The `Battle` handle — main public entry point.
//!
//! Drives a Gen 1 battle: setup → start → per-turn request/choice loop → end.
//! All combat math lives in `combat.rs` + `dispatch.rs`; this module is the
//! state machine + public API.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use anyhow::{anyhow, Result};
use hashbrown::HashMap;

use crate::data::{
    BoostTable, MonBattleData, MonSummary, MoveSlot as ApiMoveSlot, PlayerBattleData, TeamData,
};
use crate::dispatch::{end_of_turn, execute_move, locked_move_slot, pre_move_check, Log};
use crate::options::{CoreBattleEngineOptions, CoreBattleOptions};
use crate::request::{MonTurnRequest, Request, SwitchRequest, TurnRequest};
use crate::rng::Rng;
use crate::state::{Mon, MoveSlot, Side, Status};
use crate::tables::{move_by_id, species_by_id};
use crate::types::{Stat, Type};

#[derive(Clone, Copy, Debug, Default)]
enum Phase {
    #[default]
    Setup,
    Turn,
    AwaitSwitch,
    Ended,
}

/// Pending choice for one side. Either move slot 0..3 or switch to team slot 0..5.
#[derive(Clone, Copy, Debug)]
enum Choice {
    Move(u8),
    Switch(u8),
}

pub struct Battle<'a> {
    rng: Rng,
    _engine: CoreBattleEngineOptions,
    _data: core::marker::PhantomData<&'a ()>,
    sides: [Side; 2],
    pending_choice: [Option<Choice>; 2],
    pending_switch_needed: [bool; 2],
    phase: Phase,
    log: Log,
    requests: Vec<(String, Request)>,
    winner: Option<u8>,
}

pub type PublicCoreBattle<'a> = Battle<'a>;

impl<'a> Battle<'a> {
    pub fn new<D: ?Sized>(
        opts: CoreBattleOptions,
        _data: &'a D,
        engine: CoreBattleEngineOptions,
    ) -> Result<Self> {
        let seed = opts.seed.unwrap_or(0xDEAD_BEEF);
        let mut sides = <[Side; 2]>::default();
        // Tag side player IDs based on side_1.players[0].id, etc.
        if let Some(p) = opts.side_1.players.first() {
            let _ = sides[0].player_id.push_str(&p.id);
            let _ = sides[0].name.push_str(&opts.side_1.name);
        }
        if let Some(p) = opts.side_2.players.first() {
            let _ = sides[1].player_id.push_str(&p.id);
            let _ = sides[1].name.push_str(&opts.side_2.name);
        }
        if sides[0].player_id.is_empty() {
            let _ = sides[0].player_id.push_str("p1");
        }
        if sides[1].player_id.is_empty() {
            let _ = sides[1].player_id.push_str("p2");
        }

        Ok(Self {
            rng: Rng::new(seed),
            _engine: engine,
            _data: core::marker::PhantomData,
            sides,
            pending_choice: [None, None],
            pending_switch_needed: [false, false],
            phase: Phase::Setup,
            log: Log::new(),
            requests: Vec::new(),
            winner: None,
        })
    }

    pub fn update_team(&mut self, player_id: &str, team: TeamData) -> Result<()> {
        let side_idx = self.find_side(player_id)?;
        for (i, mdata) in team.members.iter().take(6).enumerate() {
            // Pick species: prefer species id else species name lowercased.
            let key = canonical_id(&mdata.species);
            let species_id = species_by_id(&key)
                .map(|s| s.id)
                .ok_or_else(|| anyhow!("unknown species: {}", &mdata.species))?;
            let move_ids: alloc::vec::Vec<&'static str> = mdata
                .moves
                .iter()
                .take(4)
                .filter_map(|m| {
                    let k = canonical_id(&m.id);
                    move_by_id(&k).map(|mv| mv.id)
                })
                .collect();
            if let Some(mut mon) = Mon::from_species(species_id, mdata.level.max(1), &move_ids) {
                if !mdata.name.is_empty() {
                    mon.name.clear();
                    let _ = mon.name.push_str(&mdata.name);
                }
                self.sides[side_idx].team[i] = mon;
            }
        }
        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        // Validate teams have at least one mon.
        for s in &self.sides {
            if s.team.iter().all(|m| m.empty()) {
                return Err(anyhow!("empty team"));
            }
        }
        // Place active = first non-empty.
        for s in self.sides.iter_mut() {
            for (i, m) in s.team.iter().enumerate() {
                if !m.empty() {
                    s.active_idx = i as u8;
                    break;
                }
            }
        }
        // Initial switch-in narration.
        for i in 0..2 {
            self.log.push(crate::log::Event::SwitchIn {
                side: i as u8,
                slot: self.sides[i].active_idx,
            });
        }
        self.phase = Phase::Turn;
        self.rebuild_requests();
        Ok(())
    }

    pub fn ended(&self) -> bool {
        matches!(self.phase, Phase::Ended)
    }

    pub fn active_requests(&self) -> ActiveRequests<'_> {
        ActiveRequests { battle: self, idx: 0 }
    }

    pub fn set_player_choice(&mut self, player_id: &str, line: &str) -> Result<()> {
        let side_idx = self.find_side(player_id)?;
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("move ") {
            // 0-based slot per mega_blastoise_core::battle_input::format_move_choice.
            let n: u8 = rest.trim().parse().map_err(|_| anyhow!("bad move slot"))?;
            if n > 3 {
                return Err(anyhow!("move slot out of range"));
            }
            self.pending_choice[side_idx] = Some(Choice::Move(n));
        } else if let Some(rest) = trimmed.strip_prefix("switch ") {
            // 0-based team index.
            let n: u8 = rest.trim().parse().map_err(|_| anyhow!("bad switch slot"))?;
            if n > 5 {
                return Err(anyhow!("switch slot out of range"));
            }
            self.pending_choice[side_idx] = Some(Choice::Switch(n));
        } else if trimmed == "pass" {
            self.pending_choice[side_idx] = Some(Choice::Move(0));
        } else {
            return Err(anyhow!("unparseable choice: {}", trimmed));
        }
        // If both sides have submitted, advance.
        if self.pending_choice[0].is_some() && self.pending_choice[1].is_some() {
            self.advance_turn();
        }
        Ok(())
    }

    pub fn new_log_entries(&mut self) -> LogEntries {
        LogEntries { entries: core::mem::take(&mut self.log.pending).into_iter() }
    }

    pub fn player_data(&self, player_id: &str) -> Result<PlayerBattleData> {
        let side_idx = self.find_side(player_id)?;
        let s = &self.sides[side_idx];
        let mut out = PlayerBattleData {
            id: player_id.to_string(),
            name: s.name.as_str().to_string(),
            mons: Vec::new(),
        };
        for (i, m) in s.team.iter().enumerate() {
            if m.empty() {
                continue;
            }
            let mut stats: HashMap<Stat, u16> = HashMap::new();
            stats.insert(Stat::Hp, m.stats[0]);
            stats.insert(Stat::Atk, m.stats[1]);
            stats.insert(Stat::Def, m.stats[2]);
            stats.insert(Stat::SpAtk, m.stats[3]);
            stats.insert(Stat::SpDef, m.stats[3]);
            stats.insert(Stat::Spe, m.stats[4]);
            let mut types = Vec::new();
            types.push(m.primary_type);
            if !matches!(m.secondary_type, Type::None) {
                types.push(m.secondary_type);
            }
            let moves = m
                .moves
                .iter()
                .filter(|sl| !sl.move_id.is_empty())
                .map(|sl| {
                    let info = move_by_id(sl.move_id).unwrap();
                    ApiMoveSlot {
                        name: info.name.to_string(),
                        id: info.id.to_string(),
                        typ: format!("{:?}", info.move_type),
                        pp: sl.pp,
                        max_pp: sl.max_pp,
                        disabled: false,
                        target: 0,
                    }
                })
                .collect();
            out.mons.push(MonBattleData {
                active: i as u8 == s.active_idx,
                player_team_position: i as u8,
                hp: m.hp_cur,
                max_hp: m.hp_max,
                status: status_label(m.status),
                species: m.species_id.to_string(),
                ability: None,
                types,
                item: None,
                summary: MonSummary {
                    name: m.name.as_str().to_string(),
                    species: m.species_id.to_string(),
                    level: m.level,
                },
                stats,
                boosts: BoostTable {
                    atk: m.stages[0],
                    def: m.stages[1],
                    spa: m.stages[2],
                    spd: m.stages[2],
                    spe: m.stages[3],
                    acc: 0,
                    eva: 0,
                },
                moves,
            });
        }
        Ok(out)
    }

    pub fn active_mon_move_pp(&self, player_id: &str) -> Option<Vec<(u8, u8)>> {
        let side_idx = self.find_side(player_id).ok()?;
        let a = self.sides[side_idx].active();
        Some(a.moves.iter().map(|s| (s.pp, s.max_pp)).collect())
    }

    pub fn drain_action_timings(&mut self) -> alloc::vec::IntoIter<(&'static str, u32)> {
        Vec::new().into_iter()
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn find_side(&self, player_id: &str) -> Result<usize> {
        for (i, s) in self.sides.iter().enumerate() {
            if s.player_id.as_str() == player_id {
                return Ok(i);
            }
        }
        Err(anyhow!("unknown player_id: {}", player_id))
    }

    fn rebuild_requests(&mut self) {
        self.requests.clear();
        if matches!(self.phase, Phase::Ended) {
            return;
        }
        for (i, s) in self.sides.iter().enumerate() {
            let player_id = s.player_id.as_str().to_string();
            if self.pending_switch_needed[i] {
                // Switch request.
                self.requests.push((
                    player_id,
                    Request::Switch(SwitchRequest {
                        needs_switch: alloc::vec![s.active_idx],
                    }),
                ));
                continue;
            }
            // Turn request for the active mon.
            let m = s.active();
            if m.empty() || m.fainted() {
                continue;
            }
            let moves: Vec<ApiMoveSlot> = m
                .moves
                .iter()
                .filter(|sl| !sl.move_id.is_empty())
                .map(|sl| {
                    let info = move_by_id(sl.move_id).unwrap();
                    ApiMoveSlot {
                        name: info.name.to_string(),
                        id: info.id.to_string(),
                        typ: format!("{:?}", info.move_type),
                        pp: sl.pp,
                        max_pp: sl.max_pp,
                        disabled: false,
                        target: 0,
                    }
                })
                .collect();
            let v = &m.volatile;
            let active = MonTurnRequest {
                team_position: s.active_idx,
                moves,
                trapped: v.has(crate::state::Volatile::TRAPPED)
                    || v.has(crate::state::Volatile::BIDING)
                    || v.has(crate::state::Volatile::CHARGING)
                    || !v.multi_turn_move.is_empty(),
                locked_into_move: v.has(crate::state::Volatile::MUST_RECHARGE)
                    || v.has(crate::state::Volatile::CHARGING)
                    || v.has(crate::state::Volatile::BIDING)
                    || !v.multi_turn_move.is_empty(),
            };
            self.requests
                .push((player_id, Request::Turn(TurnRequest { active: alloc::vec![active] })));
        }
    }

    fn advance_turn(&mut self) {
        // Execute according to phase.
        match self.phase {
            Phase::Turn => self.do_battle_turn(),
            Phase::AwaitSwitch => self.do_forced_switches(),
            _ => {}
        }
        self.pending_choice = [None, None];
        // Check for forced switches (a mon fainted).
        for i in 0..2 {
            if self.sides[i].active().fainted() && !self.team_wiped(i) {
                self.pending_switch_needed[i] = true;
            } else {
                self.pending_switch_needed[i] = false;
            }
        }
        if self.team_wiped(0) {
            self.winner = Some(1);
            self.log.push(crate::log::Event::Win { side: 1 });
            self.phase = Phase::Ended;
        } else if self.team_wiped(1) {
            self.winner = Some(0);
            self.log.push(crate::log::Event::Win { side: 0 });
            self.phase = Phase::Ended;
        } else if self.pending_switch_needed[0] || self.pending_switch_needed[1] {
            self.phase = Phase::AwaitSwitch;
        } else {
            self.phase = Phase::Turn;
        }
        self.rebuild_requests();
    }

    fn do_forced_switches(&mut self) {
        for i in 0..2 {
            if !self.pending_switch_needed[i] {
                continue;
            }
            if let Some(Choice::Switch(slot)) = self.pending_choice[i] {
                if (slot as usize) < 6 && !self.sides[i].team[slot as usize].empty()
                    && !self.sides[i].team[slot as usize].fainted()
                {
                    self.sides[i].active_idx = slot;
                    self.log.push(crate::log::Event::SwitchIn { side: i as u8, slot });
                }
            } else {
                // Auto-pick first alive.
                for (j, m) in self.sides[i].team.iter().enumerate() {
                    if !m.empty() && !m.fainted() && j as u8 != self.sides[i].active_idx {
                        self.sides[i].active_idx = j as u8;
                        self.log.push(crate::log::Event::SwitchIn { side: i as u8, slot: j as u8 });
                        break;
                    }
                }
            }
        }
    }

    fn do_battle_turn(&mut self) {
        // Determine action order: switches first, then by Speed.
        let mut order = [0usize, 1usize];
        let p0_switch = matches!(self.pending_choice[0], Some(Choice::Switch(_)));
        let p1_switch = matches!(self.pending_choice[1], Some(Choice::Switch(_)));
        if p0_switch && !p1_switch {
            order = [0, 1];
        } else if p1_switch && !p0_switch {
            order = [1, 0];
        } else {
            let s0 = self.sides[0].active().stats[4];
            let s1 = self.sides[1].active().stats[4];
            if s1 > s0 || (s1 == s0 && self.rng.coin()) {
                order = [1, 0];
            }
        }

        // Reset per-turn counter source damage.
        for s in self.sides.iter_mut() {
            s.active_mut().counter_source_dmg = 0;
        }

        for &side in &order {
            if self.sides[side].active().fainted() {
                continue;
            }
            match self.pending_choice[side] {
                Some(Choice::Switch(slot)) => {
                    let s = (slot as usize).min(5);
                    if !self.sides[side].team[s].empty() && !self.sides[side].team[s].fainted() {
                        self.sides[side].active_idx = slot;
                        self.log.push(crate::log::Event::SwitchIn { side: side as u8, slot });
                    }
                }
                Some(Choice::Move(slot)) => {
                    // Override with locked-in move if any (TwoTurn release, Bide, Wrap).
                    let effective = locked_move_slot(&self.sides[side]).unwrap_or(slot);
                    if pre_move_check(&mut self.rng, &mut self.sides, side, &mut self.log) {
                        let _ = execute_move(
                            &mut self.rng,
                            &mut self.sides,
                            side,
                            effective as usize,
                            &mut self.log,
                        );
                    }
                }
                None => {
                    // No choice (e.g. mon fainted before choice was set);
                    // honor a locked-in move if present.
                    if let Some(forced) = locked_move_slot(&self.sides[side]) {
                        if pre_move_check(&mut self.rng, &mut self.sides, side, &mut self.log) {
                            let _ = execute_move(
                                &mut self.rng,
                                &mut self.sides,
                                side,
                                forced as usize,
                                &mut self.log,
                            );
                        }
                    }
                }
            }
        }

        end_of_turn(&mut self.sides, &mut self.log);
    }

    fn team_wiped(&self, side: usize) -> bool {
        self.sides[side]
            .team
            .iter()
            .all(|m| m.empty() || m.fainted())
    }
}

fn status_label(s: Status) -> Option<String> {
    Some(match s {
        Status::None => return None,
        Status::Poison => "psn".to_string(),
        Status::BadPoison(_) => "tox".to_string(),
        Status::Burn => "brn".to_string(),
        Status::Freeze => "frz".to_string(),
        Status::Paralysis => "par".to_string(),
        Status::Sleep(_) => "slp".to_string(),
    })
}

fn canonical_id(s: &str) -> heapless::String<32> {
    let mut out = heapless::String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            for d in c.to_lowercase() {
                let _ = out.push(d);
            }
        }
    }
    out
}

/// Iterator returned by [`Battle::active_requests`].
pub struct ActiveRequests<'a> {
    battle: &'a Battle<'a>,
    idx: usize,
}

impl<'a> Iterator for ActiveRequests<'a> {
    type Item = (&'a str, &'a Request);
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.battle.requests.len() {
            return None;
        }
        let (pid, req) = &self.battle.requests[self.idx];
        self.idx += 1;
        Some((pid.as_str(), req))
    }
}

/// Iterator returned by [`Battle::new_log_entries`].
pub struct LogEntries {
    entries: alloc::vec::IntoIter<String>,
}

impl Iterator for LogEntries {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next()
    }
}
