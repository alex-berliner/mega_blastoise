//! Unified input router: USB display + USB serial and physical buttons race for each choice.
//! The first input source to provide a valid answer wins; the loser is cancelled.

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
}

impl InputSource for BattleController<'_> {
    async fn run(&mut self, bus: &InputBus) {
        self.usb.run_inner(bus, Some(&mut self.buttons)).await;
    }
}
