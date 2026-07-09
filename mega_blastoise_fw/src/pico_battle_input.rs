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

use cortex_m::asm::delay as asm_delay;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Output};
use embassy_time::{Instant, Timer};
use mega_blastoise_core::{
    ActivePrompt, ChoiceCollector, CollectEffect, InputBus, InputSource, PadEvent, PlayerChoice,
    SlotOptions, COLLECT_TICK_MS, HOLD_THRESHOLD_MS,
};
#[cfg(feature = "oled")]
use crate::subsystems::oled::send as oled_send;

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

    pub async fn wait_lobby_press(&mut self) -> LobbyPress {
        self.0.wait_lobby_press().await
    }

    /// Next classified button event from the matrix, for either player.
    /// Classification (tap vs ≥500 ms hold) lives here; ALL semantics live in
    /// [`ChoiceCollector`]. Scan state is held in `scan`, outside this future,
    /// so losing a `select` race mid-press doesn't drop the press.
    ///
    /// While a hold is showing, a second press is timed independently: if it
    /// becomes a hold it takes over and the first button goes STALE (ignored
    /// until physically released, and its release emits no event).
    pub async fn next_pad_event(&mut self, scan: &mut PadScan) -> PadEvent {
        loop {
            let mut pressed = [0u8; 4];
            for (r, mask) in pressed.iter_mut().enumerate() {
                *mask = self.0.scan_row(r);
            }
            // Stale buttons stop being stale once released.
            for r in 0..4 {
                scan.stale[r] &= pressed[r];
            }
            let now = Instant::now().as_millis();

            for i in 0..2usize {
                let player = (i + 1) as u8;
                let (move_row, switch_row) = if i == 0 { (0usize, 1usize) } else { (2, 3) };
                let live = |row: usize, cols: u8| (pressed[row] & !scan.stale[row]) & cols;
                let is_down = |row: usize, col: usize| pressed[row] & (1 << col) != 0;
                // Party row takes priority, like the original collector.
                let fresh_press = |except: Option<(usize, usize)>| -> Option<(usize, usize, bool)> {
                    for &(row, cols, switch) in
                        &[(switch_row, 0b0111u8, true), (move_row, 0b1111u8, false)]
                    {
                        if let Some(c) = (0..4).find(|&c| {
                            live(row, cols) & (1 << c) != 0 && except != Some((row, c))
                        }) {
                            return Some((row, c, switch));
                        }
                    }
                    None
                };

                match scan.st[i] {
                    PadState::Idle => {
                        if let Some((row, col, switch)) = fresh_press(None) {
                            scan.st[i] = PadState::Held { row, col, switch, t0: now, prev: None };
                        }
                    }
                    PadState::Held { row, col, switch, t0, prev } => {
                        // If the previous hold's button was released while we
                        // time the new press, its view ends now.
                        if let Some((prow, pcol)) = prev {
                            if !is_down(prow, pcol) {
                                scan.st[i] = PadState::Held { row, col, switch, t0, prev: None };
                                return PadEvent::HoldEnd { player };
                            }
                        }
                        if is_down(row, col) {
                            if now.saturating_sub(t0) >= HOLD_THRESHOLD_MS {
                                // This hold takes over; the previous button is
                                // stale until physically released.
                                if let Some((prow, pcol)) = prev {
                                    scan.stale[prow] |= 1 << pcol;
                                }
                                scan.st[i] = PadState::HoldOut { row, col };
                                return if switch {
                                    PadEvent::HoldSwitch { player, idx: col as u8 }
                                } else {
                                    PadEvent::HoldMove { player, slot: col as u8 }
                                };
                            }
                        } else {
                            scan.st[i] = match prev {
                                Some((prow, pcol)) => PadState::HoldOut { row: prow, col: pcol },
                                None => PadState::Idle,
                            };
                            return if switch {
                                PadEvent::TapSwitch { player, idx: col as u8 }
                            } else {
                                PadEvent::TapMove { player, slot: col as u8 }
                            };
                        }
                    }
                    PadState::HoldOut { row, col } => {
                        if !is_down(row, col) {
                            scan.st[i] = PadState::Idle;
                            return PadEvent::HoldEnd { player };
                        }
                        // A second press starts timing while the view stays up.
                        if let Some((nrow, ncol, nswitch)) = fresh_press(Some((row, col))) {
                            scan.st[i] = PadState::Held {
                                row: nrow,
                                col: ncol,
                                switch: nswitch,
                                t0: now,
                                prev: Some((row, col)),
                            };
                        }
                    }
                }
            }

            Timer::after_millis(6).await;
        }
    }
}

/// Raw press-classification state for [`PicoBattleInput::next_pad_event`].
#[derive(Default)]
pub struct PadScan {
    st: [PadState; 2],
    /// Per-row bitmask of buttons overridden by a newer hold — ignored until
    /// physically released.
    stale: [u8; 4],
}

#[derive(Default, Clone, Copy)]
enum PadState {
    #[default]
    Idle,
    /// Button down; timing a tap vs a hold. `prev` is a still-held button
    /// whose hold view is currently showing.
    Held { row: usize, col: usize, switch: bool, t0: u64, prev: Option<(usize, usize)> },
    /// Hold reported; waiting for release or a second press.
    HoldOut { row: usize, col: usize },
}

/// Button-only [`InputSource`] for no-USB builds: the same shared
/// [`ChoiceCollector`] loop as the USB CLI, minus typed input (CLI effects
/// are dropped — there is no terminal).
impl InputSource for PicoBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            let first = bus.prompt.receive().await;
            let batch_total = first.batch_total.max(1);
            let mut prompts: alloc::vec::Vec<ActivePrompt> = alloc::vec::Vec::with_capacity(batch_total);
            prompts.push(first);
            while prompts.len() < batch_total {
                prompts.push(bus.prompt.receive().await);
            }

            let batch: alloc::vec::Vec<SlotOptions> =
                prompts.iter().map(SlotOptions::from_prompt).collect();
            let mut fx = alloc::vec::Vec::new();
            let mut col = ChoiceCollector::new(batch, &mut fx);
            apply_oled_effects(&mut fx);

            let mut scan = PadScan::default();
            loop {
                match select(self.next_pad_event(&mut scan), Timer::after_millis(COLLECT_TICK_MS)).await {
                    Either::First(ev) => col.pad_event(ev, Instant::now().as_millis(), &mut fx),
                    Either::Second(()) => {}
                }
                let done = col.tick(Instant::now().as_millis(), &mut fx);
                apply_oled_effects(&mut fx);
                if done {
                    break;
                }
            }
            for (player_id, choice) in col.take_choices() {
                let choice = if choice.is_empty() { String::from("pass") } else { choice };
                bus.choices.send(PlayerChoice { player_id, choice }).await;
            }
        }
    }
}

/// Forward the collector's display effects to the OLED task; CLI text effects
/// are dropped (no terminal on this path).
pub(crate) fn apply_oled_effects(fx: &mut alloc::vec::Vec<CollectEffect>) {
    for e in fx.drain(..) {
        #[cfg(feature = "oled")]
        if let CollectEffect::Oled(cmd) = e {
            oled_send(cmd);
        }
        #[cfg(not(feature = "oled"))]
        let _ = e;
    }
}
