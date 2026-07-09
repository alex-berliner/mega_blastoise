/// Host mirror of `mega_blastoise_fw::subsystems::oled`.
///
/// Screen decisions come from the same `mega_blastoise_core::oled_ctl`
/// state machine the firmware and web client use; this file only renders
/// the controller's current screen into a `CliDisplay` and prints it to
/// stdout via Unicode half-blocks.
use mega_blastoise_core::{name_buf, render_screen, MoveSlot, OledCmd, OledController};

use crate::cli_display::CliDisplay;

pub struct HostOled {
    ctl: OledController,
    p1_disp: CliDisplay,
    p2_disp: CliDisplay,
    pub silent: bool,
}

impl HostOled {
    pub fn new() -> Self {
        Self {
            ctl: OledController::new(),
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
        let (buf, len) = name_buf(&name);
        self.apply(OledCmd::ActiveMon { player, name: buf, len, speed: 150 });
        if !moves.is_empty() {
            self.apply(OledCmd::MovesUpdate { player, moves });
        }
    }

    pub fn update_moves(&mut self, player: u8, moves: Vec<MoveSlot>) {
        self.apply(OledCmd::MovesUpdate { player, moves });
    }

    pub fn update_hp(&mut self, player: u8, pct: u8) {
        self.apply(OledCmd::HpUpdate { player, pct });
    }

    pub fn faint(&mut self, player: u8) {
        self.apply(OledCmd::Faint { player });
    }

    /// Show the move detail screen for `move_idx` on `player`'s display.
    pub fn show_move_detail(&mut self, player: u8, move_idx: usize) {
        self.apply(OledCmd::ShowMoveDetail { player, slot: move_idx as u8 });
    }

    pub fn win(&mut self, winner: u8) {
        self.apply(OledCmd::Win { winner });
    }

    fn apply(&mut self, cmd: OledCmd) {
        let redraw = self.ctl.apply(cmd);
        if self.silent {
            return;
        }
        for player in [1u8, 2] {
            if redraw.includes(player) {
                let disp = if player == 1 { &mut self.p1_disp } else { &mut self.p2_disp };
                render_screen(disp, &self.ctl.screen(player));
                println!("── P{player} Display ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────");
                disp.render();
            }
        }
    }
}

impl Default for HostOled {
    fn default() -> Self {
        Self::new()
    }
}
