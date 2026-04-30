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
            BoardEvent::Split { side } => {
                info!("event split side={}", defmt::Display2Format(&side));
                // Future: light which side’s “private” rail is active, if you mirror that in HW
            }
            BoardEvent::Damage { mon, health } => {
                info!(
                    "event damage mon={} health={}",
                    defmt::Display2Format(&mon),
                    defmt::Display2Format(&health)
                );
                // Future: map public HP string → NeoPixel color
            }
            BoardEvent::Heal { mon, health } => {
                info!(
                    "event heal mon={} health={}",
                    defmt::Display2Format(&mon),
                    defmt::Display2Format(&health)
                );
            }
            BoardEvent::Faint { mon } => {
                info!("event faint mon={}", defmt::Display2Format(&mon));
            }
            BoardEvent::Move { name } => {
                info!("event move name={}", defmt::Display2Format(&name));
            }
            BoardEvent::SwitchIn { name, player_id, .. } => {
                let who = player_id.unwrap_or_else(|| "?".into());
                info!(
                    "event switch_in player={} name={}",
                    defmt::Display2Format(&who),
                    defmt::Display2Format(&name)
                );
                // Future: LED on the correct player + active/bench from log `position` if you plumb it
            }
            BoardEvent::SwitchOut => {
                info!("event switch_out");
            }
            BoardEvent::Turn { n } => {
                info!("event turn n={}", n);
            }
            BoardEvent::BattleStart => {
                info!("event battle_start");
            }
            BoardEvent::Win { side } => {
                let who = side.unwrap_or_else(|| "?".into());
                info!("event win side={}", defmt::Display2Format(&who));
            }
            BoardEvent::Tie => {
                info!("event tie");
            }
            BoardEvent::Prompt { kind, .. } => {
                let _ = kind;
                info!("event prompt");
                // Future: turn indicator GPIO per player
            }
        }
    }
}
