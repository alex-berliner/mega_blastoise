use mega_blastoise_core::{
    hp_bar_color, hp_bar_count, mon_display_name, mon_player_num, oled_cmds_for_event,
    BoardEffects, BoardEvent, HpBarState, InputBus,
};

// ── LED helpers ───────────────────────────────────────────────────────────────

fn hp_color_packed(pct: u8) -> u32 {
    let (r, g, b) = hp_bar_color(pct);
    crate::pack_rgb(r, g, b)
}

// ── Per-player LED state ──────────────────────────────────────────────────────

struct LedPlayerState {
    hp_pct: u8,
}

impl LedPlayerState {
    fn new() -> Self { Self { hp_pct: 100 } }

    /// Returns HP bar only (8 slots); party LEDs are managed by sync_party_leds.
    fn render(&self) -> [u32; 8] {
        let mut buf = [0u32; 8];
        let lit = hp_bar_count(self.hp_pct);
        let color = hp_color_packed(self.hp_pct);
        for i in 0..lit { buf[i] = color; }
        buf
    }
}

// ── Web battle effects ────────────────────────────────────────────────────────
//
// What the OLEDs show is decided by the shared core state machine — this
// type forwards `oled_cmds_for_event` output to `crate::oled_apply` (exactly
// like the firmware forwards to its OLED task) and keeps only the web's own
// side channels: LED strips, JS flash effects, and party-LED bookkeeping.

pub struct WebBattleEffects<'a> {
    bus: &'a InputBus,
    p1_led: LedPlayerState,
    p2_led: LedPlayerState,
}

impl<'a> WebBattleEffects<'a> {
    pub fn new(bus: &'a InputBus) -> Self {
        let s = Self {
            bus,
            p1_led: LedPlayerState::new(),
            p2_led: LedPlayerState::new(),
        };
        s.flush_leds();
        s
    }

    fn flush_leds(&self) {
        crate::update_hp_leds(1, self.p1_led.render());
        crate::sync_party_leds(1);
        crate::update_hp_leds(2, self.p2_led.render());
        crate::sync_party_leds(2);
    }
}

fn win_leds(winner: u8) -> [u32; 24] {
    let gold = crate::pack_rgb(200, 150, 0);
    let dim  = crate::pack_rgb(40, 0, 0);
    let grey = crate::pack_rgb(60, 60, 60);
    let (c1, c2) = match winner {
        1 => (gold, dim),
        2 => (dim, gold),
        _ => (grey, grey),
    };
    let mut frame = [0u32; 24];
    for i in 0..12  { frame[i] = c1; }
    for i in 12..24 { frame[i] = c2; }
    frame
}

impl BoardEffects for WebBattleEffects<'_> {
    async fn on_event(&mut self, event: BoardEvent) {
        // ── OLED: shared event→command mapping (mega_blastoise_core) ─────────
        for cmd in oled_cmds_for_event(&event) {
            crate::oled_apply(cmd);
        }

        // ── LED strips + JS effects (web-only side channels) ──────────────────
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let (Some(p), Some(pct)) = (mon_player_num(mon), HpBarState::parse(health).map(|h| h.pct())) {
                    if p == 1 { self.p1_led.hp_pct = pct; }
                    else      { self.p2_led.hp_pct = pct; }
                    self.flush_leds();
                }
            }

            BoardEvent::SwitchIn { player_id, .. } => {
                if let Some(pid) = player_id {
                    let p = mega_blastoise_core::player_id_to_num(pid);
                    if p == 1 { self.p1_led.hp_pct = 100; }
                    else      { self.p2_led.hp_pct = 100; }
                    // Status LED is synced from player_data in on_prompt; don't reset here
                    // to avoid clobbering a status that persists through switching.
                    self.flush_leds();
                }
            }

            BoardEvent::Faint { mon, .. } => {
                if let Some(p) = mon_player_num(mon) {
                    if p == 1 { self.p1_led.hp_pct = 0; }
                    else      { self.p2_led.hp_pct = 0; }
                    crate::update_party_slot_hp(p, mon_display_name(mon), 0);
                    self.flush_leds();
                }
            }

            BoardEvent::SetStatus { mon, status } => {
                if let Some(p) = mon_player_num(mon) {
                    crate::update_party_slot_status(p, mon_display_name(mon), Some(status.clone()));
                    self.flush_leds();
                }
            }

            BoardEvent::CureStatus { mon, .. } => {
                if let Some(p) = mon_player_num(mon) {
                    crate::update_party_slot_status(p, mon_display_name(mon), None);
                    self.flush_leds();
                }
            }

            BoardEvent::SuperEffective { mon } => {
                if let Some(p) = mon_player_num(mon) { crate::set_flash(p, 1); }
            }

            BoardEvent::CriticalHit { mon } => {
                if let Some(p) = mon_player_num(mon) { crate::set_flash(p, 2); }
            }

            BoardEvent::Win { side } => {
                crate::update_leds(win_leds(BoardEvent::win_player_num(side)));
            }

            BoardEvent::Tie => {
                crate::update_leds(win_leds(0));
            }

            BoardEvent::Turn { .. } => {
                // Discard any queued button presses from the previous turn
                // so mashing doesn't cascade across turns.
                crate::clear_input_queues();
            }

            _ => {}
        }

        // ── Animation delay (same canonical per-event delay as firmware;
        //    sleep_ms is a no-op when :anim off) ────────────────────────────────
        let delay_ms = event.anim_delay_ms();
        if delay_ms > 0 {
            crate::sleep_ms(delay_ms).await;
        }

        // ── Log narration ─────────────────────────────────────────────────────
        if event.should_narrate() {
            let _ = self.bus.log.try_send(event.description());
        }
    }
}
