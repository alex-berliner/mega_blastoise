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
use crate::dispatch::{
    after_action_residuals, after_switch_in, execute_locked_move, execute_move, execute_struggle,
    locked_move_id, pre_move_check, Log,
};
use crate::options::{CoreBattleEngineOptions, CoreBattleOptions};
use crate::request::{MonTurnRequest, Request, SwitchRequest, TurnRequest};
use crate::rng::Rng;
use crate::state::{Field, Mon, Side, Status, Volatile};
use crate::tables::{move_by_id, species_by_id, MoveEffectKind, MoveEntry, FLAG_PRIO_MINUS, FLAG_PRIO_PLUS};
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
    field: Field,
    pending_choice: [Option<Choice>; 2],
    pending_switch_needed: [bool; 2],
    phase: Phase,
    log: Log,
    requests: Vec<(String, Request)>,
    winner: Option<u8>,
    turn_count: u32,
}

/// Endless Battle Clause (format rule): hard cap on battle length. Pokémon
/// Showdown ties at turn 1000; this API has no tie, so the side with the
/// higher remaining-HP fraction wins (coin flip on an exact tie).
const ENDLESS_BATTLE_TURN_CAP: u32 = 1000;

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
            field: Field::default(),
            pending_choice: [None, None],
            pending_switch_needed: [false, false],
            phase: Phase::Setup,
            log: Log::new(),
            requests: Vec::new(),
            winner: None,
            turn_count: 0,
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
            let s = &self.sides[i];
            self.log.push_board(format!("switch|player:{}|name:{}", s.player_id, s.active().name));
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
        // Advance once every side with a pending request has submitted (during
        // a forced switch only the switching side is asked to act).
        let all_in = self.requests.iter().all(|(pid, _)| {
            self.find_side(pid).map(|i| self.pending_choice[i].is_some()).unwrap_or(true)
        });
        if all_in {
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
                    acc: m.stages[4],
                    eva: m.stages[5],
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
            // During a forced-switch phase only the switching side acts; the
            // other player waits and picks their next move once the
            // replacement is out.
            if matches!(self.phase, Phase::AwaitSwitch) {
                continue;
            }
            // Turn request for the active mon.
            let m = s.active();
            if m.empty() || m.fainted() {
                continue;
            }
            let v = &m.volatile;
            let recharging = v.has(Volatile::MUST_RECHARGE);
            // A recharging mon's only option is "Recharge" — shown as a real,
            // selectable move rather than auto-submitted, so the player sees
            // what's happening. Any move choice resolves to the recharge turn.
            // Likewise a mon with no PP left anywhere is offered Struggle.
            let moves: Vec<ApiMoveSlot> = if recharging {
                alloc::vec![ApiMoveSlot {
                    name: "Recharge".to_string(),
                    id: "recharge".to_string(),
                    typ: "Normal".to_string(),
                    pp: 1,
                    max_pp: 1,
                    disabled: false,
                    target: 0,
                }]
            } else if m.out_of_pp() && v.multi_turn_move.is_empty() {
                alloc::vec![ApiMoveSlot {
                    name: "Struggle".to_string(),
                    id: "struggle".to_string(),
                    typ: "Normal".to_string(),
                    pp: 1,
                    max_pp: 1,
                    disabled: false,
                    target: 0,
                }]
            } else {
                m.moves
                    .iter()
                    .enumerate()
                    .filter(|(_, sl)| !sl.move_id.is_empty())
                    .map(|(slot, sl)| {
                        let info = move_by_id(sl.move_id).unwrap();
                        ApiMoveSlot {
                            name: info.name.to_string(),
                            id: info.id.to_string(),
                            typ: format!("{:?}", info.move_type),
                            pp: sl.pp,
                            max_pp: sl.max_pp,
                            disabled: v.has(Volatile::DISABLED)
                                && v.disabled_slot as usize == slot,
                            target: 0,
                        }
                    })
                    .collect()
            };
            let active = MonTurnRequest {
                team_position: s.active_idx,
                moves,
                trapped: recharging
                    || v.has(Volatile::TRAPPED)
                    || v.has(Volatile::BIDING)
                    || v.has(Volatile::CHARGING)
                    || !v.multi_turn_move.is_empty(),
                locked_into_move: !recharging
                    && (v.has(Volatile::CHARGING)
                        || v.has(Volatile::BIDING)
                        || !v.multi_turn_move.is_empty()),
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
            self.log.push_board(format!("win|side:1"));
            self.phase = Phase::Ended;
        } else if self.team_wiped(1) {
            self.winner = Some(0);
            self.log.push_board(format!("win|side:0"));
            self.phase = Phase::Ended;
        } else if self.turn_count >= ENDLESS_BATTLE_TURN_CAP {
            let w = self.endless_battle_winner();
            self.winner = Some(w);
            self.log.push_board(format!("win|side:{}", w));
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
                    crate::dispatch::reset_on_switch_out(&mut self.sides[i]);
                    self.sides[i].active_idx = slot;
                    {
                        let s = &self.sides[i];
                        self.log.push_board(format!("switch|player:{}|name:{}", s.player_id, s.active().name));
                    }
                    after_switch_in(&mut self.field, &mut self.sides, i, &mut self.log);
                }
            } else {
                // Auto-pick first alive.
                let pick = self.sides[i].team.iter().enumerate()
                    .find(|(j, m)| !m.empty() && !m.fainted() && *j as u8 != self.sides[i].active_idx)
                    .map(|(j, _)| j as u8);
                if let Some(j) = pick {
                    crate::dispatch::reset_on_switch_out(&mut self.sides[i]);
                    self.sides[i].active_idx = j;
                    {
                        let s = &self.sides[i];
                        self.log.push_board(format!("switch|player:{}|name:{}", s.player_id, s.active().name));
                    }
                    after_switch_in(&mut self.field, &mut self.sides, i, &mut self.log);
                }
            }
        }
    }

    /// The move a side will actually use this turn: a multi-turn lock beats
    /// the player's pick; an empty PP pool forces Struggle.
    fn effective_move(&self, side: usize) -> Option<&'static MoveEntry> {
        if self.sides[side].active().volatile.has(Volatile::MUST_RECHARGE) {
            return None; // recharge turn: normal priority bracket
        }
        if let Some(id) = locked_move_id(&self.sides[side]) {
            return move_by_id(id);
        }
        match self.pending_choice[side] {
            Some(Choice::Move(slot)) => {
                let m = self.sides[side].active();
                if m.out_of_pp() {
                    return move_by_id("struggle");
                }
                move_by_id(m.moves[(slot as usize).min(3)].move_id)
            }
            _ => None,
        }
    }

    fn do_battle_turn(&mut self) {
        self.turn_count += 1;
        // Determine action order: switches first; between moves, priority
        // bracket (Quick Attack +1, Counter -1), then modified Speed, then a
        // coin flip on ties.
        let mut order = [0usize, 1usize];
        let p0_switch = matches!(self.pending_choice[0], Some(Choice::Switch(_)));
        let p1_switch = matches!(self.pending_choice[1], Some(Choice::Switch(_)));
        if p0_switch && !p1_switch {
            order = [0, 1];
        } else if p1_switch && !p0_switch {
            order = [1, 0];
        } else {
            let prio = |mv: Option<&MoveEntry>| -> i8 {
                match mv {
                    Some(m) if m.flags & FLAG_PRIO_PLUS != 0 => 1,
                    Some(m) if m.flags & FLAG_PRIO_MINUS != 0 => -1,
                    _ => 0,
                }
            };
            let pr0 = prio(self.effective_move(0));
            let pr1 = prio(self.effective_move(1));
            let s0 = self.sides[0].active().modified[4];
            let s1 = self.sides[1].active().modified[4];
            if pr1 > pr0 || (pr1 == pr0 && (s1 > s0 || (s1 == s0 && self.rng.coin()))) {
                order = [1, 0];
            }
        }

        // Selections register before either side moves (Counter reads the
        // opponent's selected move).
        for side in 0..2 {
            if matches!(self.pending_choice[side], Some(Choice::Move(_))) {
                if let Some(mv) = self.effective_move(side) {
                    self.sides[side].last_selected_move = mv.id;
                }
            }
        }

        for (i, &side) in order.iter().enumerate() {
            if self.sides[side].active().fainted() {
                continue;
            }
            self.field.foe_acted = i == 1;
            match self.pending_choice[side] {
                Some(Choice::Switch(slot)) => {
                    let s = (slot as usize).min(5);
                    if !self.sides[side].team[s].empty() && !self.sides[side].team[s].fainted() {
                        crate::dispatch::reset_on_switch_out(&mut self.sides[side]);
                        self.sides[side].active_idx = slot;
                        {
                            let si = &self.sides[side];
                            self.log.push_board(format!("switch|player:{}|name:{}", si.player_id, si.active().name));
                        }
                        after_switch_in(&mut self.field, &mut self.sides, side, &mut self.log);
                    }
                }
                Some(Choice::Move(slot)) => {
                    self.run_move_action(side, Some(slot));
                }
                None => {
                    // No choice (e.g. mon fainted before choice was set);
                    // honor a locked-in move if present.
                    if locked_move_id(&self.sides[side]).is_some() {
                        self.run_move_action(side, None);
                    }
                }
            }
            // Gen 1: any faint ends the turn — the other side's action (and
            // residuals) simply don't happen.
            if self.sides[0].active().fainted() || self.sides[1].active().fainted() {
                break;
            }
        }

        // End-of-turn cleanup (not residuals — those ran per action).
        for i in 0..2 {
            self.sides[i].active_mut().volatile.clear(Volatile::FLINCHED);
            // Free a partial-trap victim once the trapper's lock is gone
            // (ended, cancelled, switched away, or fainted).
            if self.sides[i].active().volatile.has(Volatile::TRAPPED) {
                let foe = &self.sides[1 - i];
                let lock_active = !foe.active().fainted()
                    && !foe.active().volatile.multi_turn_move.is_empty()
                    && move_by_id(foe.active().volatile.multi_turn_move)
                        .map(|m| m.effect_kind == MoveEffectKind::Wrap)
                        .unwrap_or(false);
                if !lock_active {
                    self.sides[i].active_mut().volatile.clear(Volatile::TRAPPED);
                }
            }
        }
    }

    /// One side's move action: pre-move gate, execution (locked / struggle /
    /// chosen slot), then Gen 1's after-action residual damage.
    fn run_move_action(&mut self, side: usize, chosen_slot: Option<u8>) {
        let locked = locked_move_id(&self.sides[side]);
        // For the Disable check, the relevant slot is the one about to fire.
        let disable_slot = match locked {
            Some(id) => self.sides[side].active().find_move_slot(id),
            None => chosen_slot,
        };
        if pre_move_check(&mut self.rng, &mut self.field, &mut self.sides, side, disable_slot, &mut self.log) {
            if let Some(id) = locked_move_id(&self.sides[side]) {
                let _ = execute_locked_move(&mut self.rng, &mut self.field, &mut self.sides, side, id, &mut self.log);
            } else if self.sides[side].active().out_of_pp() {
                let _ = execute_struggle(&mut self.rng, &mut self.field, &mut self.sides, side, &mut self.log);
            } else if let Some(slot) = chosen_slot {
                let _ = execute_move(
                    &mut self.rng,
                    &mut self.field,
                    &mut self.sides,
                    side,
                    slot as usize,
                    &mut self.log,
                );
            }
        }
        after_action_residuals(&mut self.field, &mut self.sides, side, &mut self.log);
    }

    fn team_wiped(&self, side: usize) -> bool {
        self.sides[side]
            .team
            .iter()
            .all(|m| m.empty() || m.fainted())
    }

    /// Endless Battle Clause resolution: higher remaining-HP fraction wins,
    /// exact tie decided by the battle RNG (deterministic per seed).
    fn endless_battle_winner(&mut self) -> u8 {
        let totals = |side: &Side| -> (u64, u64) {
            side.team.iter().filter(|m| !m.empty()).fold((0u64, 0u64), |(c, m), mon| {
                (c + mon.hp_cur as u64, m + mon.hp_max as u64)
            })
        };
        let (cur0, max0) = totals(&self.sides[0]);
        let (cur1, max1) = totals(&self.sides[1]);
        // Compare cur0/max0 vs cur1/max1 without floats.
        let lhs = cur0 * max1.max(1);
        let rhs = cur1 * max0.max(1);
        if lhs > rhs {
            0
        } else if rhs > lhs {
            1
        } else if self.rng.coin() {
            0
        } else {
            1
        }
    }
}

fn status_label(s: Status) -> Option<String> {
    Some(match s {
        Status::None => return None,
        Status::Poison => "psn".to_string(),
        Status::BadPoison => "tox".to_string(),
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
