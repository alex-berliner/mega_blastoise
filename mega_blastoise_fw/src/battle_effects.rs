use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

use mega_blastoise_fw::hp_bar::HpBarState;
use mega_blastoise_fw::hw_object::HwObject;

pub struct BattleEffects<'a> {
    bus: Option<&'a InputBus>,
    p1_hp: HwObject<HpBarState>,
    p2_hp: HwObject<HpBarState>,
}

impl<'a> BattleEffects<'a> {
    pub fn new(bus: Option<&'a InputBus>) -> Self {
        Self {
            bus,
            p1_hp: HwObject::new("P1 HP", HpBarState::ZERO, None),
            p2_hp: HwObject::new("P2 HP", HpBarState::ZERO, None),
        }
    }
}

/// Extract the player id from a battler mon position field (`"name,player_id,pos"`).
fn mon_player_id(mon: &str) -> Option<&str> {
    mon.split(',').nth(1)
}

impl BoardEffects for BattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                defmt::info!("hp event: mon={} health={}", mon.as_str(), health.as_str());
                if let Some(hp) = HpBarState::parse(health) {
                    match mon_player_id(mon) {
                        Some("p1") => self.p1_hp.update(hp),
                        Some("p2") => self.p2_hp.update(hp),
                        _ => defmt::warn!("hp event: unknown player in mon={}", mon.as_str()),
                    }
                } else {
                    defmt::warn!("hp event: parse failed for health={}", health.as_str());
                }
            }
            _ => {}
        }

        // Split and Prompt are internal signals; suppress from the USB narrative.
        let display_on_usb =
            !matches!(&event, BoardEvent::Split { .. } | BoardEvent::Prompt { .. });

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
