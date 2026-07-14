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
use crate::rng::SimpleRng;

/// After every player has committed, either may still unready for this long
/// before the choices become final.
pub const UNREADY_GRACE_MS: u64 = 1000;
/// An AI player "thinks" for this long before entering its move. Multiple AI
/// players think in parallel (deadlines all armed on the first tick), so an
/// AI-vs-AI turn costs 1× this, not 2×.
pub const AI_THINK_MS: u64 = 2000;
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

/// Input scheme a player chose at battle start.
///
/// Concealed controls exist because the players sit across from each other:
/// which physical button you press is visible, so the meaning of every
/// button is randomized per turn and revealed only on your own screen,
/// behind a hold gesture.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ControlMode {
    #[default]
    Normal,
    Concealed,
}

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
    /// HIDDEN: all four corner (move) buttons pressed simultaneously.
    /// During the lobby ready sequence this toggles 6v6 teams.
    Chord4 { player: u8 },
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
    /// Input scheme (see [`ControlMode`]); everything below is Concealed-only,
    /// randomized once per combat turn by [`Self::set_concealed`].
    mode: ControlMode,
    /// Bottom-row position (0..3) of the Attack / Switch actions (distinct).
    attack_pos: u8,
    switch_pos: u8,
    /// Corner button (TL/TR/BL/BR) → real move slot; -1 = dead corner.
    move_map: [i8; 4],
    /// Eligible bench team indices, pre-shuffled. When more than 4, each
    /// menu open shows the next window of 4 (reopen to see the rest).
    bench: Vec<u8>,
    /// Corner order the bench entries land on (scatter).
    switch_positions: [u8; 4],
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
            mode: ControlMode::Normal,
            attack_pos: 0,
            switch_pos: 1,
            move_map: [-1; 4],
            bench: Vec::new(),
            switch_positions: [0, 1, 2, 3],
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

    /// Switch this slot to concealed controls, randomizing the action
    /// positions and corner layouts for this combat turn. `seed` should
    /// change every turn (e.g. a millisecond clock); layouts stay FIXED
    /// within the turn — release/re-hold and unready always show the same
    /// placements. No-op for auto slots (locked moves, AI): nothing to hide.
    pub fn set_concealed(&mut self, seed: u64) {
        if self.auto.is_some() {
            return;
        }
        self.mode = ControlMode::Concealed;
        let mut rng = SimpleRng::new(
            seed ^ ((self.player_num as u64) << 56) ^ 0x5eed_c0de_0b5c_u64,
        );
        // Attack and Switch land on two distinct bottom-row buttons.
        let a = (rng.next_u64() % 3) as u8;
        let mut sw = (rng.next_u64() % 2) as u8;
        if sw >= a {
            sw += 1;
        }
        self.attack_pos = a;
        self.switch_pos = sw;
        // Scatter the moves across all 4 corners (dead corners leak nothing).
        let corner_order = shuffled4(&mut rng);
        self.move_map = [-1; 4];
        for (k, corner) in corner_order.iter().enumerate().take(self.n_moves.min(4)) {
            self.move_map[*corner as usize] = k as i8;
        }
        // Switch-menu contents, shuffled once for the turn: the eligible
        // bench, plus the ACTIVE mon on in-turn menus (picking it flashes
        // invalid, holding it shows its stats — same info as normal mode).
        self.bench = (0..6u8)
            .filter(|&i| self.party_ok[i as usize])
            .collect();
        if !self.forced_switch {
            if let Some(a) = self.active_slot {
                let a = a as u8;
                if a < 6 && !self.bench.contains(&a) {
                    self.bench.push(a);
                }
            }
        }
        let n = self.bench.len();
        for i in (1..n).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            self.bench.swap(i, j);
        }
        self.switch_positions = shuffled4(&mut rng);
    }

    /// Concealed switch-menu layout for the `opens`-th open this turn:
    /// corner → team index (-1 dead). With more than 4 benched mons, each
    /// open shows the next window of 4.
    fn switch_corner_map(&self, opens: u8) -> [i8; 4] {
        let mut map = [-1i8; 4];
        let n = self.bench.len();
        if n == 0 {
            return map;
        }
        let base = if n > 4 { (opens as usize * 4) % n } else { 0 };
        for k in 0..n.min(4) {
            let team_idx = self.bench[(base + k) % n];
            map[self.switch_positions[k] as usize] = team_idx as i8;
        }
        map
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
            mode: ControlMode::Normal,
            attack_pos: 0,
            switch_pos: 1,
            move_map: [-1; 4],
            bench: Vec::new(),
            switch_positions: [0, 1, 2, 3],
        }
    }

    /// The screen this player returns to while still choosing (Normal mode).
    fn pick_screen(&self) -> OledCmd {
        if self.forced_switch {
            OledCmd::ShowSwitchScreen { player: self.player_num }
        } else {
            OledCmd::RestoreScreen { player: self.player_num }
        }
    }
}

/// Fisher-Yates over the 4 corner positions.
fn shuffled4(rng: &mut SimpleRng) -> [u8; 4] {
    let mut p = [0u8, 1, 2, 3];
    for i in (1..4usize).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        p.swap(i, j);
    }
    p
}

/// Which concealed corner menu is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CMenuKind {
    Moves,
    Switch,
}

/// Where the invalid-selection flash restores to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InvalidBack {
    /// The mode's pick screen (battle screen / switch picker / action select).
    Pick,
    /// A concealed corner menu that was open when the invalid pick happened.
    Menu { kind: CMenuKind, held: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SlotState {
    /// Waiting for a selection (Normal: battle screen; Concealed: the
    /// randomized Attack/Switch action-select screen).
    Choosing,
    /// A long-press move-detail view is up; restores on HoldEnd.
    Detail,
    /// A long-press party-stats view is up; its two pages alternate every
    /// [`STATS_PAGE_CYCLE_MS`] until HoldEnd.
    Stats { team_idx: u8, page: u8, next_flip: u64 },
    /// Invalid-selection screen is up; restores at `until`.
    Invalid { until: u64, back: InvalidBack },
    /// Choice locked in (waiting screen shown, unless auto).
    Committed,
    /// Concealed corner menu (moves or bench). `held` = opened by holding an
    /// action button, so releasing it exits; a forced-switch menu isn't held.
    CMenu { kind: CMenuKind, held: bool },
    /// Concealed nested detail (move detail / mon stats) with the menu's
    /// action button still held underneath. Stats pages cycle like [`Self::Stats`].
    CDetail { kind: CMenuKind, held: bool, real: u8, page: u8, next_flip: u64 },
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
    /// Per-slot deadline for an AI commit. Armed on the first tick after
    /// construction (the collector has no clock until then), fired in tick.
    ai_commit_at: [Option<u64>; 2],
    /// Concealed: how many times each slot's switch menu was opened this
    /// turn (drives the >4-bench windowing) and the layout of the currently
    /// open window.
    c_opens: [u8; 2],
    c_switch_map: [[i8; 4]; 2],
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
        let mut c_opens = [0u8; 2];
        let mut c_switch_map = [[-1i8; 4]; 2];
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
                } else if s.mode == ControlMode::Concealed && s.auto.is_none() {
                    if s.forced_switch {
                        // Straight to the randomized bench menu, no hold.
                        c_switch_map[i] = s.switch_corner_map(0);
                        c_opens[i] = 1;
                        st[i] = SlotState::CMenu { kind: CMenuKind::Switch, held: false };
                        fx.push(Effect::Oled(OledCmd::ShowConcealedSwitch {
                            player: s.player_num,
                            map: c_switch_map[i],
                        }));
                    } else {
                        fx.push(Effect::Oled(OledCmd::ShowActionSelect {
                            player: s.player_num,
                            attack_pos: s.attack_pos,
                            switch_pos: s.switch_pos,
                        }));
                    }
                    fx.push(Effect::Text(s.prompt_text.clone()));
                } else {
                    if s.forced_switch {
                        fx.push(Effect::Oled(OledCmd::ShowSwitchScreen { player: s.player_num }));
                    }
                    fx.push(Effect::Text(s.prompt_text.clone()));
                }
            }
            // Forced choices commit instantly; an AI's choice is held back in
            // Choosing until its think delay elapses (armed/fired in tick).
            if let Some(c) = &s.auto {
                if !s.is_ai {
                    out[i] = c.clone();
                    st[i] = SlotState::Committed;
                }
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

        Self {
            slots,
            st,
            out,
            n_real,
            grace_start: None,
            complete: false,
            ai_commit_at: [None; 2],
            c_opens,
            c_switch_map,
        }
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
        self.out[i].clear();
        self.grace_start = None;
        self.restore_pick(i, fx);
    }

    /// Show the mode's pick screen and enter its choosing state: battle
    /// screen / switch picker (Normal), action select or the forced bench
    /// menu (Concealed). Same randomized layouts as before — they're fixed
    /// for the combat turn.
    fn restore_pick(&mut self, i: usize, fx: &mut Vec<Effect>) {
        if self.slots[i].mode == ControlMode::Concealed {
            if self.slots[i].forced_switch {
                self.show_menu(i, CMenuKind::Switch, false, fx);
            } else {
                self.show_action_select(i, fx);
            }
        } else {
            fx.push(Effect::Oled(self.slots[i].pick_screen()));
            self.st[i] = SlotState::Choosing;
        }
    }

    fn reject_invalid(&mut self, i: usize, now_ms: u64, fx: &mut Vec<Effect>) {
        self.reject_invalid_back(i, now_ms, InvalidBack::Pick, fx);
    }

    fn reject_invalid_back(&mut self, i: usize, now_ms: u64, back: InvalidBack, fx: &mut Vec<Effect>) {
        fx.push(Effect::Oled(OledCmd::ShowInvalidSelection { player: self.slots[i].player_num }));
        self.st[i] = SlotState::Invalid { until: now_ms + INVALID_FLASH_MS, back };
    }

    // ── Concealed-mode screens/state ─────────────────────────────────────────

    fn show_action_select(&mut self, i: usize, fx: &mut Vec<Effect>) {
        let s = &self.slots[i];
        fx.push(Effect::Oled(OledCmd::ShowActionSelect {
            player: s.player_num,
            attack_pos: s.attack_pos,
            switch_pos: s.switch_pos,
        }));
        self.st[i] = SlotState::Choosing;
    }

    /// (Re-)show an open concealed menu with the current layout.
    fn show_menu(&mut self, i: usize, kind: CMenuKind, held: bool, fx: &mut Vec<Effect>) {
        let s = &self.slots[i];
        let cmd = match kind {
            CMenuKind::Moves => OledCmd::ShowConcealedMoves {
                player: s.player_num,
                map: s.move_map,
            },
            CMenuKind::Switch => OledCmd::ShowConcealedSwitch {
                player: s.player_num,
                map: self.c_switch_map[i],
            },
        };
        fx.push(Effect::Oled(cmd));
        self.st[i] = SlotState::CMenu { kind, held };
    }

    /// Open a menu fresh (switch menus advance the >4-bench window per open).
    fn open_menu(&mut self, i: usize, kind: CMenuKind, held: bool, fx: &mut Vec<Effect>) {
        if kind == CMenuKind::Switch {
            self.c_switch_map[i] = self.slots[i].switch_corner_map(self.c_opens[i]);
            self.c_opens[i] = self.c_opens[i].wrapping_add(1);
        }
        self.show_menu(i, kind, held, fx);
    }

    /// A corner button was tapped with a concealed menu open: map it to the
    /// real move/mon and commit (or flash invalid). Dead corners are inert.
    fn pick_corner(
        &mut self,
        i: usize,
        kind: CMenuKind,
        held: bool,
        corner: usize,
        now_ms: u64,
        fx: &mut Vec<Effect>,
    ) {
        if corner >= 4 {
            return;
        }
        let s = &self.slots[i];
        match kind {
            CMenuKind::Moves => {
                let real = s.move_map[corner];
                if real < 0 {
                    return;
                }
                let action = PlayerAction::Move(real as usize);
                match turn_action_choice(&action, s.n_moves, &s.usable, s.trapped, s.active_slot) {
                    Ok(choice) => self.commit(i, choice, fx),
                    Err(ActionReject::OutOfRange) => {}
                    Err(_) => self.reject_invalid_back(i, now_ms, InvalidBack::Menu { kind, held }, fx),
                }
            }
            CMenuKind::Switch => {
                let real = self.c_switch_map[i][corner];
                if real < 0 {
                    return;
                }
                let idx = real as usize;
                if s.forced_switch {
                    // The bench menu only contains valid targets.
                    let choice = format_switch_choice(idx);
                    self.commit(i, choice, fx);
                } else {
                    let action = PlayerAction::Switch(idx);
                    match turn_action_choice(&action, s.n_moves, &s.usable, s.trapped, s.active_slot) {
                        Ok(choice) => self.commit(i, choice, fx),
                        Err(_) => {
                            self.reject_invalid_back(i, now_ms, InvalidBack::Menu { kind, held }, fx)
                        }
                    }
                }
            }
        }
    }

    /// A corner button crossed the hold threshold with a menu open: nested
    /// detail view (move detail / mon stats) — release returns to the menu.
    fn open_detail(
        &mut self,
        i: usize,
        kind: CMenuKind,
        held: bool,
        corner: usize,
        now_ms: u64,
        fx: &mut Vec<Effect>,
    ) {
        if corner >= 4 {
            return;
        }
        let s = &self.slots[i];
        let real = match kind {
            CMenuKind::Moves => s.move_map[corner],
            CMenuKind::Switch => self.c_switch_map[i][corner],
        };
        if real < 0 {
            return;
        }
        match kind {
            CMenuKind::Moves => {
                fx.push(Effect::Oled(OledCmd::ShowMoveDetail {
                    player: s.player_num,
                    slot: real as u8,
                }));
                self.st[i] = SlotState::CDetail {
                    kind,
                    held,
                    real: real as u8,
                    page: 0,
                    next_flip: u64::MAX,
                };
            }
            CMenuKind::Switch => {
                fx.push(Effect::Oled(OledCmd::ShowPokemonStats {
                    player: s.player_num,
                    team_idx: real as u8,
                    page: 0,
                }));
                self.st[i] = SlotState::CDetail {
                    kind,
                    held,
                    real: real as u8,
                    page: 0,
                    next_flip: now_ms + STATS_PAGE_CYCLE_MS,
                };
            }
        }
    }

    /// Concealed-mode input handling (states: Choosing = action select).
    fn pad_event_concealed(&mut self, i: usize, action: Pad, now_ms: u64, fx: &mut Vec<Effect>) {
        match (self.st[i], action) {
            // Any press while committed unreadies (auto slots never get here).
            (SlotState::Committed, Pad::Tap(_) | Pad::Hold(_)) => self.unready(i, fx),
            (SlotState::Committed, Pad::HoldEnd) => {}

            // Action select: HOLD Attack/Switch on the bottom row.
            (SlotState::Choosing, Pad::Hold(HoldView::Stats(idx))) => {
                let s = &self.slots[i];
                if idx == s.attack_pos {
                    self.open_menu(i, CMenuKind::Moves, true, fx);
                } else if idx == s.switch_pos {
                    self.open_menu(i, CMenuKind::Switch, true, fx);
                }
                // The third (dead) bottom button is inert — free decoy.
            }
            (SlotState::Choosing, _) => {}

            // Menu open: tap a corner to commit, hold it for details,
            // release the action button to back out.
            (SlotState::CMenu { kind, held }, Pad::Tap(PlayerAction::Move(corner))) => {
                self.pick_corner(i, kind, held, corner, now_ms, fx);
            }
            (SlotState::CMenu { kind, held }, Pad::Hold(HoldView::Move(corner))) => {
                self.open_detail(i, kind, held, corner as usize, now_ms, fx);
            }
            (SlotState::CMenu { held: true, .. }, Pad::HoldEnd) => {
                self.show_action_select(i, fx);
            }
            (SlotState::CMenu { .. }, _) => {}

            // Nested detail: release returns one level up to the menu; a
            // different corner hold swaps the view directly.
            (SlotState::CDetail { kind, held, .. }, Pad::HoldEnd) => {
                self.show_menu(i, kind, held, fx);
            }
            (SlotState::CDetail { kind, held, .. }, Pad::Hold(HoldView::Move(corner))) => {
                self.open_detail(i, kind, held, corner as usize, now_ms, fx);
            }
            (SlotState::CDetail { .. }, _) => {}

            // Invalid flash: releasing the action button retargets the
            // restore to the action-select screen.
            (
                SlotState::Invalid { until, back: InvalidBack::Menu { held: true, .. } },
                Pad::HoldEnd,
            ) => {
                self.st[i] = SlotState::Invalid { until, back: InvalidBack::Pick };
            }
            (SlotState::Invalid { .. }, _) => {}

            // Normal-only states (Detail/Stats) never occur in concealed mode.
            _ => {}
        }
    }

    /// Feed a classified physical-button event.
    pub fn pad_event(&mut self, ev: PadEvent, now_ms: u64, fx: &mut Vec<Effect>) {
        let (player, action) = match ev {
            PadEvent::TapMove { player, slot } => (player, Pad::Tap(PlayerAction::Move(slot as usize))),
            PadEvent::TapSwitch { player, idx } => (player, Pad::Tap(PlayerAction::Switch(idx as usize))),
            PadEvent::HoldMove { player, slot } => (player, Pad::Hold(HoldView::Move(slot))),
            PadEvent::HoldSwitch { player, idx } => (player, Pad::Hold(HoldView::Stats(idx))),
            PadEvent::HoldEnd { player } => (player, Pad::HoldEnd),
            PadEvent::Chord4 { .. } => return, // lobby-only combo
        };
        let Some(i) = self.slot_index(player) else {
            return; // no prompt for this player this batch
        };
        // Auto slots (AI, locked moves) take no physical input — including a
        // still-thinking AI, whose slot sits in Choosing until its deadline.
        if self.is_auto(i) {
            return;
        }
        if self.slots[i].mode == ControlMode::Concealed {
            self.pad_event_concealed(i, action, now_ms, fx);
            return;
        }

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
        // AI think delay: arm every AI slot's deadline on the first tick (so
        // AI-vs-AI waits run in PARALLEL), commit each when its time is up.
        for i in 0..self.n_real {
            if !self.slots[i].is_ai || self.st[i] == SlotState::Committed {
                continue;
            }
            match self.ai_commit_at[i] {
                None => self.ai_commit_at[i] = Some(now_ms + AI_THINK_MS),
                Some(at) if now_ms >= at => {
                    // Commit without the ShowWaiting effect or a grace reset —
                    // matching how instant AI commits behaved before the delay.
                    self.out[i] = self.slots[i].auto.clone().unwrap_or_default();
                    self.st[i] = SlotState::Committed;
                }
                Some(_) => {}
            }
        }
        for i in 0..self.n_real {
            match self.st[i] {
                SlotState::Invalid { until, back } if now_ms >= until => match back {
                    InvalidBack::Pick => self.restore_pick(i, fx),
                    InvalidBack::Menu { kind, held } => self.show_menu(i, kind, held, fx),
                },
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
                // Concealed nested stats view: same page cycling.
                SlotState::CDetail { kind: CMenuKind::Switch, held, real, page, next_flip }
                    if now_ms >= next_flip =>
                {
                    let page = page ^ 1;
                    fx.push(Effect::Oled(OledCmd::ShowPokemonStats {
                        player: self.slots[i].player_num,
                        team_idx: real,
                        page,
                    }));
                    self.st[i] = SlotState::CDetail {
                        kind: CMenuKind::Switch,
                        held,
                        real,
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

// ─────────────────────────────────────────────────────────────────────────────
// Ready sequence (lobby): press → choose controls → READY, per player
// ─────────────────────────────────────────────────────────────────────────────

/// One player's position in the ready sequence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SeqSlot {
    /// Start screen ("PRESS TO READY").
    Idle,
    /// Controls picker is up.
    Choosing,
    /// Controls confirmed — READY (or AI) screen.
    Ready,
}

/// The shared lobby ready sequence. Per player: the start screen, then any
/// press opens the controls picker, then confirming (middle button) lands on
/// the READY screen. Any press while ready reopens the picker. A long press
/// from the start screen makes the OPPONENT an AI (which is instantly ready).
/// Once both players are ready, a 1-second grace runs before completion.
///
/// Driven exactly like [`ChoiceCollector`]: pad events + typed lines + ticks
/// in, [`Effect`]s out. Both platforms MUST use this — no lobby drift.
pub struct ReadySequence {
    st: [SeqSlot; 2],
    highlighted: [u8; 2], // 0 = Normal, 1 = Concealed
    ai: [bool; 2],
    grace_start: Option<u64>,
    complete: bool,
    /// HIDDEN: 6v6 teams for the upcoming battle (4-corner chord toggle).
    six_v_six: bool,
    /// A mode flash is showing; tick restores the state screens at this time.
    flash_restore_at: Option<u64>,
    flash_pending: bool,
}

impl ReadySequence {
    pub fn new(fx: &mut Vec<Effect>) -> Self {
        let s = Self {
            st: [SeqSlot::Idle; 2],
            highlighted: [0; 2],
            ai: [false; 2],
            grace_start: None,
            complete: false,
            six_v_six: false,
            flash_restore_at: None,
            flash_pending: false,
        };
        s.show(0, fx);
        s.show(1, fx);
        s
    }

    fn show(&self, i: usize, fx: &mut Vec<Effect>) {
        let player = (i + 1) as u8;
        let cmd = match self.st[i] {
            SeqSlot::Idle => OledCmd::LobbyState { player, ready: false, ai: false },
            SeqSlot::Choosing => OledCmd::ShowControlsSelect {
                player,
                highlighted: self.highlighted[i],
                confirmed: false,
            },
            SeqSlot::Ready => OledCmd::LobbyState { player, ready: true, ai: self.ai[i] },
        };
        fx.push(Effect::Oled(cmd));
    }

    /// Pre-mark players as AI (demo mode, VS-AI buttons): AI sides are
    /// instantly ready; human sides are dropped into the picker.
    pub fn ai_preset(&mut self, ai: [bool; 2], fx: &mut Vec<Effect>) {
        for i in 0..2 {
            if ai[i] {
                self.ai[i] = true;
                self.st[i] = SeqSlot::Ready;
            } else if self.st[i] == SeqSlot::Idle {
                self.st[i] = SeqSlot::Choosing;
            }
            self.show(i, fx);
        }
        self.grace_start = None;
    }

    /// Long-press: `player` requests an AI opponent — the opponent becomes
    /// AI (ready), the presser proceeds to the controls picker.
    pub fn request_ai_opponent(&mut self, player: u8, fx: &mut Vec<Effect>) {
        if !(1..=2).contains(&player) {
            return;
        }
        let me = (player - 1) as usize;
        let other = 1 - me;
        self.ai[other] = true;
        self.st[other] = SeqSlot::Ready;
        self.show(other, fx);
        if self.st[me] == SeqSlot::Idle {
            self.st[me] = SeqSlot::Choosing;
            self.show(me, fx);
        }
        self.grace_start = None;
    }

    /// CLI convenience (`:ready pN`): ready with the current highlight
    /// (Normal unless the picker grammar changed it) — skips the picker.
    pub fn set_ready_cmd(&mut self, player: u8, fx: &mut Vec<Effect>) {
        if !(1..=2).contains(&player) {
            return;
        }
        let i = (player - 1) as usize;
        self.st[i] = SeqSlot::Ready;
        self.show(i, fx);
    }

    /// CLI unready: back to the start screen; AI assignments are dropped
    /// (same as the old lobby's unready semantics).
    pub fn set_unready_cmd(&mut self, player: u8, fx: &mut Vec<Effect>) {
        if !(1..=2).contains(&player) {
            return;
        }
        let i = (player - 1) as usize;
        self.st[i] = SeqSlot::Idle;
        self.ai = [false, false];
        self.grace_start = None;
        self.show(0, fx);
        self.show(1, fx);
    }

    /// Feed a classified physical-button event. Holds on the start screen
    /// request an AI opponent (the classic lobby long-press); everywhere
    /// else holds act like presses. Releases are ignored.
    pub fn pad_event(&mut self, ev: PadEvent, fx: &mut Vec<Effect>) {
        let (player, bottom_idx, is_hold) = match ev {
            PadEvent::TapSwitch { player, idx } => (player, Some(idx), false),
            PadEvent::HoldSwitch { player, idx } => (player, Some(idx), true),
            PadEvent::TapMove { player, .. } => (player, None, false),
            PadEvent::HoldMove { player, .. } => (player, None, true),
            PadEvent::HoldEnd { .. } => return,
            PadEvent::Chord4 { .. } => {
                // Hidden combo: toggle 6v6 for the upcoming battle. Flash the
                // mode on both screens; tick restores the state screens.
                self.six_v_six = !self.six_v_six;
                if self.six_v_six {
                    // 6v6 forces concealed controls for both players.
                    self.highlighted = [1, 1];
                }
                let label = if self.six_v_six { "6V6 CONCEALED!" } else { "3V3 MODE" };
                let (text, len) = crate::oled_ctl::flash_buf(label);
                fx.push(Effect::Oled(OledCmd::EventFlash { player: 0, text, len }));
                fx.push(Effect::Ok(format!("hidden combo: {label}")));
                self.flash_pending = true;
                self.flash_restore_at = None;
                return;
            }
        };
        if !(1..=2).contains(&player) {
            return;
        }
        let i = (player - 1) as usize;

        // A press on an AI-assigned side reclaims it for a human.
        if self.ai[i] {
            self.ai[i] = false;
            self.st[i] = SeqSlot::Choosing;
            self.grace_start = None;
            self.show(i, fx);
            return;
        }

        match self.st[i] {
            SeqSlot::Idle => {
                if is_hold {
                    self.request_ai_opponent(player, fx);
                } else {
                    self.st[i] = SeqSlot::Choosing;
                    self.show(i, fx);
                }
            }
            SeqSlot::Choosing => match bottom_idx {
                Some(0) | Some(2) => {
                    // Left/right: swap between the two options.
                    self.highlighted[i] ^= 1;
                    self.show(i, fx);
                }
                Some(1) => {
                    // Middle: confirm → READY.
                    self.st[i] = SeqSlot::Ready;
                    self.show(i, fx);
                    let mode = if self.highlighted[i] == 1 { "concealed" } else { "normal" };
                    fx.push(Effect::Ok(format!("p{} ready ({mode} controls)", i + 1)));
                }
                _ => {} // corner buttons do nothing while choosing
            },
            SeqSlot::Ready => {
                // Any press while ready: back to the picker.
                self.st[i] = SeqSlot::Choosing;
                self.grace_start = None;
                self.show(i, fx);
            }
        }
    }

    /// Typed input: `pN normal` / `pN concealed` / `pN ok`, plus the same
    /// `:press`/`:hold`/`:release` button sims the battle collector accepts.
    pub fn typed_line(&mut self, line: &str, fx: &mut Vec<Effect>) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if let Some(ev) = parse_sim_pad_line(line) {
            self.pad_event(ev, fx);
            return;
        }
        for (i, pid) in ["p1", "p2"].iter().enumerate() {
            if let Some(rest) = line.strip_prefix(pid) {
                let rest = rest.trim();
                if self.ai[i] {
                    fx.push(Effect::Err(format!("{pid} is AI-controlled")));
                    return;
                }
                match rest {
                    "normal" => {
                        self.highlighted[i] = 0;
                        self.st[i] = SeqSlot::Choosing;
                        self.grace_start = None;
                        self.show(i, fx);
                    }
                    "concealed" => {
                        self.highlighted[i] = 1;
                        self.st[i] = SeqSlot::Choosing;
                        self.grace_start = None;
                        self.show(i, fx);
                    }
                    "ok" | "confirm" => {
                        self.st[i] = SeqSlot::Ready;
                        self.show(i, fx);
                    }
                    _ => fx.push(Effect::Err(String::from(
                        "controls: pN normal | pN concealed | pN ok",
                    ))),
                }
                return;
            }
        }
        fx.push(Effect::Err(String::from(
            "controls: pN normal | pN concealed | pN ok",
        )));
    }

    /// Advance the both-ready grace window (and the mode-flash restore).
    /// True once the sequence is final.
    pub fn tick(&mut self, now_ms: u64, fx: &mut Vec<Effect>) -> bool {
        // Restore the state screens after a mode-toggle flash.
        if self.flash_pending {
            match self.flash_restore_at {
                None => self.flash_restore_at = Some(now_ms + 1200),
                Some(at) if now_ms >= at => {
                    self.flash_pending = false;
                    self.flash_restore_at = None;
                    self.show(0, fx);
                    self.show(1, fx);
                }
                Some(_) => {}
            }
        }
        if self.complete {
            return true;
        }
        if self.st != [SeqSlot::Ready; 2] {
            self.grace_start = None;
            return false;
        }
        if self.ai[0] && self.ai[1] {
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

    /// Which players are currently on the READY screen (for platform LED /
    /// status displays).
    pub fn ready_flags(&self) -> [bool; 2] {
        [self.st[0] == SeqSlot::Ready, self.st[1] == SeqSlot::Ready]
    }

    /// HIDDEN: 6v6 teams armed via the 4-corner chord.
    pub fn six_v_six(&self) -> bool {
        self.six_v_six
    }

    /// AI assignments + chosen control modes. 6v6 (hidden chord) forces
    /// concealed controls for both players.
    pub fn take(self) -> ([bool; 2], [ControlMode; 2]) {
        let modes = if self.six_v_six {
            [ControlMode::Concealed; 2]
        } else {
            self.highlighted
                .map(|h| if h == 1 { ControlMode::Concealed } else { ControlMode::Normal })
        };
        (self.ai, modes)
    }
}

/// Parse a `:press pN <btn>` / `:hold pN <btn>` / `:release pN` sim line into
/// a [`PadEvent`] (`<btn>` = move 1-4 or party s1-s3). Shared by every input
/// phase so headless tests can drive buttons anywhere.
pub fn parse_sim_pad_line(line: &str) -> Option<PadEvent> {
    let (kind, rest) = if let Some(r) = line.strip_prefix(":press ") {
        (SimKind::Tap, r)
    } else if let Some(r) = line.strip_prefix(":hold ") {
        (SimKind::Hold, r)
    } else if let Some(r) = line.strip_prefix(":release ") {
        let player = parse_player_ref(r.trim())?;
        return Some(PadEvent::HoldEnd { player });
    } else {
        return None;
    };
    let mut parts = rest.trim().split_whitespace();
    let (pref, bref) = (parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let player = parse_player_ref(pref)?;
    if bref == "all" {
        return match kind {
            SimKind::Tap => Some(PadEvent::Chord4 { player }),
            SimKind::Hold => None,
        };
    }
    if let Some(n) = bref.strip_prefix(['s', 'S']) {
        let i: u8 = n.parse().ok()?;
        if !(1..=3).contains(&i) {
            return None;
        }
        Some(match kind {
            SimKind::Tap => PadEvent::TapSwitch { player, idx: i - 1 },
            SimKind::Hold => PadEvent::HoldSwitch { player, idx: i - 1 },
        })
    } else {
        let i: u8 = bref.parse().ok()?;
        if !(1..=4).contains(&i) {
            return None;
        }
        Some(match kind {
            SimKind::Tap => PadEvent::TapMove { player, slot: i - 1 },
            SimKind::Hold => PadEvent::HoldMove { player, slot: i - 1 },
        })
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

    // ── Ready sequence (press → choose controls → READY) ────────────────

    #[test]
    fn ready_sequence_press_choose_confirm_grace() {
        let mut fx = Vec::new();
        let mut seq = ReadySequence::new(&mut fx);
        // Start screen: any press opens the picker.
        seq.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, &mut fx);
        assert_eq!(seq.st[0], SeqSlot::Choosing);
        // Left/right swap the highlight; middle readies.
        seq.pad_event(PadEvent::TapSwitch { player: 1, idx: 2 }, &mut fx);
        assert_eq!(seq.highlighted[0], 1);
        seq.pad_event(PadEvent::TapSwitch { player: 1, idx: 0 }, &mut fx);
        assert_eq!(seq.highlighted[0], 0);
        seq.pad_event(PadEvent::TapSwitch { player: 1, idx: 2 }, &mut fx);
        seq.pad_event(PadEvent::TapSwitch { player: 1, idx: 1 }, &mut fx);
        assert_eq!(seq.st[0], SeqSlot::Ready);
        // Any press while ready reopens the picker (highlight kept).
        seq.pad_event(PadEvent::TapMove { player: 1, slot: 3 }, &mut fx);
        assert_eq!(seq.st[0], SeqSlot::Choosing);
        assert_eq!(seq.highlighted[0], 1);
        seq.pad_event(PadEvent::TapSwitch { player: 1, idx: 1 }, &mut fx);
        // P2 goes through the same flow.
        assert!(!seq.tick(100, &mut fx), "p2 not ready yet");
        seq.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, &mut fx);
        seq.pad_event(PadEvent::TapSwitch { player: 2, idx: 1 }, &mut fx);
        // Both ready: 1s grace, resettable by an unready.
        assert!(!seq.tick(1000, &mut fx));
        assert!(!seq.tick(1000 + UNREADY_GRACE_MS - 1, &mut fx));
        seq.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, &mut fx); // unready
        assert!(!seq.tick(1000 + UNREADY_GRACE_MS + 50, &mut fx), "grace reset");
        seq.pad_event(PadEvent::TapSwitch { player: 2, idx: 1 }, &mut fx);
        assert!(!seq.tick(5000, &mut fx));
        assert!(seq.tick(5000 + UNREADY_GRACE_MS, &mut fx));
        let (ai, modes) = seq.take();
        assert_eq!(ai, [false, false]);
        assert_eq!(modes, [ControlMode::Concealed, ControlMode::Normal]);
    }

    #[test]
    fn ready_sequence_long_press_grants_ai_opponent() {
        let mut fx = Vec::new();
        let mut seq = ReadySequence::new(&mut fx);
        // Hold from the start screen: opponent becomes AI (ready), presser
        // proceeds to the picker.
        seq.pad_event(PadEvent::HoldMove { player: 1, slot: 0 }, &mut fx);
        assert!(seq.ai[1]);
        assert_eq!(seq.st[1], SeqSlot::Ready);
        assert_eq!(seq.st[0], SeqSlot::Choosing);
        // A press on the AI side reclaims it for a human.
        seq.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, &mut fx);
        assert!(!seq.ai[1]);
        assert_eq!(seq.st[1], SeqSlot::Choosing);
    }

    #[test]
    fn ready_sequence_ai_preset_completes_instantly() {
        let mut fx = Vec::new();
        let mut seq = ReadySequence::new(&mut fx);
        seq.ai_preset([true, true], &mut fx);
        assert!(seq.tick(0, &mut fx), "AI vs AI needs no grace");
        let (ai, _) = seq.take();
        assert_eq!(ai, [true, true]);
    }

    #[test]
    fn ready_sequence_chord_toggles_6v6() {
        let mut fx = Vec::new();
        let mut seq = ReadySequence::new(&mut fx);
        assert!(!seq.six_v_six());
        // The hidden 4-corner chord arms 6v6 without disturbing state.
        seq.pad_event(PadEvent::Chord4 { player: 1 }, &mut fx);
        assert!(seq.six_v_six());
        assert_eq!(seq.st[0], SeqSlot::Idle, "chord must not engage the picker");
        // Works via the sim grammar too, from either player, and toggles.
        seq.typed_line(":press p2 all", &mut fx);
        assert!(!seq.six_v_six());
        seq.typed_line(":press p2 all", &mut fx);
        assert!(seq.six_v_six());
        // The flash restores the state screens after ~1.2s.
        fx.clear();
        assert!(!seq.tick(0, &mut fx));
        assert!(!seq.tick(1300, &mut fx));
        assert!(fx.iter().any(|e| matches!(e, Effect::Oled(OledCmd::LobbyState { .. }))));
        // Mid-battle chords are ignored by the choice collector.
        let (mut c, _) = concealed_pair(3);
        let mut cfx = Vec::new();
        c.pad_event(PadEvent::Chord4 { player: 1 }, 0, &mut cfx);
        assert!(cfx.is_empty());
        // 6v6 forces concealed controls for both players.
        let (_, modes) = seq.take();
        assert_eq!(modes, [ControlMode::Concealed; 2]);
    }

    // ── Concealed controls ───────────────────────────────────────────────

    /// A two-human collector with p1 concealed (4 moves, bench 1/2 alive).
    fn concealed_pair(seed: u64) -> (ChoiceCollector, Vec<Effect>) {
        let mut fx = Vec::new();
        let mut p1 = SlotOptions::from_prompt(&turn_prompt("p1", 4));
        p1.party_ok = [false, true, true, false, false, false];
        p1.active_slot = Some(0);
        p1.set_concealed(seed);
        let c = ChoiceCollector::new(
            alloc::vec![p1, SlotOptions::from_prompt(&turn_prompt("p2", 4))],
            &mut fx,
        );
        (c, fx)
    }

    #[test]
    fn concealed_scatter_is_duplicate_free() {
        for seed in 0..50u64 {
            let (c, _) = concealed_pair(seed);
            let s = &c.slots[0];
            assert_ne!(s.attack_pos, s.switch_pos, "actions on distinct buttons");
            assert!(s.attack_pos < 3 && s.switch_pos < 3);
            // 4 moves scatter across all 4 corners without duplicates.
            let mut seen = [false; 4];
            for &m in &s.move_map {
                assert!((0..4).contains(&m), "seed {seed}: bad slot {m}");
                assert!(!seen[m as usize], "seed {seed}: duplicate move on corners");
                seen[m as usize] = true;
            }
            // Menu = the two alive benched mons + the active, no duplicates.
            let mut b = s.bench.clone();
            b.sort_unstable();
            assert_eq!(b, alloc::vec![0u8, 1, 2]);
        }
    }

    #[test]
    fn concealed_hold_opens_menu_and_tap_commits() {
        let (mut c, _) = concealed_pair(7);
        let mut fx = Vec::new();
        let atk = c.slots[0].attack_pos;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: atk }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedMoves { player: 1, .. })));
        // Pick the corner mapped to real move slot 2.
        let corner = c.slots[0].move_map.iter().position(|&m| m == 2).unwrap() as u8;
        c.pad_event(PadEvent::TapMove { player: 1, slot: corner }, 10, &mut fx);
        assert_eq!(c.out[0], "move 2");
        // Releasing the action button afterwards is inert (committed).
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 20, &mut fx);
        assert_eq!(c.out[0], "move 2");
    }

    #[test]
    fn concealed_release_returns_to_action_select() {
        let (mut c, _) = concealed_pair(9);
        let mut fx = Vec::new();
        let atk = c.slots[0].attack_pos;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: atk }, 0, &mut fx);
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 100, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowActionSelect { player: 1, .. })));
        assert_eq!(c.st[0], SlotState::Choosing);
        // Dead bottom button opens nothing.
        let dead = (0..3u8)
            .find(|&p| p != c.slots[0].attack_pos && p != c.slots[0].switch_pos)
            .unwrap();
        fx.clear();
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: dead }, 200, &mut fx);
        assert!(fx.is_empty());
        // Taps (not holds) on action buttons are inert too.
        c.pad_event(PadEvent::TapSwitch { player: 1, idx: atk }, 300, &mut fx);
        assert!(c.out[0].is_empty());
    }

    #[test]
    fn concealed_nested_move_detail_unwinds_by_level() {
        let (mut c, _) = concealed_pair(11);
        let mut fx = Vec::new();
        let atk = c.slots[0].attack_pos;
        let corner = c.slots[0].move_map.iter().position(|&m| m == 1).unwrap() as u8;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: atk }, 0, &mut fx);
        fx.clear();
        c.pad_event(PadEvent::HoldMove { player: 1, slot: corner }, 600, &mut fx);
        // Detail shows the REAL move slot.
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowMoveDetail { player: 1, slot: 1 })));
        // Releasing the corner returns to the move menu…
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 700, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedMoves { .. })));
        // …and releasing the action button returns to action select.
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 800, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowActionSelect { .. })));
    }

    #[test]
    fn concealed_switch_menu_commits_team_index() {
        let (mut c, _) = concealed_pair(13);
        let mut fx = Vec::new();
        let sw = c.slots[0].switch_pos;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: sw }, 0, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedSwitch { player: 1, .. })));
        let map = c.c_switch_map[0];
        let corner = map.iter().position(|&m| m == 2).unwrap() as u8;
        c.pad_event(PadEvent::TapMove { player: 1, slot: corner }, 10, &mut fx);
        assert_eq!(c.out[0], "switch 2");
    }

    #[test]
    fn concealed_forced_switch_shows_menu_directly() {
        let mut fx = Vec::new();
        let mut p1 = SlotOptions::from_prompt(&switch_prompt("p1"));
        p1.party_ok = [false, true, true, false, false, false];
        p1.set_concealed(3);
        let mut c = ChoiceCollector::new(alloc::vec![p1], &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedSwitch { player: 1, .. })));
        // No hold underneath: releases must NOT exit the menu.
        fx.clear();
        c.pad_event(PadEvent::HoldEnd { player: 1 }, 10, &mut fx);
        assert!(matches!(c.st[0], SlotState::CMenu { held: false, .. }));
        let map = c.c_switch_map[0];
        let corner = map.iter().position(|&m| m >= 0).unwrap() as u8;
        let real = map[corner as usize];
        c.pad_event(PadEvent::TapMove { player: 1, slot: corner }, 20, &mut fx);
        assert_eq!(c.out[0], alloc::format!("switch {real}"));
        // Unready returns to the SAME menu, same layout.
        fx.clear();
        c.pad_event(PadEvent::TapMove { player: 1, slot: corner }, 30, &mut fx);
        assert!(c.out[0].is_empty());
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedSwitch { .. })));
        assert_eq!(c.c_switch_map[0], map, "layout fixed within the turn");
    }

    #[test]
    fn concealed_unready_returns_to_action_select_same_layout() {
        let (mut c, _) = concealed_pair(21);
        let mut fx = Vec::new();
        let atk = c.slots[0].attack_pos;
        let map_before = c.slots[0].move_map;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: atk }, 0, &mut fx);
        let corner = c.slots[0].move_map.iter().position(|&m| m == 0).unwrap() as u8;
        c.pad_event(PadEvent::TapMove { player: 1, slot: corner }, 10, &mut fx);
        assert_eq!(c.out[0], "move 0");
        fx.clear();
        c.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, 20, &mut fx);
        assert!(c.out[0].is_empty(), "unreadied");
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowActionSelect { player: 1, .. })));
        assert_eq!(c.slots[0].move_map, map_before, "layout fixed within the turn");
    }

    #[test]
    fn concealed_dead_corner_is_inert_and_no_pp_flashes_invalid() {
        let mut fx = Vec::new();
        let mut prompt = turn_prompt("p1", 2); // 2 moves → 2 dead corners
        if let Request::Turn(t) = &mut prompt.request {
            t.active[0].moves[1].pp = 0; // second move unusable
        }
        let mut p1 = SlotOptions::from_prompt(&prompt);
        p1.set_concealed(5);
        let mut c = ChoiceCollector::new(
            alloc::vec![p1, SlotOptions::from_prompt(&turn_prompt("p2", 4))],
            &mut fx,
        );
        let atk = c.slots[0].attack_pos;
        c.pad_event(PadEvent::HoldSwitch { player: 1, idx: atk }, 0, &mut fx);
        let dead = c.slots[0].move_map.iter().position(|&m| m < 0).unwrap() as u8;
        fx.clear();
        c.pad_event(PadEvent::TapMove { player: 1, slot: dead }, 10, &mut fx);
        assert!(fx.is_empty(), "dead corner must be inert");
        assert!(c.out[0].is_empty());
        // The 0-PP move flashes invalid and restores to the MENU.
        let no_pp = c.slots[0].move_map.iter().position(|&m| m == 1).unwrap() as u8;
        c.pad_event(PadEvent::TapMove { player: 1, slot: no_pp }, 20, &mut fx);
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowInvalidSelection { player: 1 })));
        fx.clear();
        assert!(!c.tick(20 + INVALID_FLASH_MS, &mut fx));
        assert!(has_oled(&fx, |o| matches!(o, OledCmd::ShowConcealedMoves { .. })));
        assert!(matches!(c.st[0], SlotState::CMenu { .. }));
    }

    #[test]
    fn ai_vs_ai_waits_one_second_in_parallel() {
        let mut fx = Vec::new();
        let mut p1 = SlotOptions::from_prompt(&turn_prompt("p1", 4));
        let mut p2 = SlotOptions::from_prompt(&turn_prompt("p2", 4));
        p1.set_ai_choice(String::from("move 0"));
        p2.set_ai_choice(String::from("move 1"));
        let mut c = ChoiceCollector::new(alloc::vec![p1, p2], &mut fx);
        // First tick arms BOTH think deadlines (parallel, not serial).
        assert!(!c.tick(100, &mut fx), "AI is still thinking");
        assert!(!c.tick(100 + AI_THINK_MS - 1, &mut fx));
        // Both commit on the same tick: 1× the delay total, then no grace
        // window since nobody can unready.
        assert!(c.tick(100 + AI_THINK_MS, &mut fx), "both AIs commit together");
        let ch = c.take_choices();
        assert_eq!(ch[0], (String::from("p1"), String::from("move 0")));
        assert_eq!(ch[1], (String::from("p2"), String::from("move 1")));
    }

    #[test]
    fn ai_thinking_ignores_input_and_forced_human_still_instant() {
        let mut fx = Vec::new();
        let mut p2 = SlotOptions::from_prompt(&turn_prompt("p2", 4));
        p2.set_ai_choice(String::from("move 1"));
        let mut c = ChoiceCollector::new(
            alloc::vec![SlotOptions::from_prompt(&turn_prompt("p1", 4)), p2],
            &mut fx,
        );
        c.tick(0, &mut fx); // arm the AI deadline
        // Buttons and typed lines can't steer or unready a thinking AI.
        c.pad_event(PadEvent::TapMove { player: 2, slot: 3 }, 10, &mut fx);
        assert!(c.out[1].is_empty(), "pad input must not commit for the AI");
        fx.clear();
        c.typed_line("p2 3", 10, &mut fx);
        assert!(err_containing(&fx, "forced this turn"));
        // Human commits while the AI thinks; AI lands at its deadline.
        c.typed_line("p1 1", 20, &mut fx);
        assert_eq!(c.out[0], "move 0");
        assert!(!c.tick(AI_THINK_MS - 1, &mut fx));
        assert!(!c.tick(AI_THINK_MS, &mut fx), "grace window still applies (human can unready)");
        assert_eq!(c.out[1], "move 1", "AI committed at its deadline");
        assert!(c.tick(AI_THINK_MS + UNREADY_GRACE_MS, &mut fx));
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
