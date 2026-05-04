use defmt::info;
use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

pub struct BattleEffects<'a> {
    bus: Option<&'a InputBus>,
}

impl<'a> BattleEffects<'a> {
    pub fn new(bus: Option<&'a InputBus>) -> Self {
        Self { bus }
    }
}

impl BoardEffects for BattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        let msg = event.description();
        info!("{}", defmt::Display2Format(&msg));
        if let Some(bus) = self.bus {
            let _ = bus.log.try_send(msg);
        }
    }
}
