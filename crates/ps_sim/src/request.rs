//! Requests: PS's `makeRequest` / `getRequests` and the per-Pokemon data
//! they carry, built into [`ps_protocol::Request`].
//!
//! PS computes a request once (`makeRequest`) and sends it later
//! (`sendUpdates`, `emitRequest`). What it computes for the active Pokemon
//! (`getMoveRequestData`) has side effects and can be edited afterwards
//! (`updateRequestForPokemon`), so that part is stored on the side
//! ([`SideRequestState`]); the `side` part (`getRequestData`) reads only
//! state that no choice changes, so it is built when the request is sent.
//!
//! PS: sim/battle.ts `makeRequest`, `clearRequest`, `getRequests`,
//! `canSwitch`, `getRandomSwitchable`; sim/side.ts `getRequestData`,
//! `requestState`; sim/pokemon.ts `getMoveRequestData`, `getMoves`,
//! `getLockedMove`, `getSemiLockedMove`, `getSwitchRequestData`,
//! `isLastActive`, `getHealth`, `details`.

use ps_dex::MoveId;
use ps_protocol::{
    AbilityNum, ActiveRequest, BoundedVec, Details, Health, Hp, HpStatus, ItemNum, MoveNum, MoveRequest, MoveTarget,
    Request, RequestMove, RequestMoveName, SideMoveId, SidePokemon, SideRequest, SpeciesNum, Status, SwitchRequest,
    TypeNum, WaitRequest, MAX_MOVES,
};

use crate::effect::{EffectId, EventId};
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, Trapped};

/// PS's `RequestState` (no team preview in our formats).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RequestState {
    /// `''`
    #[default]
    None,
    Move,
    Switch,
}

/// A side's `activeRequest`: what was computed at `makeRequest` time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideRequestState {
    /// `{active: [...], side, noCancel?, update?}`
    Move { active: ActiveRequest, no_cancel: bool, update: bool },
    /// `{forceSwitch: [...], side, noCancel?, update?}` (singles: one flag).
    Switch { force_switch: bool, no_cancel: bool, update: bool },
    /// `{wait: true, side}`
    Wait,
}

impl SideRequestState {
    /// `side.requestState`.
    pub fn state(&self) -> RequestState {
        match self {
            SideRequestState::Move { .. } => RequestState::Move,
            SideRequestState::Switch { .. } => RequestState::Switch,
            SideRequestState::Wait => RequestState::None,
        }
    }
}

/// The `-status` / `HP STATUS` form of a status condition id.
pub(crate) fn status_of(b: &Battle, m: MonRef) -> Option<Status> {
    let st = b.mon(m).status;
    if st.is_none() {
        return None;
    }
    Status::parse(b.dex.condition_def(st).key.as_str())
}

impl Battle {
    /// The stored `active[0]` of `side`'s move request.
    pub fn stored_active_request_mut(&mut self, side: u8) -> Option<&mut ActiveRequest> {
        match &mut self.sides[side as usize].active_request {
            Some(SideRequestState::Move { active, .. }) => Some(active),
            _ => None,
        }
    }

    /// PS `makeRequest(type)`. `None` re-makes the current request.
    // PS: sim/battle.ts makeRequest
    pub fn make_request(&mut self, kind: Option<RequestState>) {
        let kind = match kind {
            Some(k) => {
                self.request_state = k;
                for s in 0..2 {
                    self.clear_choice(s);
                }
                k
            }
            None => self.request_state,
        };
        for s in 0..2 {
            self.sides[s].active_request = None;
        }
        let requests = self.get_requests(kind);
        for (s, r) in requests.into_iter().enumerate() {
            self.sides[s].active_request = Some(r);
        }
        self.sent_requests = false;
        if (0..2).all(|s| self.is_choice_done(s)) {
            panic!("Choices are done immediately after a request");
        }
    }

    /// PS `clearRequest()`.
    // PS: sim/battle.ts clearRequest
    pub fn clear_request(&mut self) {
        self.request_state = RequestState::None;
        for s in 0..2u8 {
            self.sides[s as usize].active_request = None;
            self.clear_choice(s);
        }
    }

    /// PS `getRequests(type)`.
    // PS: sim/battle.ts getRequests
    pub fn get_requests(&mut self, kind: RequestState) -> [SideRequestState; 2] {
        let mut requests: [Option<SideRequestState>; 2] = [None, None];
        match kind {
            RequestState::Switch => {
                for s in 0..2u8 {
                    if self.sides[s as usize].pokemon_left == 0 {
                        continue;
                    }
                    let flag = self.active_at(s, 0).is_some_and(|m| self.mon(m).switch_flag.is_set());
                    if flag {
                        requests[s as usize] =
                            Some(SideRequestState::Switch { force_switch: true, no_cancel: false, update: false });
                    }
                }
            }
            _ => {
                for s in 0..2u8 {
                    if self.sides[s as usize].pokemon_left == 0 {
                        continue;
                    }
                    let m = self.active_at(s, 0).expect("a move request needs an active Pokemon");
                    let active = self.get_move_request_data(m);
                    requests[s as usize] = Some(SideRequestState::Move { active, no_cancel: false, update: false });
                }
            }
        }
        let multiple = requests.iter().filter(|r| r.is_some()).count() >= 2;
        let mut out = [SideRequestState::Wait; 2];
        for (s, r) in requests.into_iter().enumerate() {
            out[s] = match r {
                Some(mut r) => {
                    if !self.support_cancel || !multiple {
                        match &mut r {
                            SideRequestState::Move { no_cancel, .. } | SideRequestState::Switch { no_cancel, .. } => {
                                *no_cancel = true
                            }
                            SideRequestState::Wait => {}
                        }
                    }
                    r
                }
                None => SideRequestState::Wait,
            };
        }
        out
    }

    /// PS `pokemon.getLockedMove()`: the `LockMove` answer, `None` for
    /// PS's `null` (the event's `true`).
    // PS: sim/pokemon.ts getLockedMove
    pub fn get_locked_move(&mut self, m: MonRef) -> Option<ActionMove> {
        let r = self.priority_event(EventId::LockMove, Holder::Mon(m), None, EffectId::None, None, false);
        lock_answer(r)
    }

    /// PS `pokemon.getSemiLockedMove(restrictData)`.
    // PS: sim/pokemon.ts getSemiLockedMove
    pub fn get_semi_locked_move(&mut self, m: MonRef, restrict_data: bool) -> Option<ActionMove> {
        if restrict_data && self.mon(m).maybe_locked {
            return None;
        }
        let r = self.priority_event(EventId::SemiLockMove, Holder::Mon(m), None, EffectId::None, None, false);
        lock_answer(r)
    }

    /// PS `pokemon.isLastActive()`.
    // PS: sim/pokemon.ts isLastActive
    pub fn is_last_active(&self, m: MonRef) -> bool {
        let mon = self.mon(m);
        if !mon.is_active {
            return false;
        }
        let side = &self.sides[m.side as usize];
        for i in mon.position as usize + 1..side.active.len() {
            if let Some(slot) = side.active[i] {
                if !side.mons[slot as usize].fainted {
                    return false;
                }
            }
        }
        true
    }

    /// PS `possibleSwitches(side)`: the bench (`side.pokemon` past the
    /// active slots) that has not fainted.
    // PS: sim/battle.ts possibleSwitches
    pub fn possible_switches(&self, side: u8) -> ([MonRef; 6], usize) {
        let mut out = [MonRef::new(side, 0); 6];
        let mut n = 0;
        let s = &self.sides[side as usize];
        if s.pokemon_left == 0 {
            return (out, 0);
        }
        for i in s.active.len()..s.team_size as usize {
            let slot = s.order[i];
            if !s.mons[slot as usize].fainted {
                out[n] = MonRef::new(side, slot);
                n += 1;
            }
        }
        (out, n)
    }

    /// PS `canSwitch(side)`.
    // PS: sim/battle.ts canSwitch
    pub fn can_switch(&self, side: u8) -> usize {
        self.possible_switches(side).1
    }

    /// PS `getRandomSwitchable(side)`: a PRNG `sample` of the bench.
    // PS: sim/battle.ts getRandomSwitchable
    pub fn get_random_switchable(&mut self, side: u8) -> Option<MonRef> {
        let (list, n) = self.possible_switches(side);
        if n == 0 {
            return None;
        }
        Some(*self.prng.sample(&list[..n]))
    }

    /// The `move` / `id` of a request move for a slot's move.
    fn slot_move_name(&mut self, m: MonRef, mv: MoveId) -> RequestMoveName {
        let def = self.dex.move_def(mv);
        let mon = self.mon(m);
        match def.name.as_str() {
            "Hidden Power" => {
                // `Hidden Power ${hpType}` plus the power below gen 6.
                RequestMoveName::HiddenPower { ty: TypeNum(mon.hp_type.index()), power: mon.hp_power }
            }
            // `basePowerCallback(this)`, called directly by PS.
            // PS: data/moves.ts return, frustration
            "Return" => RequestMoveName::WithPower { mv: MoveNum(mv.0), power: power_or_1(u32::from(mon.happiness) * 10 / 25) },
            "Frustration" => RequestMoveName::WithPower {
                mv: MoveNum(mv.0),
                power: power_or_1((255 - u32::from(mon.happiness)) * 10 / 25),
            },
            _ => RequestMoveName::Move(MoveNum(mv.0)),
        }
    }

    /// PS `pokemon.getMoves(lockedMove, restrictData)`.
    // PS: sim/pokemon.ts getMoves
    pub fn get_moves(&mut self, m: MonRef, locked: Option<ActionMove>, restrict_data: bool) -> BoundedVec<RequestMove, MAX_MOVES> {
        let mut moves = BoundedVec::new();
        if let Some(locked) = locked {
            match locked {
                ActionMove::Recharge => {
                    let _ = moves.push(RequestMove::locked(RequestMoveName::Recharge));
                }
                ActionMove::Move(id) => {
                    // A move slot's `move` / `id`, else the dex's (the
                    // "does this happen?" fallback): the same pair here.
                    let _ = moves.push(RequestMove::locked(RequestMoveName::Move(MoveNum(id.0))));
                }
                other => panic!("locked move {other:?} has no request form"),
            }
            return moves;
        }
        let mut has_valid_move = false;
        for i in 0..4 {
            let slot = self.mon(m).move_slots[i];
            if slot.id.is_none() {
                continue;
            }
            let name = self.slot_move_name(m, slot.id);
            let mut target = request_target(self.dex.move_def(slot.id).target);
            if self.dex.move_def(slot.id).name.as_str() == "Curse" {
                let ghost = self.get_types(m).as_slice().contains(&ps_dex::Type::Ghost);
                if !ghost {
                    if let Some(t) = self.dex.move_def(slot.id).non_ghost_target() {
                        target = request_target(t);
                    }
                }
            }
            // `disabled`: false, true or 'hidden'; no Dynamax here.
            let mut disabled = slot.disabled != 0;
            let hidden = slot.disabled == 2;
            if slot.pp == 0 {
                disabled = true;
            } else if hidden {
                disabled = !restrict_data;
            }
            if !disabled {
                has_valid_move = true;
            }
            let _ = moves.push(RequestMove::slot(name, slot.pp, slot.maxpp, target, disabled));
        }
        if has_valid_move {
            moves
        } else {
            BoundedVec::new()
        }
    }

    /// PS `pokemon.getMoveRequestData()`.
    // PS: sim/pokemon.ts getMoveRequestData
    pub fn get_move_request_data(&mut self, m: MonRef) -> ActiveRequest {
        let mut locked = self.get_locked_move(m);
        let hard_locked = locked.is_some();
        if hard_locked {
            self.mon_mut(m).trapped = Trapped::Yes;
        } else {
            locked = if self.mon(m).maybe_locked { None } else { self.get_semi_locked_move(m, true) };
        }
        let is_last_active = self.is_last_active(m);
        let can_switch_in = self.can_switch(m.side) > 0;
        let mut moves = self.get_moves(m, locked, is_last_active);

        let gen1_fight = self.gen == 1 && locked.is_none() && {
            let st = self.mon(m).status;
            let key = if st.is_none() { "" } else { self.dex.condition_def(st).key.as_str() };
            key == "frz" || key == "slp" || (self.has_volatile_key(m, "partiallytrapped") && !self.mon(m).maybe_locked)
        };
        if gen1_fight {
            moves = BoundedVec::new();
            let _ = moves.push(RequestMove::locked(RequestMoveName::Fight));
        } else if moves.is_empty() {
            let struggle = self.dex.move_id("struggle").expect("struggle in the dex");
            let _ = moves.push(RequestMove::struggle(MoveNum(struggle.0)));
        }

        let mut data = ActiveRequest { moves, ..ActiveRequest::default() };
        let mon = self.mon_mut(m);
        if hard_locked || !is_last_active {
            mon.maybe_disabled = false;
            mon.maybe_locked = false;
            mon.maybe_trapped = false;
            if (hard_locked || can_switch_in) && mon.trapped.truthy() {
                data.trapped = true;
            }
        } else {
            mon.maybe_locked = mon.maybe_locked || mon.maybe_disabled;
            if mon.maybe_disabled {
                data.maybe_disabled = true;
            }
            if mon.maybe_locked {
                data.maybe_locked = true;
            }
            if can_switch_in {
                if mon.trapped == Trapped::Yes {
                    data.trapped = true;
                } else if mon.maybe_trapped {
                    data.maybe_trapped = true;
                }
            }
        }
        // No mega, Z, Dynamax or Tera data in gens 1-3.
        data
    }

    /// PS `pokemon.details` (`getUpdatedDetails()` of the base species:
    /// only permanent forme changes rewrite it).
    // PS: sim/pokemon.ts getUpdatedDetails
    pub fn details(&self, m: MonRef) -> Details {
        let mon = self.mon(m);
        let gender = match mon.gender {
            ps_dex::Gender::Male => ps_protocol::Gender::M,
            ps_dex::Gender::Female => ps_protocol::Gender::F,
            _ => ps_protocol::Gender::N,
        };
        Details { species: SpeciesNum(mon.base_species.0), level: mon.level, gender, shiny: mon.shiny, forme: mon.forme }
    }

    /// PS `pokemon.getHealth()`: both forms (the shared one exact under
    /// `reportExactHP`, a percentage under HP Percentage Mod, else pixels).
    // PS: sim/pokemon.ts getHealth
    pub fn health(&self, m: MonRef) -> Health {
        let mon = self.mon(m);
        let status = status_of(self, m);
        let shared = if self.report_exact_hp {
            Hp::new(mon.hp, mon.maxhp)
        } else if self.report_percentages {
            Hp::percent(mon.hp, mon.maxhp)
        } else {
            Hp::pixels(mon.hp, mon.maxhp)
        };
        Health::split(mon.hp, mon.maxhp, shared, status)
    }

    /// PS `pokemon.getSwitchRequestData()`.
    // PS: sim/pokemon.ts getSwitchRequestData
    pub fn get_switch_request_data(&self, m: MonRef) -> SidePokemon {
        let mon = self.mon(m);
        let side = &self.sides[m.side as usize];
        let mut moves = BoundedVec::new();
        for slot in mon.move_slots.iter().filter(|s| !s.id.is_none()) {
            let id = match self.dex.move_def(slot.id).name.as_str() {
                "Hidden Power" => SideMoveId::HiddenPower(TypeNum(mon.hp_type.index())),
                "Return" => SideMoveId::WithPower { mv: MoveNum(slot.id.0), power: power_or_1(u32::from(mon.happiness) * 10 / 25) },
                "Frustration" => SideMoveId::WithPower {
                    mv: MoveNum(slot.id.0),
                    power: power_or_1((255 - u32::from(mon.happiness)) * 10 / 25),
                },
                _ => SideMoveId::Move(MoveNum(slot.id.0)),
            };
            let _ = moves.push(id);
        }
        SidePokemon {
            ident: ps_protocol::PokemonIdent::inactive(side.id(), m.index),
            details: self.details(m),
            condition: HpStatus::exact(mon.hp, mon.maxhp, status_of(self, m)),
            active: (mon.position as usize) < side.active.len(),
            stats: [
                mon.base_stored_stats[0],
                mon.base_stored_stats[1],
                mon.base_stored_stats[2],
                mon.base_stored_stats[3],
                mon.base_stored_stats[4],
            ],
            moves,
            base_ability: (!mon.base_ability.is_none()).then_some(AbilityNum(mon.base_ability.0)),
            item: (!mon.item.is_none()).then_some(ItemNum(mon.item.0)),
            pokeball: mon.pokeball,
        }
    }

    /// PS `side.getRequestData()`.
    // PS: sim/side.ts getRequestData
    pub fn get_request_data(&self, side: u8) -> SideRequest {
        let s = &self.sides[side as usize];
        let mut pokemon = BoundedVec::new();
        for &slot in s.pokemon() {
            let _ = pokemon.push(self.get_switch_request_data(MonRef::new(side, slot)));
        }
        SideRequest { name: s.name, id: s.id(), pokemon }
    }

    /// The request `side` would be sent now (`activeRequest` with its side
    /// data), or `None` for PS's `null`.
    pub fn request(&self, side: u8) -> Option<Request> {
        let st = self.sides[side as usize].active_request?;
        let data = self.get_request_data(side);
        Some(match st {
            SideRequestState::Move { active, no_cancel, update } => Request::Move(MoveRequest {
                active: BoundedVec::from_slice(&[active]).expect("one active slot"),
                side: data,
                no_cancel,
                update,
                rqid: None,
            }),
            SideRequestState::Switch { force_switch, no_cancel, update } => Request::Switch(SwitchRequest {
                force_switch: BoundedVec::from_slice(&[force_switch]).expect("one active slot"),
                side: data,
                no_cancel,
                update,
                rqid: None,
            }),
            SideRequestState::Wait => Request::Wait(WaitRequest { side: data, rqid: None }),
        })
    }
}

/// `basePowerCallback`'s `|| 1`.
fn power_or_1(p: u32) -> u8 {
    if p == 0 {
        1
    } else {
        p as u8
    }
}

/// A `LockMove` / `SemiLockMove` answer: `true` (nobody answered) is
/// `null`; otherwise a move id.
fn lock_answer(r: Relay) -> Option<ActionMove> {
    match r {
        Relay::Bool(true) => None,
        Relay::Move(m) => Some(ActionMove::Move(m)),
        other => panic!("LockMove answered {other:?}"),
    }
}

/// A dex target as the request's `target`.
pub(crate) fn request_target(t: ps_dex::MoveTarget) -> MoveTarget {
    MoveTarget::parse(t.id()).unwrap_or(MoveTarget::Normal)
}
