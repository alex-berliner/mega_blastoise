//! The ONE battle-input state machine, shared verbatim by the RP2040 firmware
//! and the web client.
//!
//! Everything a player experiences while a choice is being collected is
//! decided here: prompt display, accept/reject validation and messages, the
//! waiting screen, tap-or-type-to-unready, the both-ready grace window,
//! invalid-selection flashes, and long-press detail views. Platforms own ONLY
//! raw IO — classifying physical presses into taps/holds, reading typed
//! lines, rendering [`OledCmd`]s, printing [`Effect`] lines, and calling
//! [`ChoiceCollector::tick`] with a monotonic clock.
//!
//! DO NOT implement any of these behaviors platform-side. If the pico and the
//! web ever feel different during input collection, the bug is that logic
//! leaked out of this module.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use gen1_battle::Request;

use crate::battle_input::{
    format_move_choice, format_switch_choice, turn_action_choice, ActionReject, ActivePrompt,
    PlayerAction,
};
use crate::board_event::player_id_to_num;
use crate::cli_parse::{parse_switch_line, parse_turn_line, TurnChoice};
use crate::display::{party_slot_from_mon, PartySlotData};
use crate::oled_ctl::OledCmd;
use crate::prompt_fmt::format_prompt;

/// After every player has committed, either may still unready for this long
/// before the choices become final.
pub const UNREADY_GRACE_MS: u64 = 1000;
/// How long the invalid-selection screen shows before restoring.
pub const INVALID_FLASH_MS: u64 = 600;
/// Recommended platform cadence for [`ChoiceCollector::tick`].
pub const COLLECT_TICK_MS: u64 = 50;
/// Press-and-hold threshold platforms use to classify a press as a hold
/// (detail view) instead of a tap (selection).
pub const HOLD_THRESHOLD_MS: u64 = 500;
/// While a party-stats view is held, its two pages alternate at this cadence.
pub const STATS_PAGE_CYCLE_MS: u64 = 2000;

/// Help lines for in-battle typed input — shared by every platform's help
/// output, and kept next to [`ChoiceCollector::typed_line`] so the two can't
/// drift.
pub const BATTLE_HELP: &[&str] = &[
    "p1 2 / p2 s3      choose a move / switch for a player",
    "2 / s3            bare form, when only one player is choosing",
    "switch N          in-turn switch by team slot (sN also works)",
    "(typing anything for a committed player unreadies them)",
    ":press pN <1-4|s1-s3>    simulate a button tap",
    ":hold pN <1-4|s1-s3>     simulate a long press down",
    ":release pN              release the held button",
];

/// A physical-input event, already classified by the platform's raw layer.
/// `player` is 1 or 2; indices are 0-based.
#[derive(Clone, Copy, Debug)]
pub enum PadEvent {
    /// Short press on a move button.
    TapMove { player: u8, slot: u8 },
    /// Short press on a party button.
    TapSwitch { player: u8, idx: u8 },
    /// A move button crossed the hold threshold (still held).
    HoldMove { player: u8, slot: u8 },
    /// A party button crossed the hold threshold (still held).
    HoldSwitch { player: u8, idx: u8 },
    /// The held button was released.
    HoldEnd { player: u8 },
}

/// What the collector wants the platform to do. Platforms map these onto
/// their IO verbatim — no filtering, no additions.
pub enum Effect {
    /// Send to the display pipeline (fw OLED channel / web canvas).
    Oled(OledCmd),
    /// Acknowledgement line ("[OK]  …" framing).
    Ok(String),
    /// Rejection line ("[!!]  …" framing).
    Err(String),
    /// Debug/informational line ("[>>]  …" framing).
    Dbg(String),
    /// Plain prompt text (multi-line, `\n`-separated).
    Text(String),
}

/// One player's distilled options for a single decision point.
pub struct SlotOptions {
    player_num: u8,
    player_id: String,
    n_moves: usize,
    usable: [bool; 4],
    trapped: bool,
    active_slot: Option<usize>,
    /// Per-team-slot switch validity (alive and benched). All-true when no
    /// player data was attached — the engine still validates.
    party_ok: [bool; 6],
    forced_switch: bool,
    /// Forced choice (locked move / no moves / team preview) or an AI pick —
    /// committed from the start, can't be unreadied.
    auto: Option<String>,
    /// The AI controls this player (affects messages only).
    is_ai: bool,
    /// Prompt text to print for human players ("" otherwise).
    prompt_text: String,
    /// Party snapshot for the stats/switch screens.
    party: Vec<PartySlotData>,
}

impl SlotOptions {
    /// Distil a prompt into the options the collector needs.
    pub fn from_prompt(prompt: &ActivePrompt) -> Self {
        let player_id = prompt.player_id.clone();
        let request = &prompt.request;
        let player_data = prompt.player_data.as_ref();
        let mut s = Self {
            player_num: player_id_to_num(&player_id),
            player_id,
            n_moves: 0,
            usable: [false; 4],
            trapped: false,
            active_slot: None,
            party_ok: [true; 6],
            forced_switch: false,
            auto: None,
            is_ai: false,
            prompt_text: String::new(),
            party: Vec::new(),
        };
        if let Some(pd) = player_data {
            for (i, ok) in s.party_ok.iter_mut().enumerate() {
                *ok = pd.mons.get(i).is_some_and(|m| !m.active && m.hp > 0);
            }
            s.party = pd.mons.iter().map(party_slot_from_mon).collect();
        }
        match request {
            Request::Turn(turn) => {
                if let Some(mon) = turn.active.first() {
                    s.active_slot = Some(mon.team_position as usize);
                    let n = mon.moves.len().min(4);
                    if n == 0 {
                        s.auto = Some(String::from("pass"));
                    } else if mon.locked_into_move {
                        s.auto = Some(format_move_choice(0));
                    } else {
                        s.n_moves = n;
                        for i in 0..n {
                            s.usable[i] = !mon.moves[i].disabled && mon.moves[i].pp > 0;
                        }
                        s.trapped = mon.trapped;
                    }
                }
            }
            Request::Switch(_) => s.forced_switch = true,
            Request::TeamPreview(_) => s.auto = Some(String::from("random")),
            Request::LearnMove(_) => s.auto = Some(String::from("pass")),
        }
        s.prompt_text = format_prompt(s.player_id.as_str(), request, player_data);
        s
    }

    /// Mark this player as AI-controlled with a pre-made choice.
    pub fn set_ai_choice(&mut self, choice: String) {
        self.auto = Some(choice);
        self.is_ai = true;
    }

    /// Placeholder slot for a single-prompt batch: permanently committed with
    /// an empty choice, never unreadies, contributes no submission.
    fn inert() -> Self {
        Self {
            player_num: 0,
            player_id: String::new(),
            n_moves: 0,
            usable: [false; 4],
            trapped: false,
            active_slot: None,
            party_ok: [false; 6],
            forced_switch: false,
            auto: Some(String::new()),
            is_ai: false,
            prompt_text: String::new(),
            party: Vec::new(),
        }
    }

    /// The screen this player returns to while still choosing.
    fn pick_screen(&self) -> OledCmd {
        if self.forced_switch {
            OledCmd::ShowSwitchScreen { player: self.player_num }
        } else {
            OledCmd::RestoreScreen { player: self.player_num }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SlotState {
    /// Waiting for a selection.
    Choosing,
    /// A long-press move-detail view is up; restores on HoldEnd.
    Detail,
    /// A long-press party-stats view is up; its two pages alternate every
    /// [`STATS_PAGE_CYCLE_MS`] until HoldEnd.
    Stats { team_idx: u8, page: u8, next_flip: u64 },
    /// Invalid-selection screen is up; restores at `until`.
    Invalid { until: u64 },
    /// Choice locked in (waiting screen shown, unless auto).
    Committed,
}

/// The shared choice-collection state machine. Construct once per prompt
/// batch; feed it pad events, typed lines, and ticks; read the choices out
/// when [`ChoiceCollector::tick`] reports completion.
pub struct ChoiceCollector {
    slots: [SlotOptions; 2],
    st: [SlotState; 2],
    out: [String; 2],
    n_real: usize,
    grace_start: Option<u64>,
    complete: bool,
}

impl ChoiceCollector {
    /// Build from the prompt batch (1 or 2 prompts). Emits the initial
    /// effects: party snapshots, pick screens, waiting-for-opponent screens,
    /// prompt text for humans, and AI announcements.
    pub fn new(mut batch: Vec<SlotOptions>, fx: &mut Vec<Effect>) -> Self {
        let n_real = batch.len().min(2);
        batch.truncate(2);
        while batch.len() < 2 {
            batch.push(SlotOptions::inert());
        }
        let slots: [SlotOptions; 2] = match batch.try_into() {
            Ok(a) => a,
            Err(_) => unreachable!("padded to exactly 2"),
        };

        let mut st = [SlotState::Choosing; 2];
        let mut out = [String::new(), String::new()];
        for i in 0..2 {
            let s = &slots[i];
            if i < n_real {
                if !s.party.is_empty() {
                    fx.push(Effect::Oled(OledCmd::PartyUpdate {
                        player: s.player_num,
                        slots: s.party.clone(),
                    }));
                }
                if s.is_ai {
                    fx.push(Effect::Dbg(format!("[AI] auto-choosing for {}", s.player_id)));
                } else {
                    if s.forced_switch {
                        fx.push(Effect::Oled(OledCmd::ShowSwitchScreen { player: s.player_num }));
                    }
                    fx.push(Effect::Text(s.prompt_text.clone()));
                }
            }
            if let Some(c) = &s.auto {
                out[i] = c.clone();
                st[i] = SlotState::Committed;
            }
        }
        // A player with no prompt this batch is waiting on the other (e.g. a
        // forced switch after a faint).
        for player in [1u8, 2] {
            let covered = slots[..n_real].iter().any(|s| s.player_num == player);
            if !covered {
                fx.push(Effect::Oled(OledCmd::ShowWaitingForOpponent { player }));
            }
        }

        Self { slots, st, out, n_real, grace_start: None, complete: false }
    }

    fn slot_index(&self, player_num: u8) -> Option<usize> {
        (0..self.n_real).find(|&i| self.slots[i].player_num == player_num)
    }

    fn is_auto(&self, i: usize) -> bool {
        self.slots[i].auto.is_some()
    }

    fn commit(&mut self, i: usize, choice: String, fx: &mut Vec<Effect>) {
        fx.push(Effect::Oled(OledCmd::ShowWaiting { player: self.slots[i].player_num }));
        self.out[i] = choice;
        self.st[i] = SlotState::Committed;
        self.grace_start = None;
    }

    fn unready(&mut self, i: usize, fx: &mut Vec<Effect>) {
        fx.push(Effect::Oled(self.slots[i].pick_screen()));
        self.out[i].clear();
        self.st[i] = SlotState::Choosing;
        self.grace_start = None;
    }

    fn reject_invalid(&mut self, i: usize, now_ms: u64, fx: &mut Vec<Effect>) {
        fx.push(Effect::Oled(OledCmd::ShowInvalidSelection { player: self.slots[i].player_num }));
        self.st[i] = SlotState::Invalid { until: now_ms + INVALID_FLASH_MS };
    }

    /// Feed a classified physical-button event.
    pub fn pad_event(&mut self, ev: PadEvent, now_ms: u64, fx: &mut Vec<Effect>) {
        let (player, action) = match ev {
            PadEvent::TapMove { player, slot } => (player, Pad::Tap(PlayerAction::Move(slot as usize))),
            PadEvent::TapSwitch { player, idx } => (player, Pad::Tap(PlayerAction::Switch(idx as usize))),
            PadEvent::HoldMove { player, slot } => (player, Pad::Hold(HoldView::Move(slot))),
            PadEvent::HoldSwitch { player, idx } => (player, Pad::Hold(HoldView::Stats(idx))),
            PadEvent::HoldEnd { player } => (player, Pad::HoldEnd),
        };
        let Some(i) = self.slot_index(player) else {
            return; // no prompt for this player this batch
        };

        match (self.st[i], action) {
            // Any press while committed unreadies (auto choices are fixed).
            (SlotState::Committed, Pad::Tap(_) | Pad::Hold(_)) => {
                if !self.is_auto(i) {
                    self.unready(i, fx);
                }
            }
            (SlotState::Committed, Pad::HoldEnd) => {}

            (SlotState::Choosing, Pad::Tap(action)) => self.tap(i, action, now_ms, fx),

            // A new hold OVERRIDES a detail view already showing — the
            // newest held button wins; the old one stays ignored (platforms
            // stale it out) until physically released.
            (
                SlotState::Choosing | SlotState::Detail | SlotState::Stats { .. },
                Pad::Hold(view),
            ) => {
                let s = &self.slots[i];
                match view {
                    HoldView::Move(slot) => {
                        fx.push(Effect::Oled(OledCmd::ShowMoveDetail {
                            player: s.player_num,
                            slot,
                        }));
                        self.st[i] = SlotState::Detail;
                    }
                    HoldView::Stats(idx) => {
                        fx.push(Effect::Oled(OledCmd::ShowPokemonStats {
                            player: s.player_num,
                            team_idx: idx,
                            page: 0,
                        }));
                        self.st[i] = SlotState::Stats {
                            team_idx: idx,
                            page: 0,
                            next_flip: now_ms + STATS_PAGE_CYCLE_MS,
                        };
                    }
                }
            }
            (SlotState::Detail | SlotState::Stats { .. }, Pad::HoldEnd) => {
                fx.push(Effect::Oled(self.slots[i].pick_screen()));
                self.st[i] = SlotState::Choosing;
            }
            // Ignore everything else (taps during the invalid flash, stray
            // hold-ends, holds during detail).
            _ => {}
        }
    }

    fn tap(&mut self, i: usize, action: PlayerAction, now_ms: u64, fx: &mut Vec<Effect>) {
        let s = &self.slots[i];
        if s.forced_switch {
            let PlayerAction::Switch(idx) = action else {
                return; // move buttons are inert during a forced switch
            };
            if s.party_ok.get(idx).copied().unwrap_or(false) {
                let choice = format_switch_choice(idx);
                self.commit(i, choice, fx);
            } else {
                self.reject_invalid(i, now_ms, fx);
            }
            return;
        }
        // Fainted-bench check first, then the shared turn rule.
        let fainted = matches!(action, PlayerAction::Switch(idx)
            if s.active_slot != Some(idx) && !s.party_ok.get(idx).copied().unwrap_or(false));
        if fainted {
            self.reject_invalid(i, now_ms, fx);
            return;
        }
        match turn_action_choice(&action, s.n_moves, &s.usable, s.trapped, s.active_slot) {
            Ok(choice) => self.commit(i, choice, fx),
            // A press on a move button beyond the move count is inert (both
            // platforms have always ignored those); real rule violations
            // (no PP, trapped, already active) flash the invalid screen.
            Err(ActionReject::OutOfRange) => {}
            Err(_) => self.reject_invalid(i, now_ms, fx),
        }
    }

    /// Feed a typed line (USB CLI / web terminal). Grammar:
    /// `pN <cmd>` always works; the bare form works while exactly one human
    /// is prompted. `<cmd>` is a move number, `sN`, `switch N`, or (during a
    /// forced switch) a party slot as `N` / `sN`. Typing for a committed
    /// player unreadies them.
    ///
    /// Button simulation (for testing without hardware): `:press pN <btn>`
    /// taps, `:hold pN <btn>` starts a long press, `:release pN` ends it —
    /// `<btn>` is a move number 1-4 or a party slot s1-s3. These go through
    /// [`Self::pad_event`], so they exercise the exact button code paths.
    pub fn typed_line(&mut self, line: &str, now_ms: u64, fx: &mut Vec<Effect>) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        if let Some(rest) = line.strip_prefix(":press ") {
            self.sim_button(rest, SimKind::Tap, now_ms, fx);
            return;
        }
        if let Some(rest) = line.strip_prefix(":hold ") {
            self.sim_button(rest, SimKind::Hold, now_ms, fx);
            return;
        }
        if let Some(rest) = line.strip_prefix(":release ") {
            match parse_player_ref(rest.trim()) {
                Some(player) => {
                    fx.push(Effect::Dbg(format!("sim release p{player}")));
                    self.pad_event(PadEvent::HoldEnd { player }, now_ms, fx);
                }
                None => fx.push(Effect::Err(String::from("usage: :release p1|p2"))),
            }
            return;
        }

        // Resolve the target slot: explicit "p1 "/"p2 " prefix, else the
        // single human slot when unambiguous.
        let mut target: Option<(usize, &str)> = None;
        for i in 0..self.n_real {
            let pid = self.slots[i].player_id.as_str();
            if let Some(rest) = line.strip_prefix(pid) {
                if rest.starts_with(' ') {
                    target = Some((i, rest.trim()));
                    break;
                }
            }
        }
        if target.is_none() {
            // A prefix naming a player with no prompt this batch.
            for pid in ["p1", "p2"] {
                if line.strip_prefix(pid).is_some_and(|r| r.starts_with(' ')) {
                    fx.push(Effect::Err(format!("{pid} has no pending prompt")));
                    return;
                }
            }
            // Bare form: unambiguous only while exactly one human has a
            // prompt (auto/AI slots take no typed input).
            let mut candidates = (0..self.n_real).filter(|&i| self.slots[i].auto.is_none());
            let (first, second) = (candidates.next(), candidates.next());
            match (first, second) {
                (Some(i), None) => target = Some((i, line)),
                (None, _) => {
                    fx.push(Effect::Err(String::from("no player is choosing right now")));
                    return;
                }
                _ => {
                    fx.push(Effect::Err(String::from(
                        "Both players are choosing — prefix with the player, e.g. \"p1 2\" or \"p2 s3\"",
                    )));
                    return;
                }
            }
        }
        let (i, rest) = target.expect("resolved above");
        let pid = self.slots[i].player_id.clone();

        if self.is_auto(i) {
            fx.push(Effect::Err(format!("{pid}'s action is forced this turn")));
            return;
        }
        // Typing anything for a committed player unreadies them — back to
        // their pick screen; they choose again with a fresh line or buttons.
        if self.st[i] == SlotState::Committed {
            self.unready(i, fx);
            fx.push(Effect::Ok(format!("{pid} unreadied — choose again")));
            return;
        }

        let s = &self.slots[i];
        if s.forced_switch {
            // Forced replacement after a faint: a party slot as "3" or "s3".
            let slot = rest.strip_prefix(['s', 'S']).unwrap_or(rest).trim();
            match parse_switch_line(slot) {
                Ok(idx) if s.party_ok.get(idx).copied().unwrap_or(false) => {
                    let choice = format_switch_choice(idx);
                    fx.push(Effect::Ok(format!("Accepted — {pid} {choice}")));
                    self.commit(i, choice, fx);
                }
                Ok(_) => {
                    fx.push(Effect::Err(format!(
                        "Rejected ({pid}) — That pokemon can no longer fight!"
                    )));
                }
                Err(msg) => fx.push(Effect::Err(format!("Rejected ({pid}) — {msg}"))),
            }
            return;
        }

        let action = match parse_turn_line(rest, s.n_moves) {
            Ok(TurnChoice::Move(m)) => PlayerAction::Move(m),
            Ok(TurnChoice::Switch(x)) => PlayerAction::Switch(x),
            Err(msg) => {
                fx.push(Effect::Err(format!("Rejected ({pid}) — {msg}")));
                return;
            }
        };
        if let PlayerAction::Switch(idx) = &action {
            // Already-active is caught by the shared validator; a benched
            // target that still fails here can only be fainted.
            if s.active_slot != Some(*idx) && !s.party_ok.get(*idx).copied().unwrap_or(false) {
                fx.push(Effect::Err(format!(
                    "Rejected ({pid}) — That pokemon can no longer fight!"
                )));
                return;
            }
        }
        match turn_action_choice(&action, s.n_moves, &s.usable, s.trapped, s.active_slot) {
            Ok(choice) => {
                fx.push(Effect::Ok(format!("Accepted — {pid} {choice}")));
                self.commit(i, choice, fx);
            }
            Err(reason) => {
                fx.push(Effect::Err(format!("Rejected ({pid}) — {}", reject_reason(reason))));
            }
        }
    }

    /// Parse and inject a simulated button: `pN <btn>` where `<btn>` is a
    /// move number 1-4 or a party slot s1-s3.
    fn sim_button(&mut self, rest: &str, kind: SimKind, now_ms: u64, fx: &mut Vec<Effect>) {
        let usage = "usage: :press|:hold pN <1-4|s1-s3>";
        let mut parts = rest.trim().split_whitespace();
        let (Some(pref), Some(bref), None) = (parts.next(), parts.next(), parts.next()) else {
            fx.push(Effect::Err(String::from(usage)));
            return;
        };
        let Some(player) = parse_player_ref(pref) else {
            fx.push(Effect::Err(String::from(usage)));
            return;
        };
        let ev = if let Some(n) = bref.strip_prefix(['s', 'S']) {
            match n.parse::<u8>() {
                Ok(i) if (1..=3).contains(&i) => match kind {
                    SimKind::Tap => PadEvent::TapSwitch { player, idx: i - 1 },
                    SimKind::Hold => PadEvent::HoldSwitch { player, idx: i - 1 },
                },
                _ => {
                    fx.push(Effect::Err(String::from(usage)));
                    return;
                }
            }
        } else {
            match bref.parse::<u8>() {
                Ok(i) if (1..=4).contains(&i) => match kind {
                    SimKind::Tap => PadEvent::TapMove { player, slot: i - 1 },
                    SimKind::Hold => PadEvent::HoldMove { player, slot: i - 1 },
                },
                _ => {
                    fx.push(Effect::Err(String::from(usage)));
                    return;
                }
            }
        };
        let verb = match kind {
            SimKind::Tap => "press",
            SimKind::Hold => "hold",
        };
        fx.push(Effect::Dbg(format!("sim {verb} p{player} {bref}")));
        self.pad_event(ev, now_ms, fx);
    }

    /// Advance timers: restores expired invalid flashes and runs the
    /// both-committed grace window. Returns true once collection is complete
    /// (call [`Self::take_choices`] then).
    pub fn tick(&mut self, now_ms: u64, fx: &mut Vec<Effect>) -> bool {
        if self.complete {
            return true;
        }
        for i in 0..self.n_real {
            match self.st[i] {
                SlotState::Invalid { until } if now_ms >= until => {
                    fx.push(Effect::Oled(self.slots[i].pick_screen()));
                    self.st[i] = SlotState::Choosing;
                }
                // Held party-stats view: alternate its two pages.
                SlotState::Stats { team_idx, page, next_flip } if now_ms >= next_flip => {
                    let page = page ^ 1;
                    fx.push(Effect::Oled(OledCmd::ShowPokemonStats {
                        player: self.slots[i].player_num,
                        team_idx,
                        page,
                    }));
                    self.st[i] = SlotState::Stats {
                        team_idx,
                        page,
                        next_flip: now_ms + STATS_PAGE_CYCLE_MS,
                    };
                }
                _ => {}
            }
        }
        let all_committed = (0..2).all(|i| self.st[i] == SlotState::Committed);
        if !all_committed {
            self.grace_start = None;
            return false;
        }
        // Forced/AI choices on every side: nothing to unready, skip the grace.
        if (0..2).all(|i| self.is_auto(i)) {
            self.complete = true;
            return true;
        }
        match self.grace_start {
            None => {
                self.grace_start = Some(now_ms);
                false
            }
            Some(t0) if now_ms.saturating_sub(t0) >= UNREADY_GRACE_MS => {
                self.complete = true;
                true
            }
            Some(_) => false,
        }
    }

    /// The collected `(player_id, choice)` pairs, in prompt order.
    pub fn take_choices(self) -> Vec<(String, String)> {
        let n = self.n_real;
        let mut out_iter = self.out.into_iter();
        self.slots
            .into_iter()
            .take(n)
            .map(|s| (s.player_id, out_iter.next().unwrap_or_default()))
            .collect()
    }
}

enum Pad {
    Tap(PlayerAction),
    Hold(HoldView),
    HoldEnd,
}

#[derive(Clone, Copy)]
enum SimKind {
    Tap,
    Hold,
}

/// "p1" / "p2" → player number.
fn parse_player_ref(s: &str) -> Option<u8> {
    match s {
        "p1" => Some(1),
        "p2" => Some(2),
        _ => None,
    }
}

enum HoldView {
    Move(u8),
    Stats(u8),
}

/// Human-readable reason for a rejected turn action.
pub fn reject_reason(r: ActionReject) -> &'static str {
    match r {
        ActionReject::OutOfRange => "no such move",
        ActionReject::Unusable => "move is disabled or out of PP",
        ActionReject::Trapped => "Pokémon is trapped, cannot switch",
        ActionReject::AlreadyActive => "that Pokémon is already in battle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gen1_battle::{MonTurnRequest, MoveSlot as ApiMove, SwitchRequest, TurnRequest};

    fn turn_prompt(player_id: &str, n_moves: usize) -> ActivePrompt {
        let moves = (0..n_moves)
            .map(|i| ApiMove {
                name: format!("Move{i}"),
                id: format!("move{i}"),
                typ: String::from("Normal"),
                pp: 10,
                max_pp: 10,
                disabled: false,
                target: 0,
            })
            .collect();
        ActivePrompt {
            player_id: String::from(player_id),
            request: Request::Turn(TurnRequest {
                active: alloc::vec![MonTurnRequest {
                    team_position: 0,
                    moves,
                    trapped: false,
                    locked_into_move: false,
                }],
            }),
            player_data: None,
            batch_total: 2,
        }
    }

    fn switch_prompt(player_id: &str) -> ActivePrompt {
        ActivePrompt {
            player_id: String::from(player_id),
            request: Request::Switch(SwitchRequest { needs_switch: alloc::vec![0] }),
            player_data: None,
            batch_total: 1,
        }
    }

    fn two_humans() -> (ChoiceCollector, Vec<Effect>) {
        let mut fx = Vec::new();
        let c = ChoiceCollector::new(
            alloc::vec![
                SlotOptions::from_prompt(&turn_prompt("p1", 4)),
                SlotOptions::from_prompt(&turn_prompt("p2", 4)),
            ],
            &mut fx,
        );
        (c, fx)
    }

    fn has_oled(fx: &[Effect], f: impl Fn(&OledCmd) -> bool) -> bool {
        fx.iter().any(|e| matches!(e, Effect::Oled(c) if f(c)))
    }

    fn err_containing(fx: &[Effect], needle: &str) -> bool {
        fx.iter().any(|e| matches!(e, Effect::Err(m) if m.contains(needle)))
    }

    #[test]
    fn tap_commits_and_shows_waiting() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 1 }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowWaiting { player: 1 })));
        assert_eq!(c.out[0], "move 1");
    }

    #[test]
    fn tap_while_committed_unreadies() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, 0, &mut fx);
        fx.clear();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 2 }, 100, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { player: 1 })));
        assert!(c.out[0].is_empty());
        assert_eq!(c.st[0], SlotState::Choosing);
    }

    #[test]
    fn grace_window_holds_completion_then_completes() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, 0, &mut fx);
        c.pad_event(PadEvent::TapMove { player: 2, slot: 1 }, 10, &mut fx);
        assert!(!c.tick(20, &mut fx), "grace just started");
        assert!(!c.tick(20 + UNREADY_GRACE_MS - 1, &mut fx));
        assert!(c.tick(20 + UNREADY_GRACE_MS, &mut fx));
        let ch = c.take_choices();
        assert_eq!(ch[0], (String::from("p1"), String::from("move 0")));
        assert_eq!(ch[1], (String::from("p2"), String::from("move 1")));
    }

    #[test]
    fn unready_during_grace_resets_it() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, 0, &mut fx);
        c.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, 0, &mut fx);
        assert!(!c.tick(10, &mut fx));
        c.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, 500, &mut fx); // unready
        assert!(!c.tick(10 + UNREADY_GRACE_MS + 100, &mut fx), "p2 is choosing again");
        c.typed_line("p2 3", 0, &mut fx);
        assert!(!c.tick(2000, &mut fx), "fresh grace");
        assert!(c.tick(2000 + UNREADY_GRACE_MS, &mut fx));
    }

    #[test]
    fn typed_prefix_and_bare_rules() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.typed_line("2", 0, &mut fx);
        assert!(err_containing(&fx, "Both players are choosing"));
        fx.clear();
        c.typed_line("p2 2", 0, &mut fx);
        assert_eq!(c.out[1], "move 1");
    }

    #[test]
    fn bare_line_targets_single_human_vs_ai() {
        let mut fx = Vec::new();
        let mut p2 = SlotOptions::from_prompt(&turn_prompt("p2", 4));
        p2.set_ai_choice(String::from("move 0"));
        let mut c = ChoiceCollector::new(
            alloc::vec![SlotOptions::from_prompt(&turn_prompt("p1", 4)), p2],
            &mut fx,
        );
        fx.clear();
        c.typed_line("3", 0, &mut fx);
        assert_eq!(c.out[0], "move 2");
        // AI can't be unreadied.
        fx.clear();
        c.typed_line("p2 1", 0, &mut fx);
        assert!(err_containing(&fx, "forced this turn"));
    }

    #[test]
    fn typing_while_committed_unreadies_instead_of_replacing() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.typed_line("p1 1", 0, &mut fx);
        assert_eq!(c.out[0], "move 0");
        fx.clear();
        c.typed_line("p1 4", 0, &mut fx);
        assert!(c.out[0].is_empty(), "unreadied, not replaced");
        assert!(fx.iter().any(|e| matches!(e, Effect::Ok(m) if m.contains("unreadied"))));
        c.typed_line("p1 4", 0, &mut fx);
        assert_eq!(c.out[0], "move 3");
    }

    #[test]
    fn sn_switch_syntax_in_turn() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.typed_line("p1 s2", 0, &mut fx);
        assert_eq!(c.out[0], "switch 1");
    }

    #[test]
    fn forced_switch_screens_and_validation() {
        let mut fx = Vec::new();
        let mut c = ChoiceCollector::new(
            alloc::vec![SlotOptions::from_prompt(&switch_prompt("p1"))],
            &mut fx,
        );
        // Init: switch picker for p1, waiting screen for promptless p2.
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowSwitchScreen { player: 1 })));
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowWaitingForOpponent { player: 2 })));

        // Without player data party_ok is all-true; typed slot works bare
        // (single human) and via sN.
        fx.clear();
        c.typed_line("s2", 0, &mut fx);
        assert_eq!(c.out[0], "switch 1");
        // Unready by tapping returns to the SWITCH picker, not the battle screen.
        fx.clear();
        c.pad_event(PadEvent::TapSwitch { player: 1, idx: 2 }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowSwitchScreen { player: 1 })));
        assert!(c.out[0].is_empty());
    }

    #[test]
    fn fainted_pick_flashes_invalid_then_restores() {
        let mut fx = Vec::new();
        let mut p1 = SlotOptions::from_prompt(&switch_prompt("p1"));
        p1.party_ok = [false, true, false, false, false, false];
        let mut c = ChoiceCollector::new(alloc::vec![p1], &mut fx);
        fx.clear();
        c.pad_event(PadEvent::TapSwitch { player: 1, idx: 2 }, 100, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowInvalidSelection { player: 1 })));
        // Taps during the flash are ignored.
        fx.clear();
        c.pad_event(PadEvent::TapSwitch { player: 1, idx: 1 }, 200, &mut fx);
        assert!(c.out[0].is_empty());
        // Restores to the switch picker at the deadline.
        c.tick(100 + INVALID_FLASH_MS, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowSwitchScreen { player: 1 })));
        // And a valid pick then commits.
        fx.clear();
        c.pad_event(PadEvent::TapSwitch { player: 1, idx: 1 }, 800, &mut fx);
        assert_eq!(c.out[0], "switch 1");
        // Typed fainted pick gets the message.
        let mut fx2 = Vec::new();
        c.typed_line("p1 s3", 0, &mut fx2); // unreadies first
        c.typed_line("p1 s3", 0, &mut fx2);
        assert!(err_containing(&fx2, "no longer fight"));
    }

    #[test]
    fn hold_shows_detail_and_restores_to_context() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::HoldMove { player: 1, slot: 2 }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowMoveDetail { player: 1, slot: 2 })));
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 600, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { player: 1 })));
        // Hold while committed unreadies instead of showing detail.
        c.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, 700, &mut fx);
        fx.clear();
        c.pad_event(PadEvent::HoldMove { player: 1, slot: 1 }, 800, &mut fx);
        assert!(!has_oled(&fx, |o| matches!(o, OledCmd::ShowMoveDetail { .. })));
        assert!(c.out[0].is_empty(), "unreadied");
    }

    #[test]
    fn stats_hold_cycles_pages() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: 1 }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { player: 1, team_idx: 1, page: 0 })));
        fx.clear();
        c.tick(STATS_PAGE_CYCLE_MS, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { page: 1, .. })));
        fx.clear();
        c.tick(2 * STATS_PAGE_CYCLE_MS, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { page: 0, .. })));
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 2 * STATS_PAGE_CYCLE_MS + 100, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { player: 1 })));
        // And the cycle stops.
        fx.clear();
        c.tick(4 * STATS_PAGE_CYCLE_MS, &mut fx);
        assert!(!has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { .. })));
    }

    #[test]
    fn sim_button_commands_drive_pad_events() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        // :press taps — commits like a real button.
        c.typed_line(":press p1 2", 0, &mut fx);
        assert_eq!(c.out[0], "move 1");
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowWaiting { player: 1 })));
        // :press while committed unreadies, like a real button.
        fx.clear();
        c.typed_line(":press p1 1", 10, &mut fx);
        assert!(c.out[0].is_empty());
        // :hold / :release drive the detail view.
        fx.clear();
        c.typed_line(":hold p2 s2", 20, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { player: 2, team_idx: 1, page: 0 })));
        fx.clear();
        c.typed_line(":release p2", 30, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { player: 2 })));
        // Bad refs get usage errors.
        fx.clear();
        c.typed_line(":press p3 1", 40, &mut fx);
        assert!(err_containing(&fx, "usage"));
        c.typed_line(":press p1 s4", 50, &mut fx);
        assert!(err_containing(&fx, "usage"));
    }

    #[test]
    fn second_hold_overrides_first_detail_view() {
        let (mut c, _) = two_humans();
        let mut fx = Vec::new();
        c.pad_event(PadEvent::HoldMove { player: 1, slot: 0 }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowMoveDetail { player: 1, slot: 0 })));
        // Second hold while the first view is up → view swaps directly.
        fx.clear();
        c.pad_event(PadEvent::HoldMove { player: 1, slot: 1 }, 100, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowMoveDetail { player: 1, slot: 1 })));
        assert!(!has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { .. })));
        // Works across view kinds too (move detail → party stats).
        fx.clear();
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: 2 }, 200, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowPokemonStats { player: 1, team_idx: 2, page: 0 })));
        // Release of the newest hold restores the pick screen.
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 300, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::RestoreScreen { player: 1 })));
        // A stray HoldEnd afterwards (first button finally released) is inert.
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 400, &mut fx);
        assert!(fx.is_empty());
    }

    #[test]
    fn both_auto_completes_without_grace() {
        let mut fx = Vec::new();
        let mut p1 = SlotOptions::from_prompt(&turn_prompt("p1", 4));
        let mut p2 = SlotOptions::from_prompt(&turn_prompt("p2", 4));
        p1.set_ai_choice(String::from("move 0"));
        p2.set_ai_choice(String::from("move 1"));
        let mut c = ChoiceCollector::new(alloc::vec![p1, p2], &mut fx);
        assert!(c.tick(0, &mut fx), "no grace when nobody can unready");
    }

    #[test]
    fn single_prompt_batch_pads_inert_and_completes() {
        let mut fx = Vec::new();
        let mut c = ChoiceCollector::new(
            alloc::vec![SlotOptions::from_prompt(&switch_prompt("p2"))],
            &mut fx,
        );
        c.typed_line("2", 0, &mut fx);
        assert!(!c.tick(0, &mut fx));
        assert!(c.tick(UNREADY_GRACE_MS, &mut fx));
        let ch = c.take_choices();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0], (String::from("p2"), String::from("switch 1")));
    }

    #[test]
    fn prefix_for_promptless_player_rejected() {
        let mut fx = Vec::new();
        let mut c = ChoiceCollector::new(
            alloc::vec![SlotOptions::from_prompt(&switch_prompt("p1"))],
            &mut fx,
        );
        fx.clear();
        c.typed_line("p2 1", 0, &mut fx);
        assert!(err_containing(&fx, "no pending prompt"));
    }
}
