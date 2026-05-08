/// Host mirror of `mega_blastoise_fw::subsystems::oled`.
///
/// Each player gets a `CliDisplay` framebuffer driven by the shared rendering
/// functions from `mega_blastoise_core::display`.  On every state change the
/// display redraws and `CliDisplay::render()` prints it to stdout via Unicode
/// half-blocks.
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use mega_blastoise_core::{render_move_detail, render_player_screen, MoveSlot};

use crate::cli_display::CliDisplay;

pub struct OledPlayerState {
    pub header: &'static str,
    pub mon_name: String,
    pub moves: Vec<MoveSlot>,
}

impl OledPlayerState {
    fn new(header: &'static str) -> Self {
        Self { header, mon_name: "---".into(), moves: Vec::new() }
    }
}

pub struct HostOled {
    pub p1: OledPlayerState,
    pub p2: OledPlayerState,
    p1_disp: CliDisplay,
    p2_disp: CliDisplay,
    pub silent: bool,
}

impl HostOled {
    pub fn new() -> Self {
        Self {
            p1: OledPlayerState::new("P1: Red"),
            p2: OledPlayerState::new("P2: Blue"),
            p1_disp: CliDisplay::new(),
            p2_disp: CliDisplay::new(),
            silent: false,
        }
    }

    pub fn silent() -> Self {
        let mut s = Self::new();
        s.silent = true;
        s
    }

    pub fn switch_in(&mut self, player: u8, name: String, moves: Vec<MoveSlot>) {
        let s = self.player_mut(player);
        s.mon_name = name;
        s.moves = moves;
        self.redraw(player);
    }

    pub fn update_moves(&mut self, player: u8, moves: Vec<MoveSlot>) {
        self.player_mut(player).moves = moves;
        self.redraw(player);
    }

    pub fn faint(&mut self, player: u8) {
        let s = self.player_mut(player);
        s.mon_name = "FAINTED".into();
        s.moves.clear();
        self.redraw(player);
    }

    /// Show the move detail screen for `move_idx` on `player`'s display.
    pub fn show_move_detail(&mut self, player: u8, move_idx: usize) {
        if self.silent { return; }
        let (moves, disp, label) = if player == 1 {
            (&self.p1.moves, &mut self.p1_disp, "P1 Display")
        } else {
            (&self.p2.moves, &mut self.p2_disp, "P2 Display")
        };
        if let Some(mv) = moves.get(move_idx) {
            render_move_detail(disp, mv);
            println!("── {label} (move detail) ──────────────────────────────────────────────────────────────────────────────────────────────────────────");
            disp.render();
        }
    }

    pub fn win(&mut self, winner: u8) {
        if self.silent { return; }
        let (msg0, msg1) = match winner {
            1 => ("WINNER!", "GG!"),
            2 => ("GG!", "WINNER!"),
            _ => ("TIE!", "TIE!"),
        };
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        self.p1_disp.clear(BinaryColor::Off).ok();
        Text::with_baseline(msg0, Point::zero(), style, Baseline::Top)
            .draw(&mut self.p1_disp).ok();
        self.p2_disp.clear(BinaryColor::Off).ok();
        Text::with_baseline(msg1, Point::zero(), style, Baseline::Top)
            .draw(&mut self.p2_disp).ok();
        println!("── P1 Display ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
        self.p1_disp.render();
        println!("── P2 Display ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
        self.p2_disp.render();
    }

    fn player_mut(&mut self, player: u8) -> &mut OledPlayerState {
        if player == 1 { &mut self.p1 } else { &mut self.p2 }
    }

    fn redraw(&mut self, player: u8) {
        if self.silent { return; }
        let (mon_name, moves, disp, label) = if player == 1 {
            (
                self.p1.mon_name.as_str(),
                self.p1.moves.as_slice(),
                &mut self.p1_disp,
                "P1 Display",
            )
        } else {
            (
                self.p2.mon_name.as_str(),
                self.p2.moves.as_slice(),
                &mut self.p2_disp,
                "P2 Display",
            )
        };
        render_player_screen(disp, mon_name, moves);
        println!("── {label} ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
        disp.render();
    }
}

impl Default for HostOled {
    fn default() -> Self {
        Self::new()
    }
}
