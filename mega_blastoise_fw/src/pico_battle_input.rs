//! Button-matrix driver — table-driven, so a PCB revision is just a pin map.
//!
//! Boards (default = the partner PCB; `breadboard` selects the old board):
//!
//! `breadboard` — hand-wired breadboard (per ELECTRONICS.md):
//!   drives GP5 (P1 moves), GP7 (P1 party), GP8 (P2 moves), GP9 (P2 party);
//!   senses GP10–GP13.
//!
//! DEFAULT — partner-made PCB (pairs probed with gpio_probe and functions
//!   confirmed on the faceplate, 2026-07-10): GP9 and GP13 are one net that
//!   serves as BOTH the 4th sense line for the GP6/7/8 drives AND the drive
//!   line for P2 party 2/3. GP13 is redundant (tied to GP9) and untouched.
//!     GP6 × {GP10, GP11, GP12, GP9} = P1 moves 1, 3, 2, 4
//!     GP7 × {GP10, GP11, GP12}      = P1 party 1-3,  GP7 × GP9 = P2 move 1
//!     GP8 × {GP10, GP11, GP12}      = P2 moves 3, 2, 4,  GP8 × GP9 = P2 party 1
//!     GP9 × {GP10, GP11}            = P2 party 2-3
//!
//! Every pin is a `Flex`: a scan drives one line low at a time (open-drain
//! style, inputs with pull-ups otherwise) and reads the mapped sense pins.
//! LOW = pressed.

extern crate alloc;

use alloc::string::String;

use cortex_m::asm::delay as asm_delay;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Flex, Pull};
use embassy_time::{Instant, Timer};
use mega_blastoise_core::{
    ActivePrompt, ChoiceCollector, CollectEffect, ControlMode, ControlsSelect, InputBus,
    InputSource, PadEvent, PlayerChoice, SlotOptions, COLLECT_TICK_MS, HOLD_THRESHOLD_MS,
};
#[cfg(feature = "oled")]
use crate::subsystems::oled::send as oled_send;

/// One physical button's identity.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PadBtn {
    /// 1 or 2.
    pub player: u8,
    /// true = party button, false = move button.
    pub switch: bool,
    /// 0-based slot (move 0-3 / party 0-2).
    pub idx: u8,
}

const fn mv(player: u8, idx: u8) -> PadBtn {
    PadBtn { player, switch: false, idx }
}
const fn pt(player: u8, idx: u8) -> PadBtn {
    PadBtn { player, switch: true, idx }
}

/// The board as data: for each drive line (index into the pin array), the
/// sense pins to read while it is low and the button each one means.
type ScanTable = &'static [(usize, &'static [(usize, PadBtn)])];

#[cfg(feature = "breadboard")]
mod board {
    use super::{mv, pt, ScanTable};
    /// Pin array order: GP5, GP7, GP8, GP9, GP10, GP11, GP12, GP13.
    pub const N_PINS: usize = 8;
    pub const SCANS: ScanTable = &[
        (0, &[(4, mv(1, 0)), (5, mv(1, 1)), (6, mv(1, 2)), (7, mv(1, 3))]),
        (1, &[(4, pt(1, 0)), (5, pt(1, 1)), (6, pt(1, 2))]),
        (2, &[(4, mv(2, 0)), (5, mv(2, 1)), (6, mv(2, 2)), (7, mv(2, 3))]),
        (3, &[(4, pt(2, 0)), (5, pt(2, 1)), (6, pt(2, 2))]),
    ];
}

#[cfg(not(feature = "breadboard"))]
mod board {
    use super::{mv, pt, ScanTable};
    /// Pin array order: GP6, GP7, GP8, GP9, GP10, GP11, GP12.
    /// Index 3 (GP9) is both a sense (rows GP6/7/8) and a drive (last scan).
    /// Function assignment confirmed against the faceplate 2026-07-10 (note
    /// the scrambled move orders — they match the physical silkscreen).
    pub const N_PINS: usize = 7;
    pub const SCANS: ScanTable = &[
        (0, &[(4, mv(1, 0)), (5, mv(1, 2)), (6, mv(1, 1)), (3, mv(1, 3))]),
        (1, &[(4, pt(1, 0)), (5, pt(1, 1)), (6, pt(1, 2)), (3, mv(2, 0))]),
        (2, &[(4, mv(2, 2)), (5, mv(2, 1)), (6, mv(2, 3)), (3, pt(2, 0))]),
        (3, &[(4, pt(2, 1)), (5, pt(2, 2))]),
    ];
}

pub use board::N_PINS;

/// Snapshot of every button's state from one full scan.
/// Bit layout: `mask[player-1][kind]`, kind 0 = moves, 1 = party.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Pressed {
    mask: [[u8; 2]; 2],
}

impl Pressed {
    fn set(&mut self, b: PadBtn) {
        self.mask[(b.player - 1) as usize][b.switch as usize] |= 1 << b.idx;
    }
    fn down(&self, b: PadBtn) -> bool {
        self.mask[(b.player - 1) as usize][b.switch as usize] & (1 << b.idx) != 0
    }
    fn any(&self, player: u8) -> bool {
        self.mask[(player - 1) as usize] != [0, 0]
    }
}

pub struct ButtonMatrix<'d> {
    pins: [Flex<'d>; board::N_PINS],
}

impl<'d> ButtonMatrix<'d> {
    pub fn new(mut pins: [Flex<'d>; board::N_PINS]) -> Self {
        for pin in pins.iter_mut() {
            pin.set_pull(Pull::Up);
            pin.set_as_input();
        }
        Self { pins }
    }

    /// One full board scan: drive each table line low in turn (open-drain,
    /// never driven high), read its mapped sense pins, restore.
    fn scan(&mut self) -> Pressed {
        let mut out = Pressed::default();
        for &(drive, senses) in board::SCANS {
            self.pins[drive].set_low();
            self.pins[drive].set_as_output();
            asm_delay(1500); // ≈ 12 µs settle at 125 MHz
            for &(sense, btn) in senses {
                if self.pins[sense].is_low() {
                    out.set(btn);
                }
            }
            self.pins[drive].set_as_input();
            asm_delay(500);
        }
        out
    }

    /// Wait for a button press from either player.
    /// Short press (< 500 ms) → P1/P2; long press (≥ 500 ms) → P1Long/P2Long.
    /// Any press performs unready in the lobby; long press selects AI opponent.
    pub async fn wait_lobby_press(&mut self) -> LobbyPress {
        loop {
            let snap = self.scan();
            for player in [1u8, 2] {
                if !snap.any(player) {
                    continue;
                }
                let mut held_ms = 0u64;
                let is_long = loop {
                    Timer::after_millis(10).await;
                    held_ms += 10;
                    if !self.scan().any(player) {
                        break false;
                    }
                    if held_ms >= 500 {
                        break true;
                    }
                };
                if is_long {
                    loop {
                        Timer::after_millis(10).await;
                        if !self.scan().any(player) {
                            break;
                        }
                    }
                    return if player == 1 { LobbyPress::P1Long } else { LobbyPress::P2Long };
                }
                return if player == 1 { LobbyPress::P1 } else { LobbyPress::P2 };
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
pub struct PicoBattleInput<'d> {
    matrix: ButtonMatrix<'d>,
    /// Per-player control scheme for the current battle, chosen at battle
    /// start (set by `main` after the controls-select phase).
    pub modes: [ControlMode; 2],
}

impl<'d> PicoBattleInput<'d> {
    pub fn new(pins: [Flex<'d>; board::N_PINS]) -> Self {
        Self { matrix: ButtonMatrix::new(pins), modes: [ControlMode::Normal; 2] }
    }

    pub async fn wait_lobby_press(&mut self) -> LobbyPress {
        self.matrix.wait_lobby_press().await
    }

    /// The battle-start controls picker, button-driven (no-USB builds).
    pub async fn run_controls_select(&mut self, ai: [bool; 2]) -> [ControlMode; 2] {
        let mut fx: alloc::vec::Vec<CollectEffect> = alloc::vec::Vec::new();
        let mut cs = ControlsSelect::new(ai, &mut fx);
        apply_oled_effects(&mut fx);
        let mut scan = PadScan::default();
        loop {
            match select(self.next_pad_event(&mut scan), Timer::after_millis(COLLECT_TICK_MS)).await
            {
                Either::First(ev) => cs.pad_event(ev, &mut fx),
                Either::Second(()) => {}
            }
            let done = cs.tick(Instant::now().as_millis());
            apply_oled_effects(&mut fx);
            if done {
                break;
            }
        }
        cs.take_modes()
    }

    /// Next classified button event from the matrix, for either player.
    /// Classification (tap vs ≥500 ms hold) lives here; ALL semantics live in
    /// [`ChoiceCollector`]. Scan state is held in `scan`, outside this future,
    /// so losing a `select` race mid-press doesn't drop the press.
    ///
    /// Two hold levels are tracked per player (concealed controls hold an
    /// action button and then hold a corner button on top): when a second
    /// press becomes a hold it takes the view over, and each release emits
    /// its own HoldEnd — innermost first. A THIRD simultaneous hold stales
    /// the oldest button (ignored until physically released, no event).
    pub async fn next_pad_event(&mut self, scan: &mut PadScan) -> PadEvent {
        loop {
            let pressed = self.matrix.scan();
            // Stale buttons stop being stale once released.
            for p in 0..2 {
                for k in 0..2 {
                    scan.stale[p][k] &= pressed.mask[p][k];
                }
            }
            let now = Instant::now().as_millis();

            for i in 0..2usize {
                let player = (i + 1) as u8;
                let is_stale = |b: PadBtn| {
                    scan.stale[(b.player - 1) as usize][b.switch as usize] & (1 << b.idx) != 0
                };
                // Party buttons take priority, like the original collector.
                let fresh_press = |except: [Option<PadBtn>; 2]| -> Option<PadBtn> {
                    for switch in [true, false] {
                        for idx in 0..4u8 {
                            let b = PadBtn { player, switch, idx };
                            if pressed.down(b) && !is_stale(b) && !except.contains(&Some(b)) {
                                return Some(b);
                            }
                        }
                    }
                    None
                };

                match scan.st[i] {
                    PadState::Idle => {
                        if let Some(btn) = fresh_press([None, None]) {
                            scan.st[i] = PadState::Held { btn, t0: now, outer: None };
                        }
                    }
                    PadState::Held { btn, t0, outer } => {
                        // If the showing hold's button was released while we
                        // time the new press, its view ends now.
                        if let Some(ob) = outer {
                            if !pressed.down(ob) {
                                scan.st[i] = PadState::Held { btn, t0, outer: None };
                                return PadEvent::HoldEnd { player };
                            }
                        }
                        if pressed.down(btn) {
                            if now.saturating_sub(t0) >= HOLD_THRESHOLD_MS {
                                // This hold takes the view over; the button
                                // underneath stays tracked so its own release
                                // still emits a HoldEnd (nested menus).
                                scan.st[i] = PadState::HoldOut { btn, outer };
                                return if btn.switch {
                                    PadEvent::HoldSwitch { player, idx: btn.idx }
                                } else {
                                    PadEvent::HoldMove { player, slot: btn.idx }
                                };
                            }
                        } else {
                            scan.st[i] = match outer {
                                Some(ob) => PadState::HoldOut { btn: ob, outer: None },
                                None => PadState::Idle,
                            };
                            return if btn.switch {
                                PadEvent::TapSwitch { player, idx: btn.idx }
                            } else {
                                PadEvent::TapMove { player, slot: btn.idx }
                            };
                        }
                    }
                    PadState::HoldOut { btn, outer } => {
                        // An outer button released underneath the showing
                        // view: no event (its view isn't up), just untrack.
                        if let Some(ob) = outer {
                            if !pressed.down(ob) {
                                scan.st[i] = PadState::HoldOut { btn, outer: None };
                                continue;
                            }
                        }
                        if !pressed.down(btn) {
                            scan.st[i] = match outer {
                                Some(ob) => PadState::HoldOut { btn: ob, outer: None },
                                None => PadState::Idle,
                            };
                            return PadEvent::HoldEnd { player };
                        }
                        // A second press starts timing while the view stays
                        // up. If a third button is already tracked, it goes
                        // stale (released with no event).
                        if let Some(nb) = fresh_press([Some(btn), outer]) {
                            if let Some(ob) = outer {
                                scan.stale[(ob.player - 1) as usize][ob.switch as usize] |=
                                    1 << ob.idx;
                            }
                            scan.st[i] = PadState::Held { btn: nb, t0: now, outer: Some(btn) };
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
    /// Buttons overridden by a newer hold — ignored until physically
    /// released. `stale[player-1][kind]` bitmask, kind 0 = moves, 1 = party.
    stale: [[u8; 2]; 2],
}

#[derive(Default, Clone, Copy)]
enum PadState {
    #[default]
    Idle,
    /// Button down; timing a tap vs a hold. `outer` is a still-held button
    /// whose hold view is currently showing.
    Held { btn: PadBtn, t0: u64, outer: Option<PadBtn> },
    /// Hold reported; waiting for release or a second press. `outer` is a
    /// still-held earlier hold underneath this view (concealed menus).
    HoldOut { btn: PadBtn, outer: Option<PadBtn> },
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

            let mut batch: alloc::vec::Vec<SlotOptions> =
                prompts.iter().map(SlotOptions::from_prompt).collect();
            // Apply each player's chosen control scheme (fresh layouts per turn).
            for (slot, p) in batch.iter_mut().zip(&prompts) {
                let idx = if p.player_id.as_str() == "p1" { 0 } else { 1 };
                if self.modes[idx] == ControlMode::Concealed {
                    slot.set_concealed(Instant::now().as_millis() ^ (idx as u64) << 33);
                }
            }
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
