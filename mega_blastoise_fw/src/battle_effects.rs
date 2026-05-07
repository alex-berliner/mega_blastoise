use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

use mega_blastoise_fw::hp_bar::HpBarState;
use mega_blastoise_fw::hw_object::HwObject;

#[cfg(feature = "buzzer")]
use crate::subsystems::buzzer::{buzz, BuzzerCmd};

#[cfg(feature = "oled")]
use crate::subsystems::oled::{send as oled_send, OledCmd};

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

fn mon_player_id(mon: &str) -> Option<&str> {
    mon.split(',').nth(1)
}

/// Copy up to 12 bytes of a Pokémon name into a fixed-size buffer.
fn name_buf(name: &str) -> ([u8; 12], u8) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(12) as u8;
    let mut buf = [b' '; 12];
    buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
    (buf, len)
}

impl BoardEffects for BattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        // ── HP tracking + hardware output ─────────────────────────────────────
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                defmt::info!("hp event: mon={} health={}", mon.as_str(), health.as_str());
                if let Some(hp) = HpBarState::parse(health) {
                    let player = mon_player_id(mon);
                    match player {
                        Some("p1") => {
                            self.p1_hp.update(hp);
                            let pct = if hp.max > 0 { (hp.current as u32 * 100 / hp.max as u32) as u8 } else { 0 };
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::HpUpdate { player: 1, pct });
                        }
                        Some("p2") => {
                            self.p2_hp.update(hp);
                            let pct = if hp.max > 0 { (hp.current as u32 * 100 / hp.max as u32) as u8 } else { 0 };
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::HpUpdate { player: 2, pct });
                        }
                        _ => defmt::warn!("hp event: unknown player in mon={}", mon.as_str()),
                    }
                    if matches!(&event, BoardEvent::Damage { .. }) {
                        #[cfg(feature = "buzzer")]
                        buzz(BuzzerCmd::Hit);
                    }
                } else {
                    defmt::warn!("hp event: parse failed for health={}", health.as_str());
                }
            }

            BoardEvent::Faint { mon } => {
                defmt::info!("faint: {}", mon.as_str());
                if let Some(pid) = mon_player_id(mon) {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    #[cfg(feature = "oled")]
                    oled_send(OledCmd::Faint { player });
                }
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Faint);
            }

            BoardEvent::SwitchIn { name, player_id, .. } => {
                if let Some(pid) = player_id {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    let (buf, len) = name_buf(name.as_str());
                    #[cfg(feature = "oled")]
                    oled_send(OledCmd::ActiveMon { player, name: buf, len });
                }
            }

            BoardEvent::Win { .. } | BoardEvent::Tie => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Win);
                #[cfg(feature = "oled")]
                oled_send(OledCmd::Win);
            }

            _ => {}
        }

        // ── USB log narration ─────────────────────────────────────────────────
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
