use core::sync::atomic::{AtomicBool, Ordering};
use embassy_time::Timer;
use mega_blastoise_core::{mon_player_id, mon_player_num, player_id_to_num, BoardEffects, BoardEvent, HpBarState, InputBus};

pub static ANIM_ENABLED: AtomicBool = AtomicBool::new(true);

#[cfg(feature = "buzzer")]
use crate::subsystems::buzzer::{buzz, BuzzerCmd};

#[cfg(feature = "oled")]
use crate::subsystems::oled::{send as oled_send, OledCmd};

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


/// Copy up to 12 bytes of a Pokémon name into a fixed-size buffer.
fn name_buf(name: &str) -> ([u8; 12], u8) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(12) as u8;
    let mut buf = [b' '; 12];
    buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
    (buf, len)
}

/// Flash a transient combat-narration overlay on BOTH players' OLEDs.
/// Combat messages (move used, crit, miss, status, …) are shared context,
/// so they always go to both screens (`player: 0`), not just the acting
/// side. `text` is byte-truncated to 48; `render_event_text` word-wraps it.
#[cfg(feature = "oled")]
fn oled_flash(text: &str) {
    let bytes = text.as_bytes();
    let len = bytes.len().min(48) as u8;
    let mut buf = [b' '; 48];
    buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
    oled_send(OledCmd::EventFlash { player: 0, text: buf, len });
}

impl BoardEffects for BattleEffects<'_> {
    async fn on_event(&mut self, event: BoardEvent) {
        // ── HP tracking + hardware output ─────────────────────────────────────
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                defmt::info!("hp event: mon={} health={}", mon.as_str(), health.as_str());
                if let Some(hp) = HpBarState::parse(health) {
                    let player = mon_player_id(mon);
                    let pct = hp.pct();
                    match player {
                        Some("p1") => {
                            defmt::info!("P1 HP: {}/{}", hp.current, hp.max);
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::HpUpdate { player: 1, pct });
                            #[cfg(feature = "leds")]
                            self.led(LedCmd::HpUpdate { player: 1, pct });
                        }
                        Some("p2") => {
                            defmt::info!("P2 HP: {}/{}", hp.current, hp.max);
                            #[cfg(feature = "oled")]
                            oled_send(OledCmd::HpUpdate { player: 2, pct });
                            #[cfg(feature = "leds")]
                            self.led(LedCmd::HpUpdate { player: 2, pct });
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

            BoardEvent::Faint { mon, team_slot: _team_slot } => {
                defmt::info!("faint: {}", mon.as_str());
                if let Some(player) = mon_player_num(mon) {
                    #[cfg(feature = "oled")]
                    oled_send(OledCmd::Faint { player });
                    #[cfg(feature = "leds")]
                    if let Some(slot) = _team_slot {
                        self.led(LedCmd::Faint { player, slot: *slot });
                    }
                }
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Faint);
            }

            BoardEvent::SwitchIn { name, player_id, moves, team_slot: _team_slot, .. } => {
                if let Some(pid) = player_id {
                    let player = player_id_to_num(pid);
                    let (buf, len) = name_buf(name.as_str());
                    #[cfg(feature = "oled")]
                    oled_send(OledCmd::ActiveMon { player, name: buf, len });
                    #[cfg(feature = "oled")]
                    if !moves.is_empty() {
                        oled_send(OledCmd::MovesUpdate { player, moves: moves.clone() });
                    }
                    #[cfg(feature = "leds")]
                    if let Some(slot) = _team_slot {
                        self.led(LedCmd::SwitchIn { player, slot: *slot });
                    }
                }
            }

            BoardEvent::SuperEffective { .. } => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::SuperEffective);
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::CriticalHit { .. } => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Crit);
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::SetStatus { mon: _mon, status: _status } => {
                #[cfg(feature = "leds")]
                if let Some(player) = mon_player_num(_mon) {
                    if let Some(s) = LedStatus::from_str(_status.as_str()) {
                        self.led(LedCmd::SetStatus { player, status: s });
                    }
                }
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::CureStatus { mon: _mon, .. } => {
                #[cfg(feature = "leds")]
                if let Some(player) = mon_player_num(_mon) {
                    self.led(LedCmd::CureStatus { player });
                }
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::Win { side } => {
                let winner = BoardEvent::win_player_num(side);
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Win);
                #[cfg(feature = "oled")]
                oled_send(OledCmd::Win { winner });
                #[cfg(feature = "leds")]
                self.led(LedCmd::Win { winner });
            }

            BoardEvent::Tie => {
                #[cfg(feature = "buzzer")]
                buzz(BuzzerCmd::Win);
                #[cfg(feature = "oled")]
                oled_send(OledCmd::Win { winner: 0 });
                #[cfg(feature = "leds")]
                self.led(LedCmd::Win { winner: 0 });
            }

            BoardEvent::MovesUpdate { player_id, moves } => {
                let player = player_id_to_num(player_id.as_str());
                #[cfg(feature = "oled")]
                oled_send(OledCmd::MovesUpdate { player, moves: moves.clone() });
            }

            BoardEvent::Prompt { player_id, .. } => {
                // Restore normal OLED view at the start of each prompt in case a
                // long-press detail screen was left open (e.g. USB won the input race).
                let player = player_id_to_num(player_id.as_str());
                #[cfg(feature = "oled")]
                oled_send(OledCmd::RestoreScreen { player });
            }

            // ── Transient narration flashes for events without a state screen ──
            BoardEvent::Move { .. } => {
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::Miss { .. }
            | BoardEvent::Immune { .. }
            | BoardEvent::Resisted { .. }
            | BoardEvent::Fail { .. } => {
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
            }

            BoardEvent::Cant { .. } => {
                #[cfg(feature = "oled")]
                oled_flash(&event.description());
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
