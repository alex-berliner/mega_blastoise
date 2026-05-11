use mega_blastoise_core::{
    anim, hp_bar_color, hp_bar_count, mon_display_name, mon_player_id, player_id_to_num, render_event_text,
    render_player_screen, render_win_screen, BoardEffects, BoardEvent, HpBarState, InputBus,
    MoveSlot,
};

use crate::web_display::WasmDisplay;

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


// ── Per-player OLED state ─────────────────────────────────────────────────────

struct OledPlayerState {
    mon_name: std::string::String,
    moves: std::vec::Vec<MoveSlot>,
}

impl OledPlayerState {
    fn new() -> Self {
        Self { mon_name: "---".into(), moves: std::vec::Vec::new() }
    }
}

// ── Web battle effects ────────────────────────────────────────────────────────

pub struct WebBattleEffects<'a> {
    bus: &'a InputBus,
    p1_oled: OledPlayerState,
    p2_oled: OledPlayerState,
    p1_led: LedPlayerState,
    p2_led: LedPlayerState,
    p1_disp: WasmDisplay,
    p2_disp: WasmDisplay,
}

impl<'a> WebBattleEffects<'a> {
    pub fn new(bus: &'a InputBus) -> Self {
        let mut s = Self {
            bus,
            p1_oled: OledPlayerState::new(),
            p2_oled: OledPlayerState::new(),
            p1_led: LedPlayerState::new(),
            p2_led: LedPlayerState::new(),
            p1_disp: WasmDisplay::new(),
            p2_disp: WasmDisplay::new(),
        };
        s.redraw(1);
        s.redraw(2);
        s.flush_leds();
        s
    }

    fn redraw(&mut self, player: u8) {
        if player == 1 {
            render_player_screen(
                &mut self.p1_disp,
                &self.p1_oled.mon_name,
                &self.p1_oled.moves,
            );
            crate::update_pixels(1, self.p1_disp.to_rgba());
        } else {
            render_player_screen(
                &mut self.p2_disp,
                &self.p2_oled.mon_name,
                &self.p2_oled.moves,
            );
            crate::update_pixels(2, self.p2_disp.to_rgba());
        }
    }

    fn flush_leds(&self) {
        crate::update_hp_leds(1, self.p1_led.render());
        crate::sync_party_leds(1);
        crate::update_hp_leds(2, self.p2_led.render());
        crate::sync_party_leds(2);
    }

    fn flash_both(&mut self, text: &str) {
        render_event_text(&mut self.p1_disp, text);
        render_event_text(&mut self.p2_disp, text);
        // display_only: don't corrupt P_BATTLE_PIXELS — long-press restore must
        // return the real battle state, not the flash text.
        crate::display_only(1, self.p1_disp.to_rgba());
        crate::display_only(2, self.p2_disp.to_rgba());
    }

    fn redraw_both(&mut self) {
        self.redraw(1);
        self.redraw(2);
    }
}

fn player_num(mon: &str) -> Option<u8> {
    mon_player_id(mon).map(|id| player_id_to_num(id))
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
        let narrate = event.should_narrate();

        match &event {
            BoardEvent::Move { .. } => {
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::MOVE_MS).await;
                self.redraw_both();
            }

            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let (Some(p), Some(pct)) = (player_num(mon), HpBarState::parse(health).map(|h| h.pct())) {
                    if p == 1 { self.p1_led.hp_pct = pct; }
                    else      { self.p2_led.hp_pct = pct; }
                    self.redraw(p);
                    self.flush_leds();
                }
                crate::sleep_ms(anim::DAMAGE_MS).await;
            }

            BoardEvent::SwitchIn { name, player_id, moves, .. } => {
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::SWITCH_IN_MS / 2).await;
                if let Some(pid) = player_id {
                    let p = player_id_to_num(pid);
                    crate::update_moves(p, moves.clone());
                    crate::set_active_mon_name(p, name);
                    if p == 1 {
                        self.p1_oled.mon_name = name.clone();
                        self.p1_oled.moves = moves.clone();
                        self.p1_led.hp_pct = 100;
                    } else {
                        self.p2_oled.mon_name = name.clone();
                        self.p2_oled.moves = moves.clone();
                        self.p2_led.hp_pct = 100;
                    }
                    // Status LED is synced from player_data in on_prompt; don't reset here
                    // to avoid clobbering a status that persists through switching.
                    self.flush_leds();
                }
                self.redraw_both();
                crate::sleep_ms(anim::SWITCH_IN_MS / 2).await;
            }

            BoardEvent::MovesUpdate { player_id, moves } => {
                let p = player_id_to_num(player_id);
                crate::update_moves(p, moves.clone());
                if p == 1 { self.p1_oled.moves = moves.clone(); }
                else { self.p2_oled.moves = moves.clone(); }
                self.redraw(p);
            }

            BoardEvent::Faint { mon, .. } => {
                let desc = event.description();
                if let Some(p) = player_num(mon) {
                    if p == 1 { self.p1_led.hp_pct = 0; }
                    else      { self.p2_led.hp_pct = 0; }
                    crate::update_party_slot_hp(p, mon_display_name(mon), 0);
                    self.flush_leds();
                }
                self.flash_both(&desc);
                crate::sleep_ms(anim::FAINT_MS).await;
                self.redraw_both();
            }

            BoardEvent::SetStatus { mon, status } => {
                if let Some(p) = player_num(mon) {
                    crate::update_party_slot_status(p, mon_display_name(mon), Some(status.clone()));
                    self.flush_leds();
                }
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::EFFECT_MS).await;
                self.redraw_both();
            }

            BoardEvent::CureStatus { mon, .. } => {
                if let Some(p) = player_num(mon) {
                    crate::update_party_slot_status(p, mon_display_name(mon), None);
                    self.flush_leds();
                }
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::EFFECT_MS).await;
                self.redraw_both();
            }

            BoardEvent::SuperEffective { mon } => {
                if let Some(p) = player_num(mon) { crate::set_flash(p, 1); }
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::EFFECT_MS).await;
                self.redraw_both();
            }

            BoardEvent::CriticalHit { mon } => {
                if let Some(p) = player_num(mon) { crate::set_flash(p, 2); }
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::EFFECT_MS).await;
                self.redraw_both();
            }

            BoardEvent::Win { side } => {
                let winner = BoardEvent::win_player_num(side);
                let (msg1, msg2) = BoardEvent::win_messages(winner);
                render_win_screen(&mut self.p1_disp, msg1);
                render_win_screen(&mut self.p2_disp, msg2);
                crate::update_pixels(1, self.p1_disp.to_rgba());
                crate::update_pixels(2, self.p2_disp.to_rgba());
                crate::update_leds(win_leds(winner));
                crate::sleep_ms(anim::WIN_MS).await;
            }

            BoardEvent::Tie => {
                render_win_screen(&mut self.p1_disp, "TIE!");
                render_win_screen(&mut self.p2_disp, "TIE!");
                crate::update_pixels(1, self.p1_disp.to_rgba());
                crate::update_pixels(2, self.p2_disp.to_rgba());
                crate::update_leds(win_leds(0));
                crate::sleep_ms(anim::WIN_MS).await;
            }

            BoardEvent::Miss { .. }
            | BoardEvent::Immune { .. }
            | BoardEvent::Resisted { .. }
            | BoardEvent::Cant { .. }
            | BoardEvent::Fail { .. } => {
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::BRIEF_MS).await;
                self.redraw_both();
            }

            BoardEvent::Turn { .. } => {
                // Discard any queued button presses from the previous turn
                // so mashing doesn't cascade across turns.
                crate::clear_input_queues();
            }

            _ => {}
        }

        if narrate {
            let _ = self.bus.log.try_send(event.description());
        }
    }
}
