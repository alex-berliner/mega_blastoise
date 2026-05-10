use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use mega_blastoise_core::{
    render_player_screen, BoardEffects, BoardEvent, InputBus, MoveSlot,
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

// Status LED colors (match firmware; battler emits lowercase IDs)
fn status_color(status: &str) -> u32 {
    match status {
        "par" => pack_rgb(255, 200, 0),
        "brn" => pack_rgb(255, 60, 0),
        "frz" => pack_rgb(0, 200, 255),
        "psn" | "tox" => pack_rgb(150, 0, 200),
        "slp" => pack_rgb(0, 80, 0),
        _ => 0,
    }
}

// ── Per-player LED state ──────────────────────────────────────────────────────

struct LedPlayerState {
    hp_pct: u8,
    party: [Option<std::string::String>; 3],
    status: u32,
}

impl LedPlayerState {
    fn new() -> Self { Self { hp_pct: 100, party: [None, None, None], status: 0 } }

    fn alive_count(&self) -> usize {
        self.party.iter().filter(|s| s.is_some()).count()
    }

    fn register_switch(&mut self, name: &str) {
        if self.party.iter().any(|s| s.as_deref() == Some(name)) { return; }
        if let Some(slot) = self.party.iter_mut().find(|s| s.is_none()) {
            *slot = Some(name.to_string());
        }
    }

    fn register_faint(&mut self, name: &str) {
        for slot in &mut self.party {
            if slot.as_deref() == Some(name) { *slot = None; return; }
        }
    }

    fn render(&self) -> [u32; 12] {
        let mut buf = [0u32; 12];
        let lit = hp_lit(self.hp_pct);
        let color = hp_color(self.hp_pct);
        for i in 0..lit { buf[i] = color; }
        for i in 0..self.alive_count().min(3) {
            buf[8 + i] = pack_rgb(0, 160, 0);
        }
        buf[11] = self.status;
        buf
    }
}

fn build_led_frame(p1: &LedPlayerState, p2: &LedPlayerState) -> [u32; 24] {
    let mut frame = [0u32; 24];
    let b1 = p1.render();
    let b2 = p2.render();
    frame[..12].copy_from_slice(&b1);
    frame[12..].copy_from_slice(&b2);
    frame
}

// ── Per-player OLED state ─────────────────────────────────────────────────────

struct OledPlayerState {
    mon_name: std::string::String,
    moves: std::vec::Vec<MoveSlot>,
    hp_pct: u8,
}

impl OledPlayerState {
    fn new() -> Self {
        Self { mon_name: "---".into(), moves: std::vec::Vec::new(), hp_pct: 100 }
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
        crate::update_leds(build_led_frame(&s.p1_led, &s.p2_led));
        s
    }

    fn redraw(&mut self, player: u8) {
        if player == 1 {
            render_player_screen(
                &mut self.p1_disp,
                &self.p1_oled.mon_name,
                &self.p1_oled.moves,
                self.p1_oled.hp_pct,
            );
            crate::update_pixels(1, self.p1_disp.to_rgba());
        } else {
            render_player_screen(
                &mut self.p2_disp,
                &self.p2_oled.mon_name,
                &self.p2_oled.moves,
                self.p2_oled.hp_pct,
            );
            crate::update_pixels(2, self.p2_disp.to_rgba());
        }
    }

    fn flush_leds(&self) {
        crate::update_leds(build_led_frame(&self.p1_led, &self.p2_led));
    }
}

fn player_num(mon: &str) -> Option<u8> {
    let id = mon.split(',').nth(1)?.trim();
    if id == "p1" { Some(1) } else if id == "p2" { Some(2) } else { None }
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
    fn on_event(&mut self, event: BoardEvent) {
        match &event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let (Some(p), Some(pct)) = (player_num(mon), parse_hp_pct(health)) {
                    if p == 1 {
                        self.p1_oled.hp_pct = pct;
                        self.p1_led.hp_pct = pct;
                    } else {
                        self.p2_oled.hp_pct = pct;
                        self.p2_led.hp_pct = pct;
                    }
                    self.redraw(p);
                    self.flush_leds();
                }
            }

            BoardEvent::SwitchIn { name, player_id, moves, .. } => {
                if let Some(pid) = player_id {
                    let p = if pid == "p1" { 1u8 } else { 2u8 };
                    crate::update_moves(p, moves.clone());
                    if p == 1 {
                        self.p1_oled.mon_name = name.clone();
                        self.p1_oled.moves = moves.clone();
                        self.p1_oled.hp_pct = 100;
                        self.p1_led.hp_pct = 100;
                        self.p1_led.register_switch(name);
                    } else {
                        self.p2_oled.mon_name = name.clone();
                        self.p2_oled.moves = moves.clone();
                        self.p2_oled.hp_pct = 100;
                        self.p2_led.hp_pct = 100;
                        self.p2_led.register_switch(name);
                    }
                    self.redraw(p);
                    self.flush_leds();
                }
            }

            BoardEvent::MovesUpdate { player_id, moves } => {
                let p = if player_id == "p1" { 1u8 } else { 2u8 };
                crate::update_moves(p, moves.clone());
                if p == 1 { self.p1_oled.moves = moves.clone(); }
                else { self.p2_oled.moves = moves.clone(); }
                self.redraw(p);
            }

            BoardEvent::Faint { mon } => {
                let mon_name = mon.split(',').next().unwrap_or(mon.as_str()).trim();
                if let Some(p) = player_num(mon) {
                    if p == 1 {
                        self.p1_oled.hp_pct = 0;
                        self.p1_led.hp_pct = 0;
                        self.p1_led.register_faint(mon_name);
                        self.p1_led.status = 0;
                    } else {
                        self.p2_oled.hp_pct = 0;
                        self.p2_led.hp_pct = 0;
                        self.p2_led.register_faint(mon_name);
                        self.p2_led.status = 0;
                    }
                    self.redraw(p);
                    self.flush_leds();
                }
            }

            BoardEvent::SetStatus { mon, status } => {
                if let Some(p) = player_num(mon) {
                    let color = status_color(status);
                    if p == 1 { self.p1_led.status = color; }
                    else { self.p2_led.status = color; }
                    self.flush_leds();
                }
            }

            BoardEvent::CureStatus { mon, .. } => {
                if let Some(p) = player_num(mon) {
                    if p == 1 { self.p1_led.status = 0; }
                    else { self.p2_led.status = 0; }
                    self.flush_leds();
                }
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
            }

            BoardEvent::Tie => {
                draw_win_screen(&mut self.p1_disp, "TIE!");
                draw_win_screen(&mut self.p2_disp, "TIE!");
                crate::update_pixels(1, self.p1_disp.to_rgba());
                crate::update_pixels(2, self.p2_disp.to_rgba());
                crate::update_leds(win_leds(0));
            }

            BoardEvent::SuperEffective { mon } => {
                if let Some(p) = player_num(mon) { crate::set_flash(p, 1); }
            }

            BoardEvent::CriticalHit { mon } => {
                if let Some(p) = player_num(mon) { crate::set_flash(p, 2); }
            }

            _ => {}
        }

        let narrate = !matches!(
            &event,
            BoardEvent::Split { .. } | BoardEvent::Prompt { .. } | BoardEvent::MovesUpdate { .. }
        );
        if narrate {
            let _ = self.bus.log.try_send(event.description());
        }
    }
}
