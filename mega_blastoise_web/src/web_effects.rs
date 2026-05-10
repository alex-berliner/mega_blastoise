use embedded_graphics::{
    mono_font::{ascii::{FONT_6X10, FONT_5X8}, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use mega_blastoise_core::{
    anim, render_player_screen, BoardEffects, BoardEvent, InputBus, MoveSlot,
};

use crate::web_display::WasmDisplay;

// ── LED helpers ───────────────────────────────────────────────────────────────

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn hp_color(pct: u8) -> u32 {
    if pct > 50 { pack_rgb(0, 180, 0) }
    else if pct > 25 { pack_rgb(180, 150, 0) }
    else { pack_rgb(200, 0, 0) }
}

fn hp_lit(pct: u8) -> usize {
    if pct == 0 { return 0; }
    ((pct as usize * 8 + 99) / 100).min(8)
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
        let lit = hp_lit(self.hp_pct);
        let color = hp_color(self.hp_pct);
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
        draw_event_text(&mut self.p1_disp, text);
        draw_event_text(&mut self.p2_disp, text);
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
    let id = mon.split(',').nth(1)?.trim();
    if id == "p1" { Some(1) } else if id == "p2" { Some(2) } else { None }
}

fn mon_name(mon: &str) -> &str {
    mon.split(',').next().unwrap_or(mon)
}

fn parse_hp_pct(health: &str) -> Option<u8> {
    let health = health.trim();
    if let Some((cur, max)) = health.split_once('/') {
        let cur: u32 = cur.trim().parse().ok()?;
        let max: u32 = max.trim().parse().ok()?;
        if max > 0 { Some((cur * 100 / max) as u8) } else { Some(0) }
    } else {
        let v: u8 = health.parse().ok()?;
        Some(v)
    }
}

fn win_leds(winner: u8) -> [u32; 24] {
    let gold = pack_rgb(200, 150, 0);
    let dim  = pack_rgb(40, 0, 0);
    let grey = pack_rgb(60, 60, 60);
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

fn draw_event_text(disp: &mut WasmDisplay, text: &str) {
    disp.clear_all();
    let style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let ts = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();

    // Target chars per line: evenly divide for long text, 21-char cap for short.
    let target = if text.len() > 25 { (text.len() + 2) / 3 } else { 21 };

    let mut lines = [""; 3];
    let mut n = 0usize;
    let mut rest = text;
    while !rest.is_empty() && n < 3 {
        if rest.len() <= target || n == 2 {
            lines[n] = rest;
            n += 1;
            break;
        }
        let search_end = (target + 4).min(rest.len());
        let at = rest[..search_end].rfind(' ').unwrap_or(target.min(rest.len()));
        lines[n] = rest[..at].trim();
        n += 1;
        rest = rest[at..].trim_start();
    }

    let start_y: i32 = match n {
        1 => 28,
        2 => 23,
        _ => 17,
    };
    for i in 0..n {
        if !lines[i].is_empty() {
            Text::with_text_style(lines[i], Point::new(64, start_y + i as i32 * 10), style, ts)
                .draw(disp).ok();
        }
    }
}

fn draw_win_screen(disp: &mut WasmDisplay, msg: &str) {
    disp.clear_all();
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let ts = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    Text::with_text_style(msg, Point::new(64, 27), style, ts)
        .draw(disp)
        .ok();
}

impl BoardEffects for WebBattleEffects<'_> {
    async fn on_event(&mut self, event: BoardEvent) {
        let narrate = !matches!(
            &event,
            BoardEvent::Split { .. } | BoardEvent::Prompt { .. } | BoardEvent::MovesUpdate { .. }
        );

        match &event {
            BoardEvent::Move { .. } => {
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::MOVE_MS).await;
                self.redraw_both();
            }

            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let (Some(p), Some(pct)) = (player_num(mon), parse_hp_pct(health)) {
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
                    let p = if pid == "p1" { 1u8 } else { 2u8 };
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
                let p = if player_id == "p1" { 1u8 } else { 2u8 };
                crate::update_moves(p, moves.clone());
                if p == 1 { self.p1_oled.moves = moves.clone(); }
                else { self.p2_oled.moves = moves.clone(); }
                self.redraw(p);
            }

            BoardEvent::Faint { mon } => {
                let desc = event.description();
                if let Some(p) = player_num(mon) {
                    if p == 1 { self.p1_led.hp_pct = 0; }
                    else      { self.p2_led.hp_pct = 0; }
                    crate::update_party_slot_hp(p, mon_name(mon), 0);
                    self.flush_leds();
                }
                self.flash_both(&desc);
                crate::sleep_ms(anim::FAINT_MS).await;
                self.redraw_both();
            }

            BoardEvent::SetStatus { mon, status } => {
                if let Some(p) = player_num(mon) {
                    crate::update_party_slot_status(p, mon_name(mon), Some(status.clone()));
                    self.flush_leds();
                }
                let desc = event.description();
                self.flash_both(&desc);
                crate::sleep_ms(anim::EFFECT_MS).await;
                self.redraw_both();
            }

            BoardEvent::CureStatus { mon, .. } => {
                if let Some(p) = player_num(mon) {
                    crate::update_party_slot_status(p, mon_name(mon), None);
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
                let winner = match side.as_deref() {
                    Some("0") => 1u8,
                    Some("1") => 2u8,
                    _ => 0,
                };
                let (msg1, msg2) = match winner {
                    1 => ("WINNER!", "GG!"),
                    2 => ("GG!", "WINNER!"),
                    _ => ("TIE!", "TIE!"),
                };
                draw_win_screen(&mut self.p1_disp, msg1);
                draw_win_screen(&mut self.p2_disp, msg2);
                crate::update_pixels(1, self.p1_disp.to_rgba());
                crate::update_pixels(2, self.p2_disp.to_rgba());
                crate::update_leds(win_leds(winner));
                crate::sleep_ms(anim::WIN_MS).await;
            }

            BoardEvent::Tie => {
                draw_win_screen(&mut self.p1_disp, "TIE!");
                draw_win_screen(&mut self.p2_disp, "TIE!");
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
