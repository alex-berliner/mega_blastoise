//! Physical outputs (RGB / buzzer / PWM). Branch on [`BoardEvent`] — descriptions are for RTT only.

use defmt::info;
use mega_blastoise_core::{BoardEffects, BoardEvent};

pub struct DefmtBattleEffects;

impl DefmtBattleEffects {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefmtBattleEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardEffects for DefmtBattleEffects {
    fn on_event(&mut self, event: BoardEvent) {
        let msg = event.description();
        info!("{}", defmt::Display2Format(&msg));

        match event {
            BoardEvent::Split { .. } => {
                // Future: light which side’s “private” rail is active, if you mirror that in HW
            }
            BoardEvent::Damage { health, .. } => {
                let _ = health;
                // Future: map public HP string → NeoPixel color
            }
            BoardEvent::Faint { .. } => {}
            BoardEvent::Move { .. } => {}
            BoardEvent::SwitchIn { .. } => {
                // Future: LED on the correct player + active/bench from log `position` if you plumb it
            }
            BoardEvent::Prompt { kind, .. } => {
                let _ = kind;
                // Future: turn indicator GPIO per player
            }
            _ => {}
        }
    }
}
