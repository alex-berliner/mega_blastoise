//! 4×4 button matrix driver.
//!
//! Physical mapping (per ELECTRONICS.md):
//! - Row 0 (GP6):  P1 move buttons 1–4  → cols 0–3
//! - Row 1 (GP7):  P1 party buttons 1–3 → cols 0–2  (col 3 unused)
//! - Row 2 (GP8):  P2 move buttons 1–4  → cols 0–3
//! - Row 3 (GP9):  P2 party buttons 1–3 → cols 0–2  (col 3 unused)
//!
//! Row pins (GP6–9) are `Output`s, driven LOW one at a time during scan.
//! Col pins (GP10–13) are `Input`s with internal pull-ups; LOW = pressed.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::Request;
use cortex_m::asm::delay as asm_delay;
use embassy_rp::gpio::{Input, Output};
use embassy_time::Timer;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus,
    InputSource,
};

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

    /// Wait for a move button for `player_id`, returning the 0-based move slot.
    /// Retries silently if `is_usable(col)` returns false (disabled / no PP).
    pub async fn wait_move<F>(&mut self, player_id: &str, n: usize, is_usable: F) -> usize
    where
        F: Fn(usize) -> bool,
    {
        let row = if player_id == "p2" { 2 } else { 0 };
        loop {
            let col = self.wait_press(row, n).await;
            if is_usable(col) { return col; }
        }
    }

    /// Wait for a party switch button for `player_id`.
    /// Returns a 0-based party index (cols 0–2 → party slots 0–2).
    pub async fn wait_switch(&mut self, player_id: &str) -> usize {
        let row = if player_id == "p2" { 3 } else { 1 };
        self.wait_press(row, 3).await
    }
}

/// Thin wrapper around [`ButtonMatrix`] that implements [`InputSource`].
///
/// In the full game `BattleController` races this against USB serial.
/// For button-only operation pass `.run(&bus)` directly to `run_battle`.
pub struct PicoBattleInput<'d>(pub ButtonMatrix<'d>);

impl<'d> PicoBattleInput<'d> {
    pub fn new(rows: [Output<'d>; 4], cols: [Input<'d>; 4]) -> Self {
        Self(ButtonMatrix::new(rows, cols))
    }

    pub async fn wait_move<F>(&mut self, player_id: &str, n: usize, is_usable: F) -> usize
    where
        F: Fn(usize) -> bool,
    {
        self.0.wait_move(player_id, n, is_usable).await
    }

    pub async fn wait_switch(&mut self, player_id: &str) -> usize {
        self.0.wait_switch(player_id).await
    }
}

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
                    let idx = self
                        .wait_move(player_id, n, |i| {
                            !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0
                        })
                        .await;
                    parts.push(format_move_choice(idx));
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
