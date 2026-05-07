/// Host mirror of `mega_blastoise_fw::subsystems::oled`.
///
/// Maintains in-memory display state and prints updates to stdout, mirroring
/// what the firmware would render on the SSD1306 OLEDs.
pub struct OledPlayerState {
    pub header: &'static str,
    pub mon_name: String,
    pub hp_pct: u8,
    pub fainted: bool,
}

impl OledPlayerState {
    fn new(header: &'static str) -> Self {
        Self { header, mon_name: "---".into(), hp_pct: 100, fainted: false }
    }

    fn render(&self) -> String {
        let bar_filled = (self.hp_pct as usize * 20 / 100).min(20);
        let bar: String = "█".repeat(bar_filled) + &"░".repeat(20 - bar_filled);
        let label = if self.fainted { "FAINTED".into() } else { self.mon_name.clone() };
        format!("[{}] {}  HP {}% [{}]", self.header, label, self.hp_pct, bar)
    }
}

pub struct HostOled {
    pub p1: OledPlayerState,
    pub p2: OledPlayerState,
    pub silent: bool,
}

impl HostOled {
    pub fn new() -> Self {
        Self {
            p1: OledPlayerState::new("P1: Red"),
            p2: OledPlayerState::new("P2: Blue"),
            silent: false,
        }
    }

    pub fn silent() -> Self {
        let mut s = Self::new();
        s.silent = true;
        s
    }

    pub fn update_hp(&mut self, player: u8, pct: u8) {
        let s = self.player_mut(player);
        s.hp_pct = pct;
        self.print(player);
    }

    pub fn active_mon(&mut self, player: u8, name: impl Into<String>) {
        let s = self.player_mut(player);
        s.mon_name = name.into();
        s.fainted = false;
        self.print(player);
    }

    pub fn faint(&mut self, player: u8) {
        self.player_mut(player).fainted = true;
        self.print(player);
    }

    pub fn win(&mut self, winner: u8) {
        if !self.silent {
            let (msg0, msg1) = match winner {
                1 => ("WINNER!", "GG!"),
                2 => ("GG!", "WINNER!"),
                _ => ("TIE!", "TIE!"),
            };
            println!("[OLED] P1: {msg0} | P2: {msg1}");
        }
    }

    fn player_mut(&mut self, player: u8) -> &mut OledPlayerState {
        if player == 1 { &mut self.p1 } else { &mut self.p2 }
    }

    fn print(&self, player: u8) {
        if !self.silent {
            let state = if player == 1 { &self.p1 } else { &self.p2 };
            println!("[OLED] {}", state.render());
        }
    }
}

impl Default for HostOled {
    fn default() -> Self {
        Self::new()
    }
}
