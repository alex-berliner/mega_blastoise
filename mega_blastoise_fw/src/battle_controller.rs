//! Input router: USB serial and physical GPIO buttons race for each choice.
//!
//! [`BattleController`] races USB (rich terminal with menus) against GPIO button
//! matrix; whichever answers first wins.

use mega_blastoise_core::{InputBus, InputSource};

use crate::pico_battle_input::PicoBattleInput;
use crate::usb_input::UsbBattleInput;

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
