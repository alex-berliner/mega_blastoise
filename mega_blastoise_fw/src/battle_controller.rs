//! Input router: USB serial and physical GPIO buttons race for each choice.
//!
//! [`BattleController`] — legacy rich-terminal path (USB shows menus).
//! [`ButtonBattleController`] — button-event path: USB sends a digit, GPIO scans
//! a row; whichever arrives first wins.  All battle-protocol logic lives in
//! [`mega_blastoise_core::ButtonController`].

use embassy_futures::select::{select, Either};
use mega_blastoise_core::{ButtonController, ButtonSource, InputBus, InputSource, PlayerAction};

use crate::pico_battle_input::PicoBattleInput;
use crate::usb_input::{UsbBattleInput, UsbButtonInput};

// ── Legacy rich-terminal controller ──────────────────────────────────────────

pub struct BattleController<'d> {
    usb: UsbBattleInput<'d>,
    buttons: PicoBattleInput<'d>,
}

impl<'d> BattleController<'d> {
    pub fn new(usb: UsbBattleInput<'d>, buttons: PicoBattleInput<'d>) -> Self {
        Self { usb, buttons }
    }

    pub fn into_parts(self) -> (UsbBattleInput<'d>, PicoBattleInput<'d>) {
        (self.usb, self.buttons)
    }
}

impl InputSource for BattleController<'_> {
    async fn run(&mut self, bus: &InputBus) {
        self.usb.run_inner(bus, Some(&mut self.buttons)).await;
    }
}

// ── Button-event controller ───────────────────────────────────────────────────

/// Races USB serial digits against GPIO button scans; the first valid press wins.
/// Wraps both in [`ButtonController`] so battle-protocol logic is shared.
struct CombinedButtonSource<'d> {
    usb: UsbButtonInput<'d>,
    gpio: PicoBattleInput<'d>,
}

impl ButtonSource for CombinedButtonSource<'_> {
    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        match select(
            self.gpio.wait_action(player_id, n_moves),
            self.usb.wait_action(player_id, n_moves),
        )
        .await
        {
            Either::First(a) | Either::Second(a) => a,
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        match select(
            self.gpio.wait_switch(player_id),
            self.usb.wait_switch(player_id),
        )
        .await
        {
            Either::First(s) | Either::Second(s) => s,
        }
    }
}

pub struct ButtonBattleController<'d> {
    inner: ButtonController<CombinedButtonSource<'d>>,
}

impl<'d> ButtonBattleController<'d> {
    pub fn new(usb: UsbButtonInput<'d>, gpio: PicoBattleInput<'d>) -> Self {
        Self {
            inner: ButtonController::new(CombinedButtonSource { usb, gpio }),
        }
    }
}

impl InputSource for ButtonBattleController<'_> {
    async fn run(&mut self, bus: &InputBus) {
        self.inner.run(bus).await
    }
}
