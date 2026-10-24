//! Choices: PS's `Side.choose` family. A side's [`Choice`] collects the
//! actions of one `>pN` command; `commitChoices` hands them to the queue.
//!
//! The typed [`ps_protocol::Choice`] replaces PS's choice strings: `move N`
//! and `switch N` by index (names, targets and gimmicks are out of scope),
//! `pass`, `default`. `undo` goes to [`Battle::undo_choice`], as
//! `BattleStream` routes it.
//!
//! PS: sim/side.ts `choose`, `chooseMove`, `chooseSwitch`, `choosePass`,
//! `autoChoose`, `getChoiceIndex`, `clearChoice`, `isChoiceDone`,
//! `commitChoices`, `updateDisabledRequest`, `updateRequestForPokemon`,
//! `emitChoiceError`.

use ps_dex::text::{name_is_id, IdBuf};
use ps_dex::MoveId;
use ps_protocol::{Arg, ArgValue, Kw, Msg, MoveNum, SideId, Text, TextArg, TextId, Word};

use crate::queue::{Action, ActionKind, ActionMove};
use crate::request::RequestState;
use crate::state::{Battle, MonRef};

/// Gen 1's `side.lastSelectedMove` / `lastEnemySelectedMove`: a move, or
/// one of the cartridge's placeholder ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectedMove {
    /// `nomove`, the initial value.
    #[default]
    NoMove,
    /// `cannotmove` (partially trapped).
    CannotMove,
    Move(MoveId),
}

impl SelectedMove {
    /// As the move id of a queued move action.
    pub fn action_move(self) -> ActionMove {
        match self {
            SelectedMove::NoMove => ActionMove::NoMove,
            SelectedMove::CannotMove => ActionMove::CannotMove,
            SelectedMove::Move(m) => ActionMove::Move(m),
        }
    }
}

/// One chosen action: the queue action it becomes, and `moveSlot` (the
/// chosen move's index, `None` through the Fight button or a lock).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChosenAction {
    pub action: Action,
    pub move_slot: Option<u8>,
}

/// Most actions one singles choice holds (an auto-pass and an action).
pub const CHOICE_ACTIONS_CAP: usize = 2;

/// PS's `side.choice`, without the gimmick flags of later gens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub cant_undo: bool,
    /// `error !== ''`.
    pub error: bool,
    pub len: u8,
    pub actions: [Option<ChosenAction>; CHOICE_ACTIONS_CAP],
    pub forced_switches_left: u8,
    pub forced_passes_left: u8,
    /// `switchIns`: a bit per `side.pokemon` index.
    pub switch_ins: u8,
}

impl Choice {
    pub const EMPTY: Choice = Choice {
        cant_undo: false,
        error: false,
        len: 0,
        actions: [None; CHOICE_ACTIONS_CAP],
        forced_switches_left: 0,
        forced_passes_left: 0,
        switch_ins: 0,
    };

    pub fn actions(&self) -> impl Iterator<Item = &ChosenAction> {
        self.actions[..self.len as usize].iter().flatten()
    }

    fn push(&mut self, a: ChosenAction) {
        let n = self.len as usize;
        assert!(n < CHOICE_ACTIONS_CAP, "more than {CHOICE_ACTIONS_CAP} chosen actions");
        self.actions[n] = Some(a);
        self.len += 1;
    }
}

impl Default for Choice {
    fn default() -> Self {
        Choice::EMPTY
    }
}

fn side_id(side: u8) -> SideId {
    if side == 0 {
        SideId::P1
    } else {
        SideId::P2
    }
}

impl Battle {
    /// PS `side.requestState`.
    pub fn side_request_state(&self, side: u8) -> RequestState {
        self.sides[side as usize].active_request.map_or(RequestState::None, |r| r.state())
    }

    /// The Pokemon in `side`'s active position `i` (PS `side.active[i]`).
    fn active_mon(&self, side: u8, i: usize) -> MonRef {
        self.active_at(side, i as u8).expect("empty active slot")
    }

    /// PS `side.emitChoiceError(message, update)`. `update` is the
    /// `updateRequestForPokemon` callback (it returns whether the request
    /// changed). Returns `false`.
    // PS: sim/side.ts emitChoiceError
    pub fn emit_choice_error(
        &mut self,
        side: u8,
        text: Text,
        update: Option<(MonRef, fn(&mut Battle, MonRef) -> bool)>,
    ) -> bool {
        self.sides[side as usize].choice.error = true;
        let updated = match update {
            Some((m, f)) => self.update_request_for_pokemon(m, f),
            None => false,
        };
        self.outbox.push_side(side_id(side), Msg::Error { side: side_id(side), unavailable: updated, text });
        if updated {
            self.emit_request(side, true);
        }
        false
    }

    /// PS `side.updateRequestForPokemon(pokemon, update)`: applies `update`
    /// to the stored request's entry for `pokemon` (`?? true`).
    // PS: sim/side.ts updateRequestForPokemon
    pub fn update_request_for_pokemon(&mut self, m: MonRef, update: fn(&mut Battle, MonRef) -> bool) -> bool {
        let is_move = matches!(self.sides[m.side as usize].active_request, Some(crate::request::SideRequestState::Move { .. }));
        assert!(is_move, "Can't update a request without active Pokemon");
        update(self, m)
    }

    /// PS `side.updateDisabledRequest(pokemon, req)`, on the stored request.
    /// Singles: only `maybeLocked` can change (the `maybeDisabled` branch is
    /// doubles-only, and no gen 1-3 request has a gimmick to switch off).
    // PS: sim/side.ts updateDisabledRequest
    pub fn update_disabled_request(&mut self, m: MonRef) -> bool {
        let mut updated = false;
        if self.mon(m).maybe_locked {
            self.mon_mut(m).maybe_locked = false;
            if let Some(req) = self.stored_active_request_mut(m.side) {
                req.maybe_locked = false;
            }
            updated = true;
        }
        updated
    }

    /// PS `side.clearChoice()`.
    // PS: sim/side.ts clearChoice
    pub fn clear_choice(&mut self, side: u8) {
        let mut forced_switches = 0;
        let mut forced_passes = 0;
        if self.request_state == RequestState::Switch {
            let s = &self.sides[side as usize];
            let can_switch_out = (0..s.active.len())
                .filter(|&i| s.active[i].is_some_and(|slot| s.mons[slot as usize].switch_flag.is_set()))
                .count() as u8;
            let can_switch_in = self.can_switch(side) as u8;
            forced_switches = can_switch_out.min(can_switch_in);
            forced_passes = can_switch_out - forced_switches;
        }
        // Only relevant for gen 3: the queued switches' targets.
        let mut switch_ins = 0u8;
        for a in self.queue.get_switches(Some(side)) {
            if let Some(t) = a.target {
                switch_ins |= 1 << self.mon(t).position;
            }
        }
        self.sides[side as usize].choice =
            Choice { forced_switches_left: forced_switches, forced_passes_left: forced_passes, switch_ins, ..Choice::EMPTY };
    }

    /// PS `side.isChoiceDone()`.
    // PS: sim/side.ts isChoiceDone
    pub fn is_choice_done(&mut self, side: u8) -> bool {
        if self.side_request_state(side) == RequestState::None {
            return true;
        }
        if self.sides[side as usize].choice.forced_switches_left != 0 {
            return false;
        }
        self.get_choice_index(side, false);
        self.sides[side as usize].choice.len as usize >= self.sides[side as usize].active.len()
    }

    /// PS `side.getChoiceIndex(isPass)`, with its auto-pass.
    // PS: sim/side.ts getChoiceIndex
    pub fn get_choice_index(&mut self, side: u8, is_pass: bool) -> usize {
        let mut index = self.sides[side as usize].choice.len as usize;
        let n_active = self.sides[side as usize].active.len();
        if !is_pass {
            match self.side_request_state(side) {
                RequestState::Move => {
                    // No Commanding in gens 1-3.
                    while index < n_active && self.mon(self.active_mon(side, index)).fainted {
                        self.choose_pass(side);
                        index += 1;
                    }
                }
                RequestState::Switch => {
                    while index < n_active && !self.mon(self.active_mon(side, index)).switch_flag.is_set() {
                        self.choose_pass(side);
                        index += 1;
                    }
                }
                _ => {}
            }
        }
        index
    }

    /// PS `side.choose(input)` for one typed choice.
    // PS: sim/side.ts choose
    pub fn side_choose(&mut self, side: u8, choice: ps_protocol::Choice) -> bool {
        if self.side_request_state(side) == RequestState::None {
            let id = if self.ended { TextId::CantDoGameOver } else { TextId::CantDoNotYourTurn };
            return self.emit_choice_error(side, Text::new(id), None);
        }
        if self.sides[side as usize].choice.cant_undo {
            return self.emit_choice_error(side, Text::new(TextId::CantUndoLeak), None);
        }
        self.clear_choice(side);
        // One choice string, so never more than `active.length`.
        match choice {
            ps_protocol::Choice::Move(n) => {
                if !self.choose_move(side, Some(n)) {
                    return false;
                }
            }
            ps_protocol::Choice::MoveName(name) => {
                // PS's `chooseMove` answers a wrong request state itself.
                let rs = self.side_request_state(side);
                if rs != RequestState::Move {
                    let t = Text::with1(TextId::CantMoveNeedResponse, rs.word());
                    return self.emit_choice_error(side, t, None);
                }
                match self.resolve_move_name(side, name.as_str()) {
                    Some(n) => {
                        if !self.choose_move(side, Some(n)) {
                            return false;
                        }
                    }
                    None => todo!(
                        "unported: PS's chooseMove by name (no move matching {:?})",
                        name.as_str()
                    ),
                }
            }
            ps_protocol::Choice::Switch(n) => {
                if !self.choose_switch(side, Some(n)) {
                    return false;
                }
            }
            ps_protocol::Choice::SwitchName(name) => {
                // PS's `chooseSwitch` answers a wrong request state itself.
                let rs = self.side_request_state(side);
                if rs != RequestState::Switch {
                    let t = Text::with1(TextId::CantSwitchNeedResponse, rs.word());
                    return self.emit_choice_error(side, t, None);
                }
                match self.resolve_switch_name(side, name.as_str()) {
                    Some(n) => {
                        if !self.choose_switch(side, Some(n)) {
                            return false;
                        }
                    }
                    None => todo!(
                        "unported: PS's chooseSwitch by name (no Pokemon named {:?})",
                        name.as_str()
                    ),
                }
            }
            ps_protocol::Choice::Pass => {
                if !self.choose_pass(side) {
                    return false;
                }
            }
            ps_protocol::Choice::Default => {
                if !self.auto_choose(side) {
                    return false;
                }
            }
            ps_protocol::Choice::Undo => panic!("undo is not a choice: route it to undo_choice"),
        }
        !self.sides[side as usize].choice.error
    }

    /// PS `chooseMove`'s name branch: the move's id is `toID(moveText)`
    /// and the request's move with that id is the choice. `hiddenpowerXX`
    /// counts as `hiddenpower`. Returns the 1-based slot.
    // PS: sim/side.ts chooseMove (the string branch); sim/dex.ts toID.
    fn resolve_move_name(&mut self, side: u8, name: &str) -> Option<u8> {
        if self.side_request_state(side) != RequestState::Move {
            return None;
        }
        let index = self.get_choice_index(side, false);
        if index >= self.sides[side as usize].active.len() {
            return None;
        }
        let pokemon = self.active_mon(side, index);
        let request = self.get_move_request_data(pokemon);
        let id = IdBuf::from_name(name);
        let moveid = id.as_str();
        // PS: `if (moveid.startsWith("hiddenpower")) moveid = "hiddenpower";`
        let hidden_power = moveid.starts_with("hiddenpower");
        for (i, m) in request.moves.iter().enumerate() {
            let matched = match m.name {
                ps_protocol::RequestMoveName::Move(mv)
                | ps_protocol::RequestMoveName::WithPower { mv, .. } => {
                    name_is_id(self.dex.move_def(MoveId(mv.0)).name.as_str(), moveid)
                }
                ps_protocol::RequestMoveName::HiddenPower { .. } => hidden_power,
                ps_protocol::RequestMoveName::Recharge => moveid == "recharge",
                ps_protocol::RequestMoveName::Fight => moveid == "fight",
            };
            if matched {
                return Some((i + 1) as u8);
            }
        }
        None
    }

    /// PS `chooseSwitch`'s name branch: the Pokemon whose nickname matches
    /// the text (case-insensitively) or whose species id does. Returns the
    /// 1-based slot.
    // PS: sim/side.ts chooseSwitch (the name branch); sim/dex.ts toID.
    fn resolve_switch_name(&self, side: u8, name: &str) -> Option<u8> {
        let s = &self.sides[side as usize];
        let id = IdBuf::from_name(name);
        let lower = name.to_lowercase();
        for (i, &slot) in s.order.iter().enumerate().take(s.team_size as usize) {
            let mon = &s.mons[slot as usize];
            // PS: `slotText.toLowerCase() === mon.name.toLowerCase()`.
            if mon.name.as_str().to_lowercase() == lower {
                return Some((i + 1) as u8);
            }
            // PS: `toID(slotText) === mon.species.id`.
            if name_is_id(self.dex.species_def(mon.species).name.as_str(), id.as_str()) {
                return Some((i + 1) as u8);
            }
        }
        None
    }

    /// PS `side.chooseMove(moveText)`: `None` is `autoChoose`'s call with
    /// no move; `Some(n)` the 1-based `move n`.
    // PS: sim/side.ts chooseMove
    pub fn choose_move(&mut self, side: u8, move_text: Option<u8>) -> bool {
        let rs = self.side_request_state(side);
        if rs != RequestState::Move {
            let t = Text::with1(TextId::CantMoveNeedResponse, rs.word());
            return self.emit_choice_error(side, t, None);
        }
        let index = self.get_choice_index(side, false);
        if index >= self.sides[side as usize].active.len() {
            return self.emit_choice_error(side, Text::new(TextId::CantMoveTooMany), None);
        }
        let auto_choose = move_text.is_none();
        let pokemon = self.active_mon(side, index);

        let request = self.get_move_request_data(pokemon);
        let move_index = move_text.map_or(0, |n| i32::from(n) - 1);
        if move_index < 0 || move_index as usize >= request.moves.len() {
            let t = Text::with2(
                TextId::CantMoveNoMoveIndex,
                TextArg::nickname(self.ident(pokemon)),
                TextArg::int(move_index + 1),
            );
            return self.emit_choice_error(side, t, None);
        }
        let mut move_slot = move_index as usize;
        let mut moveid = request.moves[move_slot].name;
        let mut target_type = request.moves[move_slot].target;

        let moves = self.get_moves(pokemon, None, false);
        if auto_choose {
            for (i, m) in request.moves.iter().enumerate() {
                if m.disabled == Some(true) {
                    continue;
                }
                if i < moves.len() && m.name == moves[i].name && moves[i].disabled == Some(true) {
                    continue;
                }
                moveid = m.name;
                move_slot = i;
                target_type = m.target;
                break;
            }
        }
        let mv = self.request_move_id(moveid);

        // Targets: none in singles (`targetLoc` is always 0 here), and an
        // auto-chosen move never takes one.
        let _ = target_type;

        let locked = match self.get_locked_move(pokemon) {
            Some(l) => Some(l),
            None => self.get_semi_locked_move(pokemon, false),
        };
        if let Some(locked) = locked {
            let mon = self.mon(pokemon);
            let target_loc = mon.last_move_target_loc;
            // `pokemon.volatiles[lockedMoveID]?.targetLoc`: no gen 1-3
            // lock stores one.
            if mon.maybe_locked {
                self.sides[side as usize].choice.cant_undo = true;
            }
            let mut a = Action::new(ActionKind::Move).with_pokemon(pokemon);
            a.target_loc = target_loc;
            a.move_ = locked;
            self.sides[side as usize].choice.push(crate::choice::ChosenAction { action: a, move_slot: None });
            return true;
        } else if self.gen == 1 && {
            let mon = self.mon(pokemon);
            let st = mon.status;
            let key = if st.is_none() { "" } else { self.dex.condition_def(st).key.as_str() };
            key == "frz" || key == "slp" || self.has_volatile_key(pokemon, "partiallytrapped")
        } {
            if self.mon(pokemon).maybe_locked {
                self.sides[side as usize].choice.cant_undo = true;
            }
            let mut a = Action::new(ActionKind::Move).with_pokemon(pokemon);
            a.move_ = ActionMove::Fight;
            self.sides[side as usize].choice.push(ChosenAction { action: a, move_slot: None });
            return true;
        } else if moves.is_empty() {
            if self.gen <= 4 {
                let struggle = self.dex.move_id("struggle").expect("struggle in the dex");
                let mon = self.ident(pokemon);
                let effect = Arg::new(ArgValue::MoveEffect(MoveNum(struggle.0)));
                self.outbox.push_side(side_id(side), Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
            }
            if self.mon(pokemon).maybe_locked {
                self.sides[side as usize].choice.cant_undo = true;
            }
            let struggle = self.dex.move_id("struggle").expect("struggle in the dex");
            let mut a = Action::new(ActionKind::Move).with_pokemon(pokemon);
            a.move_ = ActionMove::Move(struggle);
            self.sides[side as usize].choice.push(ChosenAction { action: a, move_slot: None });
            return true;
        } else if self.gen == 1 {
            // Gen 1 tracks disabled moves by slot.
            if moves[move_slot].disabled == Some(true) {
                let t = Text::with2(
                    TextId::CantMoveDisabled,
                    TextArg::nickname(self.ident(pokemon)),
                    TextArg::mv(MoveNum(mv.id().map_or(0, |m| m.0))),
                );
                return self.emit_choice_error(side, t, None);
            }
        } else {
            let mut is_enabled = false;
            for m in moves.iter() {
                if m.name != moveid {
                    continue;
                }
                if m.disabled != Some(true) {
                    is_enabled = true;
                    break;
                }
                // `disabledSource` is not tracked (no ported effect sets it).
            }
            if !is_enabled {
                assert!(!auto_choose, "autoChoose chose a disabled move");
                let t = Text::with2(
                    TextId::CantMoveDisabled,
                    TextArg::nickname(self.ident(pokemon)),
                    TextArg::mv(MoveNum(mv.id().map_or(0, |m| m.0))),
                );
                fn update(b: &mut Battle, m: MonRef) -> bool {
                    // Marks the move disabled in the request; the chosen
                    // move is the one `choose_move` was looking at, which
                    // is already shown disabled (PS only flips
                    // `m.disabled` when it is not).
                    b.update_disabled_request(m)
                }
                return self.emit_choice_error(side, t, Some((pokemon, update)));
            }
        }

        let mut a = Action::new(ActionKind::Move).with_pokemon(pokemon);
        a.target_loc = 0;
        a.move_ = mv;
        self.sides[side as usize].choice.push(ChosenAction { action: a, move_slot: Some(move_slot as u8) });
        if self.mon(pokemon).maybe_disabled {
            // Singles.
            self.sides[side as usize].choice.cant_undo = true;
        }
        true
    }

    /// The action move of a request move's `id` (`recharge`, `fight`,
    /// `struggle` or a move slot's).
    fn request_move_id(&self, name: ps_protocol::RequestMoveName) -> ActionMove {
        use ps_protocol::RequestMoveName as N;
        match name {
            N::Move(m) | N::WithPower { mv: m, .. } => ActionMove::Move(MoveId(m.0)),
            N::HiddenPower { .. } => ActionMove::Move(self.dex.move_id("hiddenpower").expect("hiddenpower in the dex")),
            N::Recharge => ActionMove::Recharge,
            N::Fight => ActionMove::Fight,
        }
    }

    /// `pokemon.volatiles[key]` by condition key.
    pub fn has_volatile_key(&self, m: MonRef, key: &str) -> bool {
        self.dex
            .condition_id(key)
            .is_some_and(|c| self.mon(m).volatiles.contains(crate::effect::EffectId::Condition(c)))
    }

    /// PS `side.chooseSwitch(slotText)`: `Some(n)` is `switch n`
    /// (1-based), `None` the auto-choice.
    // PS: sim/side.ts chooseSwitch
    pub fn choose_switch(&mut self, side: u8, slot_text: Option<u8>) -> bool {
        let rs = self.side_request_state(side);
        if rs != RequestState::Move && rs != RequestState::Switch {
            let t = Text::with1(TextId::CantSwitchNeedResponse, rs.word());
            return self.emit_choice_error(side, t, None);
        }
        let index = self.get_choice_index(side, false);
        let n_active = self.sides[side as usize].active.len();
        if index >= n_active {
            if rs == RequestState::Switch {
                return self.emit_choice_error(side, Text::new(TextId::CantSwitchTooManySwitches), None);
            }
            return self.emit_choice_error(side, Text::new(TextId::CantSwitchTooMany), None);
        }
        let pokemon = self.active_mon(side, index);
        let slot: usize = match slot_text {
            None => {
                if rs != RequestState::Switch {
                    return self.emit_choice_error(side, Text::new(TextId::CantSwitchSelect), None);
                }
                // No Revival Blessing in gens 1-3.
                if self.sides[side as usize].choice.forced_switches_left == 0 {
                    return self.choose_pass(side);
                }
                let s = &self.sides[side as usize];
                let mut slot = n_active;
                while s.choice.switch_ins & (1 << slot) != 0 || s.mons[s.order[slot] as usize].fainted {
                    slot += 1;
                }
                slot
            }
            Some(0) => {
                // `parseInt('0') - 1 < 0`: a name lookup, which a number
                // never matches.
                let t = Text::with1(TextId::CantSwitchNoName, TextArg::int(0));
                return self.emit_choice_error(side, t, None);
            }
            Some(n) => n as usize - 1,
        };
        let s = &self.sides[side as usize];
        if slot >= s.team_size as usize {
            let t = Text::with1(TextId::CantSwitchNoSlot, TextArg::int(slot as i32 + 1));
            return self.emit_choice_error(side, t, None);
        } else if slot < n_active {
            return self.emit_choice_error(side, Text::new(TextId::CantSwitchActive), None);
        } else if s.choice.switch_ins & (1 << slot) != 0 {
            let t = Text::with1(TextId::CantSwitchOnce, TextArg::int(slot as i32 + 1));
            return self.emit_choice_error(side, t, None);
        }
        let target = MonRef::new(side, s.order[slot]);
        if self.mon(target).fainted {
            return self.emit_choice_error(side, Text::new(TextId::CantSwitchFainted), None);
        }
        if rs == RequestState::Move {
            if self.mon(pokemon).trapped.truthy() {
                fn update(b: &mut Battle, m: MonRef) -> bool {
                    let mut updated = false;
                    if let Some(req) = b.stored_active_request_mut(m.side) {
                        if req.maybe_trapped {
                            req.maybe_trapped = false;
                            updated = true;
                        }
                        if !req.trapped {
                            req.trapped = true;
                            updated = true;
                        }
                    }
                    updated
                }
                return self.emit_choice_error(side, Text::new(TextId::CantSwitchTrapped), Some((pokemon, update)));
            } else if self.mon(pokemon).maybe_trapped {
                self.sides[side as usize].choice.cant_undo = true;
            }
        } else if rs == RequestState::Switch {
            let c = &mut self.sides[side as usize].choice;
            assert!(c.forced_switches_left != 0, "Player somehow switched too many Pokemon");
            c.forced_switches_left -= 1;
        }
        self.sides[side as usize].choice.switch_ins |= 1 << slot;
        let kind = if rs == RequestState::Switch { ActionKind::InstaSwitch } else { ActionKind::Switch };
        let mut a = Action::new(kind).with_pokemon(pokemon);
        a.target = Some(target);
        self.sides[side as usize].choice.push(ChosenAction { action: a, move_slot: None });
        true
    }

    /// PS `side.choosePass()`.
    // PS: sim/side.ts choosePass
    pub fn choose_pass(&mut self, side: u8) -> bool {
        let index = self.get_choice_index(side, true);
        if index >= self.sides[side as usize].active.len() {
            return false;
        }
        let pokemon = self.active_mon(side, index);
        match self.side_request_state(side) {
            RequestState::Switch => {
                if self.mon(pokemon).switch_flag.is_set() {
                    if self.sides[side as usize].choice.forced_passes_left == 0 {
                        let t = Text::with1(TextId::CantPassReplace, TextArg::nickname(self.ident(pokemon)));
                        return self.emit_choice_error(side, t, None);
                    }
                    self.sides[side as usize].choice.forced_passes_left -= 1;
                }
            }
            RequestState::Move => {
                if !self.mon(pokemon).fainted {
                    let t = Text::with1(TextId::CantPassMustMove, TextArg::nickname(self.ident(pokemon)));
                    return self.emit_choice_error(side, t, None);
                }
            }
            _ => return self.emit_choice_error(side, Text::new(TextId::CantPassNotMoveOrSwitch), None),
        }
        self.sides[side as usize].choice.push(ChosenAction { action: Action::new(ActionKind::Pass), move_slot: None });
        true
    }

    /// PS `side.autoChoose()`.
    // PS: sim/side.ts autoChoose
    pub fn auto_choose(&mut self, side: u8) -> bool {
        match self.side_request_state(side) {
            RequestState::Switch => {
                let mut i = 0;
                while !self.is_choice_done(side) {
                    assert!(self.choose_switch(side, None), "autoChoose switch crashed");
                    i += 1;
                    assert!(i <= 10, "autoChoose failed: infinite looping");
                }
            }
            RequestState::Move => {
                let mut i = 0;
                while !self.is_choice_done(side) {
                    assert!(self.choose_move(side, None), "autoChoose crashed");
                    i += 1;
                    assert!(i <= 10, "autoChoose failed: infinite looping");
                }
            }
            _ => {}
        }
        true
    }

    /// PS `side.commitChoices()`: gen 1's selected-move bookkeeping, then
    /// the actions go to the queue.
    // PS: sim/side.ts commitChoices
    pub fn side_commit_choices(&mut self, side: u8) {
        let choice = self.sides[side as usize].choice;
        let mut actions = [None; CHOICE_ACTIONS_CAP];
        for (i, c) in choice.actions().enumerate() {
            actions[i] = Some(*c);
        }
        if self.gen == 1 {
            for c in actions.iter_mut().flatten() {
                if c.action.choice != ActionKind::Move {
                    continue;
                }
                let Some(pokemon) = c.action.pokemon else { continue };
                let s = side as usize;
                match c.action.move_ {
                    ActionMove::Fight => {
                        let st = self.mon(pokemon).status;
                        let key = if st.is_none() { "" } else { self.dex.condition_def(st).key.as_str() };
                        if key == "frz" || key == "slp" {
                            let slot = self.sides[s].last_selected_move_slot;
                            let ms = self.get_move_slot(pokemon, slot).expect("moveSlot is null which shouldn't happen");
                            self.sides[s].last_enemy_selected_move = SelectedMove::Move(ms);
                        } else if self.has_volatile_key(pokemon, "partiallytrapped") {
                            self.sides[s].last_selected_move = SelectedMove::CannotMove;
                            self.sides[s].last_enemy_selected_move = SelectedMove::CannotMove;
                        }
                    }
                    ActionMove::Move(m) if self.dex.move_def(m).name.as_str() == "Struggle" => {
                        self.sides[s].last_selected_move = SelectedMove::Move(m);
                        self.sides[s].last_enemy_selected_move = SelectedMove::Move(m);
                    }
                    mv => {
                        if let Some(slot) = c.move_slot {
                            let id = mv.id().expect("a move slot's move");
                            self.sides[s].last_selected_move = SelectedMove::Move(id);
                            self.sides[s].last_selected_move_slot = slot;
                            self.sides[s].last_move = id;
                        }
                        let slot = self.sides[s].last_selected_move_slot;
                        let ms = self.get_move_slot(pokemon, slot).expect("moveSlot is null which shouldn't happen");
                        self.sides[s].last_enemy_selected_move = SelectedMove::Move(ms);
                    }
                }
                c.action.move_ = self.sides[s].last_selected_move.action_move();
            }
        }
        for c in actions.iter().flatten() {
            self.add_choice(c.action);
        }
    }

    /// PS `pokemon.getMoveSlot(slotIndex)` (gen 1): the slot's move id.
    // PS: sim/pokemon.ts getMoveSlot
    pub fn get_move_slot(&self, m: MonRef, slot: u8) -> Option<MoveId> {
        let ms = self.mon(m).move_slots.get(slot as usize)?;
        (!ms.id.is_none()).then_some(ms.id)
    }
}

impl RequestState {
    /// The `requestState` word in "You need a ... response".
    pub fn word(self) -> TextArg {
        match self {
            RequestState::Move => TextArg::word(Word::Move),
            RequestState::Switch => TextArg::word(Word::Switch),
            RequestState::None => TextArg::NONE,
        }
    }
}
