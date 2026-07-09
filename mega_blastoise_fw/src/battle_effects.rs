use core::sync::atomic::{AtomicBool, Ordering};
use embassy_time::Timer;
use mega_blastoise_core::{mon_player_num, BoardEffects, BoardEvent, HpBarState, InputBus};
#[cfg(feature = "leds")]
use mega_blastoise_core::player_id_to_num;

pub static ANIM_ENABLED: AtomicBool = AtomicBool::new(true);

#[cfg(feature = "buzzer")]
use crate::subsystems::buzzer::{buzz, BuzzerCmd};

#[cfg(feature = "oled")]
use crate::subsystems::oled::send as oled_send;

#[cfg(feature = "leds")]
use crate::subsystems::led::{send as led_send, LedCmd, LedStatus};

pub struct BattleEffects<'a> {
    bus: Option<&'a InputBus>,
    /// When false, hardware LED output is suppressed (used for the lobby's
    /// demo battle so it doesn't fight the calm idle animation).
    #[cfg(feature = "leds")]
    leds: bool,
}

impl<'a> BattleEffects<'a> {
    pub fn new(bus: Option<&'a InputBus>, leds_enabled: bool) -> Self {
        #[cfg(not(feature = "leds"))]
        let _ = leds_enabled;
        Self {
            bus,
            #[cfg(feature = "leds")]
            leds: leds_enabled,
        }
    }

    #[cfg(feature = "leds")]
    fn led(&self, cmd: LedCmd) {
        if self.leds {
            led_send(cmd);
        }
    }
}

impl BoardEffects for BattleEffects<'_> {
    async fn on_event(&mut self, event: BoardEvent) {
        // ── OLED: shared event→command mapping (mega_blastoise_core) ─────────
        // What the displays show is decided once, in core, identically for
        // firmware and web. This block only forwards.
        #[cfg(feature = "oled")]
        for cmd in mega_blastoise_core::oled_cmds_for_event(&event) {
            oled_send(cmd);
        }

        // ── LED strips + buzzer (hardware-only side effects) ──────────────────
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                defmt::info!("hp event: mon={} health={}", mon.as_str(), health.as_str());
                if let Some(hp) = HpBarState::parse(health) {
                    let _pct = hp.pct();
                    match mon_player_num(mon) {
                        Some(_player) => {
                            defmt::info!("P{} HP: {}/{}", _player, hp.current, hp.max);
                            #[cfg(feature = "leds")]
                            self.led(LedCmd::HpUpdate { player: _player, pct: _pct });
                        }
                        None => defmt::warn!("hp event: unknown player in mon={}", mon.as_str()),
                    }
                    if matches!(&event, BoardEvent::Damage { .. }) {
                        #[cfg(feature = "buzzer")]
                        buzz(BuzzerCmd::Hit);
                    }
                } else {
                    defmt::warn!("hp event: parse failed for health={}", health.as_str());
                }
            }

            BoardEvent::Faint { mon, team_slot: _team_slot } => {
                defmt::info!("faint: {}", mon.as_str());
                #[cfg(feature = "leds")]
                if let (Some(player), Some(slot)) = (mon_player_num(mon), _team_slot) {
                    self.led(LedCmd::Faint { player, slot: *slot });
                }
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Faint);
            }

            BoardEvent::SwitchIn { player_id: _player_id, team_slot: _team_slot, .. } => {
                #[cfg(feature = "leds")]
                if let (Some(pid), Some(slot)) = (_player_id, _team_slot) {
                    self.led(LedCmd::SwitchIn { player: player_id_to_num(pid), slot: *slot });
                }
            }

            BoardEvent::SuperEffective { .. } => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::SuperEffective);
            }

            BoardEvent::CriticalHit { .. } => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Crit);
            }

            BoardEvent::SetStatus { mon: _mon, status: _status } => {
                #[cfg(feature = "leds")]
                if let Some(player) = mon_player_num(_mon) {
                    if let Some(s) = LedStatus::from_str(_status.as_str()) {
                        self.led(LedCmd::SetStatus { player, status: s });
                    }
                }
            }

            BoardEvent::CureStatus { mon: _mon, .. } => {
                #[cfg(feature = "leds")]
                if let Some(player) = mon_player_num(_mon) {
                    self.led(LedCmd::CureStatus { player });
                }
            }

            BoardEvent::Win { side } => {
                let _winner = BoardEvent::win_player_num(side);
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Win);
                #[cfg(feature = "leds")]
                self.led(LedCmd::Win { winner: _winner });
            }

            BoardEvent::Tie => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Win);
                #[cfg(feature = "leds")]
                self.led(LedCmd::Win { winner: 0 });
            }

            _ => {}
        }

        // ── Animation delay ───────────────────────────────────────────────────
        // Skipped under `trace` (fast hardware-verification builds) — same
        // rationale as DemoAi's pacing delay. Normal builds keep animations,
        // runtime-toggleable via `:anim off`.
        #[cfg(not(feature = "trace"))]
        {
            let delay_ms = event.anim_delay_ms();
            if delay_ms > 0 && ANIM_ENABLED.load(Ordering::Relaxed) {
                Timer::after_millis(delay_ms as u64).await;
            }
        }

        // ── USB log narration ─────────────────────────────────────────────────
        if event.should_narrate() {
            if let Some(bus) = self.bus {
                let msg = event.description();
                if bus.log.try_send(msg).is_err() {
                    defmt::warn!("battle_effects: log channel full, event dropped");
                }
            }
        }
    }
}
