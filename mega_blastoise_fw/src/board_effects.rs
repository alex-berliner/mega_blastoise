//! Board output effects: RTT logging and USB serial forwarding.

use defmt::info;
use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

/// Logs all battle events to RTT (defmt). Used when no USB sink is available.
pub struct DefmtBattleEffects;

impl DefmtBattleEffects {
    pub fn new() -> Self { Self }
}

impl Default for DefmtBattleEffects {
    fn default() -> Self { Self::new() }
}

impl BoardEffects for DefmtBattleEffects {
    fn on_event(&mut self, event: BoardEvent) {
        let msg = event.description();
        info!("{}", defmt::Display2Format(&msg));
    }
}

/// Forwards battle event descriptions to the USB serial terminal via `bus.log`.
/// Also logs to RTT. If the log channel is full the line is dropped (non-blocking).
pub struct UsbBattleEffects<'a> {
    bus: &'a InputBus,
}

impl<'a> UsbBattleEffects<'a> {
    pub fn new(bus: &'a InputBus) -> Self { Self { bus } }
}

impl<'a> BoardEffects for UsbBattleEffects<'a> {
    fn on_event(&mut self, event: BoardEvent) {
        let msg = event.description();
        info!("{}", defmt::Display2Format(&msg));
        // try_send is non-blocking; drop if full (log channel is 8 deep).
        let _ = self.bus.log.try_send(msg);
    }
}
