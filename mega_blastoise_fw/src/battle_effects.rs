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
        // Split and Prompt are internal engine signals; the USB UI handles prompts itself
        // and split markers are noise for the player.
        let display_on_usb = !matches!(&event, BoardEvent::Split { .. } | BoardEvent::Prompt { .. });

        if display_on_usb {
            if let Some(bus) = self.bus {
                let msg = event.description();
                if bus.log.try_send(msg).is_err() {
                    defmt::warn!("battle_effects: log channel full, event dropped");
                }
            }
        }
    }
}
