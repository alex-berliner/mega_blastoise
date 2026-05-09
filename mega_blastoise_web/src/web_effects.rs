use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

pub struct WebBattleEffects<'a> {
    bus: &'a InputBus,
}

impl<'a> WebBattleEffects<'a> {
    pub fn new(bus: &'a InputBus) -> Self {
        Self { bus }
    }
}

impl BoardEffects for WebBattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        let narrate = !matches!(
            &event,
            BoardEvent::Split { .. } | BoardEvent::Prompt { .. } | BoardEvent::MovesUpdate { .. }
        );
        if narrate {
            let _ = self.bus.log.try_send(event.description());
        }
    }
}
