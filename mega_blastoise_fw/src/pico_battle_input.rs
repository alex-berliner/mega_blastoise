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
    format_move_choice, format_switch_choice, join_choice_parts, party_slot_from_mon, player_id_to_num, ActivePrompt,
    ButtonSource, InputBus, InputSource, PlayerAction,
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
    /// selecting) and mid-turn switching. Returns each player's choice string in
    /// the same order as `players`.
    pub async fn wait_two_turns(&mut self, players: [PlayerTurn; 2]) -> [String; 2] {
        const LONG_MS: u64 = 500;
        let mut st = [TwoState::Wait; 2];
        let mut out: [String; 2] = [String::new(), String::new()];

        // Players with a forced choice (locked move / no moves) need no input.
        for i in 0..2 {
            if let Some(c) = &players[i].auto {
                out[i] = c.clone();
                st[i] = TwoState::Done;
            }
        }

        loop {
            if matches!(st[0], TwoState::Done) && matches!(st[1], TwoState::Done) {
                return out;
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
                        // Switch row takes priority, mirroring wait_action.
                        let mut moved = false;
                        if pt.can_switch() {
                            if let Some(c) = (0..3).find(|&c| pressed[pt.switch_row][c]) {
                                st[i] = TwoState::Hold { row: pt.switch_row, col: c, switch: true, t0: now };
                                moved = true;
                            }
                        }
                        if !moved && pt.n_moves > 0 {
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
                                    oled_send(OledCmd::ShowPokemonStats { player: pt.player_num, team_idx: col as u8 });
                                } else {
                                    oled_send(OledCmd::ShowMoveDetail { player: pt.player_num, slot: col as u8 });
                                }
                                st[i] = TwoState::Shown { row };
                            }
                        } else if now.saturating_sub(t0) < LONG_MS {
                            // Short press → a selection (if valid).
                            if switch {
                                out[i] = format_switch_choice(col);
                                st[i] = TwoState::Done;
                            } else if pt.usable[col] {
                                out[i] = format_move_choice(col);
                                st[i] = TwoState::Done;
                            } else {
                                st[i] = TwoState::Wait; // disabled / no PP — ignore
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
                }
            }

            Timer::after_millis(6).await;
        }
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
    /// `Request::Switch` — only a party button is valid (forced switch).
    must_switch: bool,
    /// Forced choice (locked move, no moves, team preview) — submit without input.
    auto: Option<String>,
}

impl PlayerTurn {
    /// Distil a player's `Request` into the options the matrix collector needs.
    pub fn from_request(player_id: &str, request: &Request) -> Self {
        let p2 = player_id == "p2";
        let mut pt = Self {
            player_num: if p2 { 2 } else { 1 },
            move_row: if p2 { 2 } else { 0 },
            switch_row: if p2 { 3 } else { 1 },
            n_moves: 0,
            usable: [false; 4],
            trapped: false,
            must_switch: false,
            auto: None,
        };
        match request {
            Request::Turn(turn) => {
                if let Some(mon) = turn.active.first() {
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
            Request::Switch(_) => pt.must_switch = true,
            Request::TeamPreview(_) => pt.auto = Some(String::from("random")),
            Request::LearnMove(_) => pt.auto = Some(String::from("pass")),
        }
        pt
    }

    fn can_switch(&self) -> bool {
        self.must_switch || !self.trapped
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
                    { oled_send(OledCmd::ShowPokemonStats { player, team_idx: col as u8 });
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
                    { oled_send(OledCmd::ShowPokemonStats { player, team_idx: col as u8 });
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
            bus.choices.send(choice).await;
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
                    loop {
                        match self.wait_action(player_id, n).await {
                            PlayerAction::Move(slot) if slot < n => {
                                if !mon_req.moves[slot].disabled && mon_req.moves[slot].pp > 0 {
                                    parts.push(format_move_choice(slot));
                                    break;
                                }
                            }
                            PlayerAction::Switch(idx) if !mon_req.trapped => {
                                parts.push(format_switch_choice(idx));
                                break;
                            }
                            _ => {}
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
