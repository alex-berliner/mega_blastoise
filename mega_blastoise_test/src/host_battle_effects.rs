/// Host mirror of `mega_blastoise_fw::battle_effects::BattleEffects`.
///
/// Tracks HP for both active Pokémon via `HostHwObject<HostHpBarState>` (mirroring
/// the firmware's `HwObject<HpBarState>`). When a `bus` is provided, narration events
/// are routed through `bus.log` exactly as the firmware does; without one, they go
/// directly to stdout.
use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

use crate::host_hp_bar::HostHpBarState;
use crate::host_hw_object::HostHwObject;

pub struct HostBattleEffects<'a> {
    bus: Option<&'a InputBus>,
    p1_hp: HostHwObject<HostHpBarState>,
    p2_hp: HostHwObject<HostHpBarState>,
}

impl<'a> HostBattleEffects<'a> {
    pub fn new(bus: Option<&'a InputBus>) -> Self {
        Self {
            bus,
            p1_hp: HostHwObject::new("P1 HP", HostHpBarState::ZERO, None),
            p2_hp: HostHwObject::new("P2 HP", HostHpBarState::ZERO, None),
        }
    }

    pub fn p1_hp(&self) -> &HostHpBarState {
        self.p1_hp.state()
    }

    pub fn p2_hp(&self) -> &HostHpBarState {
        self.p2_hp.state()
    }
}

fn mon_player_id(mon: &str) -> Option<&str> {
    mon.split(',').nth(1)
}

impl BoardEffects for HostBattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let Some(hp) = HostHpBarState::parse(health) {
                    match mon_player_id(mon) {
                        Some("p1") => self.p1_hp.update(hp),
                        Some("p2") => self.p2_hp.update(hp),
                        _ => eprintln!("[WARN] hp event: unknown player in mon={}", mon),
                    }
                } else {
                    eprintln!("[WARN] hp event: parse failed for health={}", health);
                }
            }
            _ => {}
        }

        // Split and Prompt are internal signals; suppress from narration (mirrors firmware).
        let narrate = !matches!(&event, BoardEvent::Split { .. } | BoardEvent::Prompt { .. });
        if narrate {
            if let Some(bus) = self.bus {
                if bus.log.try_send(event.description()).is_err() {
                    eprintln!("[WARN] log channel full, event dropped");
                }
            } else {
                println!("{}", event.description());
            }
        }
    }
}
