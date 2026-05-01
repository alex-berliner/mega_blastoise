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
use embassy_rp::gpio::Input;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus,
};

fn debounced_is_low(pin: &Input) -> bool {
    if !pin.is_low() {
        return false;
    }
    for _ in 0..50_000 {
        cortex_m::asm::nop();
    }
    pin.is_low()
}

fn wait_first_pressed_slice(pins: &[Input<'_>]) -> usize {
    loop {
        for i in 0..pins.len() {
            if debounced_is_low(&pins[i]) {
                while pins[i].is_low() {
                    cortex_m::asm::nop();
                }
                return i;
            }
        }
        cortex_m::asm::nop();
    }
}

/// RP2040 button matrix for physical move/switch buttons.
pub struct PicoBattleInput<'d> {
    /// Move buttons → protocol `move 0` … `move 3` (first N used if fewer moves).
    pub move_pins: [Input<'d>; 4],
    /// Switch buttons → protocol `switch 0` … `switch 5`.
    pub switch_pins: [Input<'d>; 6],
}

impl<'d> PicoBattleInput<'d> {
    pub fn new(move_pins: [Input<'d>; 4], switch_pins: [Input<'d>; 6]) -> Self {
        Self {
            move_pins,
            switch_pins,
        }
    }
}

impl<'d> PicoBattleInput<'d> {
    pub async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { player_id, request } = bus.prompt.wait().await;
            let choice = self.handle(&player_id, &request);
            bus.choices.send(choice).await;
        }
    }

    fn handle(&mut self, player_id: &str, request: &Request) -> String {
        let _ = player_id;
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n_moves = mon_req.moves.len().min(4);
                    if n_moves == 0 {
                        parts.push(String::from("pass"));
                        continue;
                    }
                    let slice = &self.move_pins[..n_moves];
                    loop {
                        let idx = wait_first_pressed_slice(slice);
                        let m = &mon_req.moves[idx];
                        if m.disabled || m.pp == 0 {
                            continue;
                        }
                        parts.push(format_move_choice(idx));
                        break;
                    }
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let mut parts = Vec::new();
                for _ in 0..sw.needs_switch.len() {
                    let team_index = wait_first_pressed_slice(&self.switch_pins);
                    parts.push(format_switch_choice(team_index));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }
}
