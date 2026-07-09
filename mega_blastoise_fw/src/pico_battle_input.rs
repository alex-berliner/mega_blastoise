//! 4×4 button matrix driver.
//!
//! Physical mapping (per ELECTRONICS.md):
//! - Row 0 (GP5):  P1 move buttons 1–4  → cols 0–3
//! - Row 1 (GP7):  P1 party buttons 1–3 → cols 0–2  (col 3 unused)
//! - Row 2 (GP8):  P2 move buttons 1–4  → cols 0–3
//! - Row 3 (GP9):  P2 party buttons 1–3 → cols 0–2  (col 3 unused)
//!
//! Row pins (GP5,7,8,9) are `Output`s, driven LOW one at a time during scan.
//! Col pins (GP10–13) are `Input`s with internal pull-ups; LOW = pressed.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use gen1_battle::{PlayerBattleData, Request};
use cortex_m::asm::delay as asm_delay;
use embassy_rp::gpio::{Input, Output};
use embassy_time::{Instant, Timer};
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, party_slot_from_mon, player_id_to_num,
    turn_action_choice, ActionReject, ActivePrompt, ButtonSource, InputBus, InputSource,
    PlayerAction, PlayerChoice,
};
#[cfg(feature = "oled")]
use crate::subsystems::oled::{send as oled_send, OledCmd};

pub struct ButtonMatrix<'d> {
    rows: [Output<'d>; 4],
    cols: [Input<'d>; 4],
}

impl<'d> ButtonMatrix<'d> {
    pub fn new(rows: [Output<'d>; 4], cols: [Input<'d>; 4]) -> Self {
        Self { rows, cols }
    }

    /// Drive `row` LOW, settle ~12 µs, read all four cols, drive row HIGH again.
    /// Returns a bitmask: bit n = col n is pressed (active LOW).
    fn scan_row(&mut self, row: usize) -> u8 {
        self.rows[row].set_low();
        asm_delay(1500); // ≈ 12 µs settle at 125 MHz
        let mut mask = 0u8;
        for col in 0..4 {
            if self.cols[col].is_low() {
                mask |= 1 << col;
            }
        }
        self.rows[row].set_high();
        mask
    }

    /// Wait for exactly one button press in `row`, only accepting cols 0..max_cols.
    /// Polls at 5 ms intervals; waits for key-up before returning (simple debounce).
    pub async fn wait_press(&mut self, row: usize, max_cols: usize) -> usize {
        let col_mask = (1u8 << max_cols) - 1;
        loop {
            let pressed = self.scan_row(row) & col_mask;
            if let Some(col) = (0..max_cols).find(|&c| pressed & (1 << c) != 0) {
                loop {
                    Timer::after_millis(10).await;
                    if self.scan_row(row) & col_mask == 0 { break; }
                }
                return col;
            }
            Timer::after_millis(5).await;
        }
    }

    /// Wait for a party switch button for `player_id`.
    /// Returns a 0-based party index (cols 0–2 → party slots 0–2).
    pub async fn wait_switch(&mut self, player_id: &str) -> usize {
        let row = if player_id == "p2" { 3 } else { 1 };
        self.wait_press(row, 3).await
    }

    /// Wait for all buttons in `row` to be released (used after long-press detection).
    pub async fn wait_release(&mut self, row: usize) {
        loop {
            Timer::after_millis(10).await;
            if self.scan_row(row) == 0 { break; }
        }
    }

    /// Wait for a button press from a specific player (rows 0+1 = P1, rows 2+3 = P2).
    /// Short press (< 500 ms) → P1/P2; long press (≥ 500 ms) → P1Long/P2Long.
    /// Any press performs unready in the lobby; long press selects AI opponent.
    pub async fn wait_lobby_press(&mut self) -> LobbyPress {
        loop {
            let p1 = self.scan_row(0) | self.scan_row(1);
            let p2 = self.scan_row(2) | self.scan_row(3);
            if p1 != 0 {
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if self.scan_row(0) | self.scan_row(1) == 0 { break false; }
                    if held_ms >= 500 { break true; }
                };
                if is_long {
                    loop {
                        Timer::after_millis(10).await;
                        if self.scan_row(0) | self.scan_row(1) == 0 { break; }
                    }
                    return LobbyPress::P1Long;
                } else {
                    return LobbyPress::P1;
                }
            }
            if p2 != 0 {
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if self.scan_row(2) | self.scan_row(3) == 0 { break false; }
                    if held_ms >= 500 { break true; }
                };
                if is_long {
                    loop {
                        Timer::after_millis(10).await;
                        if self.scan_row(2) | self.scan_row(3) == 0 { break; }
                    }
                    return LobbyPress::P2Long;
                } else {
                    return LobbyPress::P2;
                }
            }
            Timer::after_millis(5).await;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LobbyPress { P1, P2, P1Long, P2Long }

/// Thin wrapper around [`ButtonMatrix`] that implements [`InputSource`].
///
/// In the full game `BattleController` races this against USB serial.
/// For button-only operation pass `.run(&bus)` directly to `run_battle`.
pub struct PicoBattleInput<'d>(pub ButtonMatrix<'d>);

impl<'d> PicoBattleInput<'d> {
    pub fn new(rows: [Output<'d>; 4], cols: [Input<'d>; 4]) -> Self {
        Self(ButtonMatrix::new(rows, cols))
    }

    pub async fn wait_switch(&mut self, player_id: &str) -> usize {
        self.0.wait_switch(player_id).await
    }

    pub async fn wait_lobby_press(&mut self) -> LobbyPress {
        self.0.wait_lobby_press().await
    }

    /// Collect *both* players' choices at the same time, scanning the whole
    /// matrix in one loop so neither board blocks the other. Handles long-press
    /// (≥500 ms shows move detail / party stats on that player's OLED instead of
    /// selecting) and mid-turn switching. Completes when both players are done;
    /// read the choices out of `progress`.
    ///
    /// All state lives in `progress`, so this future can be dropped (e.g. losing
    /// a `select` race against USB input) and re-entered without forgetting a
    /// choice a player already committed.
    pub async fn wait_two_turns(&mut self, players: &[PlayerTurn; 2], progress: &mut TwoTurnProgress) {
        const LONG_MS: u64 = 500;
        let st = &mut progress.st;
        let out = &mut progress.out;

        loop {
            if matches!(st[0], TwoState::Done) && matches!(st[1], TwoState::Done) {
                return;
            }

            // One full matrix scan, shared by both players.
            let mut pressed = [[false; 4]; 4];
            for r in 0..4 {
                let m = self.0.scan_row(r);
                for c in 0..4 {
                    pressed[r][c] = m & (1 << c) != 0;
                }
            }
            let now = Instant::now().as_millis();

            for i in 0..2 {
                let pt = &players[i];
                match st[i] {
                    TwoState::Done => {}
                    TwoState::Wait => {
                        // Switch row takes priority. Always enter Hold on a party press
                        // so a long-press can show stats even for a trapped/active mon;
                        // the actual selection is validated on release.
                        if let Some(c) = (0..3).find(|&c| pressed[pt.switch_row][c]) {
                            st[i] = TwoState::Hold { row: pt.switch_row, col: c, switch: true, t0: now };
                        } else if pt.n_moves > 0 {
                            if let Some(c) = (0..pt.n_moves).find(|&c| pressed[pt.move_row][c]) {
                                st[i] = TwoState::Hold { row: pt.move_row, col: c, switch: false, t0: now };
                            }
                        }
                    }
                    TwoState::Hold { row, col, switch, t0 } => {
                        if pressed[row][col] {
                            // Still held — promote to a detail view at the threshold.
                            if now.saturating_sub(t0) >= LONG_MS {
                                #[cfg(feature = "oled")]
                                if switch {
                                    oled_send(OledCmd::ShowPokemonStats { player: pt.player_num, team_idx: col as u8, page: 0 });
                                } else {
                                    oled_send(OledCmd::ShowMoveDetail { player: pt.player_num, slot: col as u8 });
                                }
                                st[i] = TwoState::Shown { row };
                            }
                        } else if now.saturating_sub(t0) < LONG_MS {
                            // Short press → a selection, validated by the shared rule
                            // (handles disabled/PP, trapped, and switch-to-active),
                            // plus the fainted-bench check.
                            let action = if switch {
                                PlayerAction::Switch(col)
                            } else {
                                PlayerAction::Move(col)
                            };
                            let verdict = if switch && !pt.party_ok[col.min(5)] {
                                Err(())
                            } else {
                                turn_action_choice(&action, pt.n_moves, &pt.usable, pt.trapped, pt.active_slot).map_err(|_| ())
                            };
                            match verdict {
                                Ok(choice) => {
                                    out[i] = choice;
                                    st[i] = TwoState::Done;
                                }
                                Err(()) => {
                                    // Flash the invalid-selection screen briefly.
                                    #[cfg(feature = "oled")]
                                    oled_send(OledCmd::ShowInvalidSelection { player: pt.player_num });
                                    st[i] = TwoState::Invalid { until: now + 600 };
                                }
                            }
                        } else {
                            st[i] = TwoState::Wait;
                        }
                    }
                    TwoState::Shown { row } => {
                        // Detail view stays up until the button is released.
                        if !(0..4).any(|c| pressed[row][c]) {
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::RestoreScreen { player: pt.player_num });
                            st[i] = TwoState::Wait;
                        }
                    }
                    TwoState::Invalid { until } => {
                        if now >= until {
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::RestoreScreen { player: pt.player_num });
                            st[i] = TwoState::Wait;
                        }
                    }
                }
            }

            Timer::after_millis(6).await;
        }
    }
}

/// State of a two-player parallel wait, held by the caller so the
/// [`PicoBattleInput::wait_two_turns`] future can be raced against other input
/// (USB lines) and re-entered without losing a committed choice.
pub struct TwoTurnProgress {
    st: [TwoState; 2],
    out: [String; 2],
}

impl TwoTurnProgress {
    pub fn new(players: &[PlayerTurn; 2]) -> Self {
        let mut p = Self { st: [TwoState::Wait; 2], out: [String::new(), String::new()] };
        // Players with a forced choice (locked move / no moves) need no input.
        for i in 0..2 {
            if let Some(c) = &players[i].auto {
                p.out[i] = c.clone();
                p.st[i] = TwoState::Done;
            }
        }
        p
    }

    pub fn is_done(&self, i: usize) -> bool {
        matches!(self.st[i], TwoState::Done)
    }

    pub fn all_done(&self) -> bool {
        self.is_done(0) && self.is_done(1)
    }

    /// Commit an externally validated choice (e.g. a typed USB line) for
    /// player `i`. Dismisses that player's long-press detail view if showing.
    pub fn set_choice(&mut self, i: usize, players: &[PlayerTurn; 2], choice: String) {
        #[cfg(feature = "oled")]
        if matches!(self.st[i], TwoState::Shown { .. }) {
            oled_send(OledCmd::RestoreScreen { player: players[i].player_num });
        }
        #[cfg(not(feature = "oled"))]
        let _ = players;
        self.out[i] = choice;
        self.st[i] = TwoState::Done;
    }

    pub fn into_choices(self) -> [String; 2] {
        self.out
    }
}

/// One player's options for a single decision point, distilled from their
/// `Request` so the parallel collector doesn't need the gen1 battle types.
pub struct PlayerTurn {
    /// 1 or 2 — used to target the right OLED for long-press detail views.
    player_num: u8,
    /// Matrix row scanned for this player's move buttons.
    move_row: usize,
    /// Matrix row scanned for this player's party buttons.
    switch_row: usize,
    /// Number of move buttons that are live (0 = no move input this turn).
    n_moves: usize,
    /// Per-move usability (not disabled and has PP).
    usable: [bool; 4],
    /// Active mon can't switch out.
    trapped: bool,
    /// Team index of the active mon (so switching to it is rejected).
    active_slot: Option<usize>,
    /// Per-team-slot switch validity (alive and not active). All-true when no
    /// player data was attached — the engine still validates.
    party_ok: [bool; 6],
    /// This is a forced replacement after a faint (Request::Switch).
    forced_switch: bool,
    /// Forced choice (locked move, no moves, team preview) — submit without input.
    auto: Option<String>,
}

impl PlayerTurn {
    /// Distil a player's `Request` into the options the matrix collector needs.
    pub fn from_request(player_id: &str, request: &Request, player_data: Option<&PlayerBattleData>) -> Self {
        let p2 = player_id == "p2";
        let mut pt = Self {
            player_num: if p2 { 2 } else { 1 },
            move_row: if p2 { 2 } else { 0 },
            switch_row: if p2 { 3 } else { 1 },
            n_moves: 0,
            usable: [false; 4],
            trapped: false,
            active_slot: None,
            party_ok: [true; 6],
            forced_switch: false,
            auto: None,
        };
        if let Some(pd) = player_data {
            for (i, ok) in pt.party_ok.iter_mut().enumerate() {
                *ok = pd.mons.get(i).is_some_and(|m| !m.active && m.hp > 0);
            }
        }
        match request {
            Request::Turn(turn) => {
                if let Some(mon) = turn.active.first() {
                    pt.active_slot = Some(mon.team_position as usize);
                    let n = mon.moves.len().min(4);
                    if n == 0 {
                        pt.auto = Some(String::from("pass"));
                    } else if mon.locked_into_move {
                        pt.auto = Some(format_move_choice(0));
                    } else {
                        pt.n_moves = n;
                        for i in 0..n {
                            pt.usable[i] = !mon.moves[i].disabled && mon.moves[i].pp > 0;
                        }
                        pt.trapped = mon.trapped;
                    }
                }
            }
            Request::Switch(_) => pt.forced_switch = true, // no moves, pick a bench mon
            Request::TeamPreview(_) => pt.auto = Some(String::from("random")),
            Request::LearnMove(_) => pt.auto = Some(String::from("pass")),
        }
        pt
    }

    /// Number of live move buttons this turn (for parsing typed move slots).
    pub fn n_moves(&self) -> usize {
        self.n_moves
    }

    /// This prompt is a forced replacement after a faint.
    pub fn forced_switch(&self) -> bool {
        self.forced_switch
    }

    /// Whether team slot `i` is a legal switch target (alive and benched).
    pub fn switch_target_ok(&self, i: usize) -> bool {
        self.party_ok.get(i).copied().unwrap_or(false)
    }

    /// True when team slot `i` is not the currently active slot.
    pub fn active_slot_is_not(&self, i: usize) -> bool {
        self.active_slot != Some(i)
    }

    /// Validate a typed action through the same shared rule as button presses
    /// (disabled/PP, trapped, switch-to-active).
    pub fn typed_action_choice(&self, action: &PlayerAction) -> Result<String, ActionReject> {
        turn_action_choice(action, self.n_moves, &self.usable, self.trapped, self.active_slot)
    }
}

/// Per-player progress inside [`PicoBattleInput::wait_two_turns`].
#[derive(Clone, Copy)]
enum TwoState {
    /// No button down yet.
    Wait,
    /// A button is held; timing it to tell a tap from a long-press.
    Hold { row: usize, col: usize, switch: bool, t0: u64 },
    /// Long-press detail view is showing; waiting for release.
    Shown { row: usize },
    /// Invalid selection screen is showing; restores at the deadline.
    Invalid { until: u64 },
    /// Choice committed.
    Done,
}

/// [`ButtonSource`] — scan the GPIO matrix and return whichever button fires first.
/// All battle-protocol logic (disabled moves, trapped, PP) lives in [`ButtonController`].
impl ButtonSource for PicoBattleInput<'_> {
    fn on_prompt(
        &mut self,
        player_id: &str,
        _request: &Request,
        player_data: &Option<PlayerBattleData>,
    ) {
        #[cfg(feature = "oled")]
        if let Some(pd) = player_data {
            let player = player_id_to_num(player_id);
            let slots = pd.mons.iter().map(party_slot_from_mon).collect();
            oled_send(OledCmd::PartyUpdate { player, slots });
        }
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        let move_row   = if player_id == "p2" { 2 } else { 0 };
        let switch_row = if player_id == "p2" { 3 } else { 1 };
        let player     = player_id_to_num(player_id);
        let move_mask  = (1u8 << n_moves) - 1;
        loop {
            // Switch buttons — long press shows party stats; short press selects.
            let s = self.0.scan_row(switch_row) & 0b0000_0111;
            if let Some(col) = (0..3usize).find(|&c| s & (1 << c) != 0) {
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if self.0.scan_row(switch_row) & (1 << col) == 0 { break false; }
                    if held_ms >= 500 { break true; }
                };
                if is_long {
                    #[cfg(feature = "oled")]
                    { oled_send(OledCmd::ShowPokemonStats { player, team_idx: col as u8, page: 0 });
                      self.0.wait_release(switch_row).await;
                      oled_send(OledCmd::RestoreScreen { player }); }
                    #[cfg(not(feature = "oled"))]
                    self.0.wait_release(switch_row).await;
                } else {
                    return PlayerAction::Switch(col);
                }
            }

            // Move buttons — detect long press (≥500 ms) vs short press.
            let m = self.0.scan_row(move_row) & move_mask;
            if let Some(col) = (0..n_moves).find(|&c| m & (1 << c) != 0) {
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if self.0.scan_row(move_row) & (1 << col) == 0 {
                        break false; // released before threshold
                    }
                    if held_ms >= 500 {
                        break true; // threshold crossed, still held
                    }
                };
                if is_long {
                    #[cfg(feature = "oled")]
                    { oled_send(OledCmd::ShowMoveDetail { player, slot: col as u8 });
                      self.0.wait_release(move_row).await;
                      oled_send(OledCmd::RestoreScreen { player }); }
                    #[cfg(not(feature = "oled"))]
                    self.0.wait_release(move_row).await;
                    // Don't return — loop back and wait for a new press.
                } else {
                    return PlayerAction::Move(col);
                }
            }

            Timer::after_millis(5).await;
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let row    = if player_id == "p2" { 3 } else { 1 };
        let player = player_id_to_num(player_id);
        loop {
            let s = self.0.scan_row(row) & 0b0000_0111;
            if let Some(col) = (0..3usize).find(|&c| s & (1 << c) != 0) {
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if self.0.scan_row(row) & (1 << col) == 0 { break false; }
                    if held_ms >= 500 { break true; }
                };
                if is_long {
                    #[cfg(feature = "oled")]
                    { oled_send(OledCmd::ShowPokemonStats { player, team_idx: col as u8, page: 0 });
                      self.0.wait_release(row).await;
                      oled_send(OledCmd::RestoreScreen { player }); }
                    #[cfg(not(feature = "oled"))]
                    self.0.wait_release(row).await;
                } else {
                    return col;
                }
            }
            Timer::after_millis(5).await;
        }
    }
}

/// Standalone [`InputSource`] — kept for button-only operation without USB.
impl InputSource for PicoBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { player_id, request, .. } = bus.prompt.receive().await;
            let choice = self.handle_request(&player_id, &request).await;
            bus.choices.send(PlayerChoice { player_id, choice }).await;
        }
    }
}

impl PicoBattleInput<'_> {
    async fn handle_request(&mut self, player_id: &str, request: &Request) -> String {
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        parts.push(String::from("pass"));
                        continue;
                    }
                    if mon_req.locked_into_move {
                        parts.push(format_move_choice(0));
                        continue;
                    }
                    let mut usable = [false; 4];
                    for i in 0..n {
                        usable[i] = !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0;
                    }
                    let active_slot = Some(mon_req.team_position as usize);
                    loop {
                        let action = self.wait_action(player_id, n).await;
                        if let Ok(choice) = turn_action_choice(&action, n, &usable, mon_req.trapped, active_slot) {
                            parts.push(choice);
                            break;
                        }
                    }
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let mut parts = Vec::new();
                for _ in 0..sw.needs_switch.len() {
                    let idx = self.wait_switch(player_id).await;
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }
}
