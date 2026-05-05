//! Physical buttons: **4** move (slots 0–3) and **6** switch (party indices 0–5).
//! Wiring: GPIO → button → GND, internal pull-up (`Pull::Up`).
//!
//! Default GPIO mapping (change to match your wiring):
//! - Moves: **GPIO 6, 7, 8, 9** → buttons labelled move **1–4** (protocol slots **0–3**).
//! - Switch: **GPIO 10–15** → buttons **1–6** (party slots **0–5**).

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::Request;
use embassy_futures::select::{select, select3, select4, Either, Either3, Either4};
use embassy_rp::gpio::Input;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus,
    InputSource,
};

/// RP2040 GPIO button matrix for physical move/switch input.
pub struct PicoBattleInput<'d> {
    /// Move buttons → protocol `move 0` … `move 3`.  GPIO 6–9, active-low.
    pub move_pins: [Input<'d>; 4],
    /// Switch buttons → protocol `switch 0` … `switch 5`.  GPIO 10–15, active-low.
    pub switch_pins: [Input<'d>; 6],
}

impl<'d> PicoBattleInput<'d> {
    pub fn new(move_pins: [Input<'d>; 4], switch_pins: [Input<'d>; 6]) -> Self {
        Self { move_pins, switch_pins }
    }

    /// Wait for one of the first `n` move buttons to be pressed **and released**.
    /// Retries silently if `is_usable(idx)` returns false (disabled / no PP).
    pub async fn wait_move<F>(&mut self, n: usize, is_usable: F) -> usize
    where
        F: Fn(usize) -> bool,
    {
        loop {
            let idx = self.wait_any_move_press(n).await;
            if is_usable(idx) {
                return idx;
            }
        }
    }

    /// Wait for any of the 6 switch buttons to be pressed and released.
    /// Returns the 0-based party slot index.
    pub async fn wait_switch(&mut self) -> usize {
        let [s0, s1, s2, s3, s4, s5] = &mut self.switch_pins;
        let idx = match select(
            select3(s0.wait_for_low(), s1.wait_for_low(), s2.wait_for_low()),
            select3(s3.wait_for_low(), s4.wait_for_low(), s5.wait_for_low()),
        )
        .await
        {
            Either::First(e) => match e {
                Either3::First(_) => 0,
                Either3::Second(_) => 1,
                Either3::Third(_) => 2,
            },
            Either::Second(e) => match e {
                Either3::First(_) => 3,
                Either3::Second(_) => 4,
                Either3::Third(_) => 5,
            },
        };
        self.switch_pins[idx].wait_for_high().await;
        idx
    }

    async fn wait_any_move_press(&mut self, n: usize) -> usize {
        let [p0, p1, p2, p3] = &mut self.move_pins;
        match n.min(4) {
            1 => {
                p0.wait_for_low().await;
                p0.wait_for_high().await;
                0
            }
            2 => match select(p0.wait_for_low(), p1.wait_for_low()).await {
                Either::First(_) => { p0.wait_for_high().await; 0 }
                Either::Second(_) => { p1.wait_for_high().await; 1 }
            },
            3 => match select3(p0.wait_for_low(), p1.wait_for_low(), p2.wait_for_low()).await {
                Either3::First(_) => { p0.wait_for_high().await; 0 }
                Either3::Second(_) => { p1.wait_for_high().await; 1 }
                Either3::Third(_) => { p2.wait_for_high().await; 2 }
            },
            _ => match select4(
                p0.wait_for_low(),
                p1.wait_for_low(),
                p2.wait_for_low(),
                p3.wait_for_low(),
            )
            .await
            {
                Either4::First(_) => { p0.wait_for_high().await; 0 }
                Either4::Second(_) => { p1.wait_for_high().await; 1 }
                Either4::Third(_) => { p2.wait_for_high().await; 2 }
                Either4::Fourth(_) => { p3.wait_for_high().await; 3 }
            },
        }
    }
}

/// Standalone `InputSource` implementation for button-only operation (no USB display).
/// In the full game `BattleController` races buttons against USB instead of using this.
impl InputSource for PicoBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { request, .. } = bus.prompt.receive().await;
            let choice = self.handle_request(&request).await;
            bus.choices.send(choice).await;
        }
    }
}

impl<'d> PicoBattleInput<'d> {
    async fn handle_request(&mut self, request: &Request) -> String {
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
                        .wait_move(n, |i| !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0)
                        .await;
                    parts.push(format_move_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let mut parts = Vec::new();
                for _ in 0..sw.needs_switch.len() {
                    let idx = self.wait_switch().await;
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }
}
