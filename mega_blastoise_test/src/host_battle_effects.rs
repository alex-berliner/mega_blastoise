/// Host mirror of `mega_blastoise_fw::battle_effects::BattleEffects`.
///
/// Handles the same [`BoardEvent`] variants as the firmware: HP tracking,
/// active-mon updates, faint, and win.  Calls [`HostBuzzer`] and [`HostOled`]
/// stubs so that tests can observe sound/display events without hardware.
use mega_blastoise_core::{BoardEffects, BoardEvent, InputBus};

use crate::host_buzzer::HostBuzzer;
use crate::host_hp_bar::HostHpBarState;
use crate::host_hw_object::HostHwObject;
use crate::host_led::HostLed;
use crate::host_oled::HostOled;

pub struct HostBattleEffects<'a> {
    bus: Option<&'a InputBus>,
    p1_hp: HostHwObject<HostHpBarState>,
    p2_hp: HostHwObject<HostHpBarState>,
    pub buzzer: HostBuzzer,
    pub oled: HostOled,
    pub led: HostLed,
}

impl<'a> HostBattleEffects<'a> {
    pub fn new(bus: Option<&'a InputBus>) -> Self {
        Self {
            bus,
            p1_hp: HostHwObject::new("P1 HP", HostHpBarState::ZERO, None),
            p2_hp: HostHwObject::new("P2 HP", HostHpBarState::ZERO, None),
            buzzer: HostBuzzer::new(),
            oled: HostOled::new(),
            led: HostLed::new(),
        }
    }

    /// Silence all stdout output from buzzer, OLED, and LED stubs (useful in automated tests).
    pub fn silent(bus: Option<&'a InputBus>) -> Self {
        let mut s = Self::new(bus);
        s.buzzer = HostBuzzer::silent();
        s.oled = HostOled::silent();
        s.led = HostLed::silent();
        s
    }

    pub fn p1_hp(&self) -> &HostHpBarState { self.p1_hp.state() }
    pub fn p2_hp(&self) -> &HostHpBarState { self.p2_hp.state() }
}

fn mon_player_id(mon: &str) -> Option<&str> {
    mon.split(',').nth(1)
}

fn status_label(s: &str) -> &str {
    match s {
        "par" => "PAR",
        "brn" => "BRN",
        "frz" => "FRZ",
        "psn" => "PSN",
        "tox" => "TOX",
        "slp" => "SLP",
        other => other,
    }
}

impl BoardEffects for HostBattleEffects<'_> {
    fn on_event(&mut self, event: BoardEvent) {
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                eprintln!("[RTT] hp event: mon={mon} health={health}");
                if let Some(hp) = HostHpBarState::parse(health) {
                    let pct = hp.pct() as u8;
                    match mon_player_id(mon) {
                        Some("p1") => {
                            self.p1_hp.update(hp);
                            self.oled.update_hp(1, pct);
                            self.led.update_hp(1, pct);
                        }
                        Some("p2") => {
                            self.p2_hp.update(hp);
                            self.oled.update_hp(2, pct);
                            self.led.update_hp(2, pct);
                        }
                        _ => eprintln!("[RTT:WARN] hp event: unknown player in mon={mon}"),
                    }
                    if matches!(&event, BoardEvent::Damage { .. }) {
                        self.buzzer.hit();
                    }
                } else {
                    eprintln!("[RTT:WARN] hp event: parse failed for health={health}");
                }
            }

            BoardEvent::Faint { mon } => {
                eprintln!("[RTT] faint: {mon}");
                if let Some(pid) = mon_player_id(mon) {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    self.oled.faint(player);
                    self.led.faint(player);
                }
                self.buzzer.faint();
            }

            BoardEvent::SwitchIn { name, player_id, moves, .. } => {
                if let Some(pid) = player_id {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    self.oled.switch_in(player, name.clone(), moves.clone());
                }
            }

            BoardEvent::MovesUpdate { player_id, moves } => {
                let player = if player_id.as_str() == "p1" { 1u8 } else { 2u8 };
                self.oled.update_moves(player, moves.clone());
            }

            BoardEvent::SuperEffective { .. } => {
                self.buzzer.super_effective();
            }

            BoardEvent::CriticalHit { .. } => {
                self.buzzer.critical_hit();
            }

            BoardEvent::SetStatus { mon, status } => {
                if let Some(pid) = mon_player_id(mon) {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    self.led.set_status(player, status_label(status.as_str()));
                }
            }

            BoardEvent::CureStatus { mon, .. } => {
                if let Some(pid) = mon_player_id(mon) {
                    let player = if pid == "p1" { 1u8 } else { 2u8 };
                    self.led.cure_status(player);
                }
            }

            BoardEvent::Win { side } => {
                let winner = match side.as_deref() {
                    Some("0") => 1u8,
                    Some("1") => 2u8,
                    _ => 0,
                };
                self.buzzer.win();
                self.oled.win(winner);
                self.led.win(winner);
            }

            BoardEvent::Tie => {
                self.buzzer.win();
                self.oled.win(0);
                self.led.win(0);
            }

            _ => {}
        }

        // Split, Prompt, MovesUpdate are internal signals; suppress from narration.
        let narrate = !matches!(
            &event,
            BoardEvent::Split { .. } | BoardEvent::Prompt { .. } | BoardEvent::MovesUpdate { .. }
        );
        if narrate {
            if let Some(bus) = self.bus {
                if bus.log.try_send(event.description()).is_err() {
                    eprintln!("[RTT:WARN] battle_effects: log channel full, event dropped");
                }
            } else {
                println!("{}", event.description());
            }
        }
    }
}
