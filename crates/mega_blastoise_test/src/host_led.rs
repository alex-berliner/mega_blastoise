/// Host mirror of `mega_blastoise_fw::subsystems::led`.
///
/// Tracks LED state in memory and prints text descriptions instead of
/// driving WS2812B hardware.
pub struct HostLed {
    pub p1: PlayerLedDisplay,
    pub p2: PlayerLedDisplay,
    pub silent: bool,
}

pub struct PlayerLedDisplay {
    pub hp_pct: u8,
    pub alive_count: u8,
    pub status: Option<String>,
}

impl PlayerLedDisplay {
    fn new() -> Self {
        Self { hp_pct: 100, alive_count: 3, status: None }
    }


    fn render_bar(&self) -> String {
        let lit = if self.hp_pct == 0 { 0 } else { (self.hp_pct as usize * 8 + 99) / 100 };
        let color = if self.hp_pct > 50 { "G" } else if self.hp_pct > 25 { "Y" } else { "R" };
        format!(
            "HP [{}{}] {}%  party:[{}{}{}]  status:{}",
            color.repeat(lit),
            ".".repeat(8usize.saturating_sub(lit)),
            self.hp_pct,
            if self.alive_count >= 1 { "●" } else { "○" },
            if self.alive_count >= 2 { "●" } else { "○" },
            if self.alive_count >= 3 { "●" } else { "○" },
            self.status.as_deref().unwrap_or("—"),
        )
    }
}

impl HostLed {
    pub fn new() -> Self {
        Self { p1: PlayerLedDisplay::new(), p2: PlayerLedDisplay::new(), silent: false }
    }

    pub fn silent() -> Self {
        let mut s = Self::new();
        s.silent = true;
        s
    }

    pub fn update_hp(&mut self, player: u8, pct: u8) {
        self.player_mut(player).hp_pct = pct;
        self.print(player);
    }

    pub fn faint(&mut self, player: u8) {
        let p = self.player_mut(player);
        p.hp_pct = 0;
        if p.alive_count > 0 { p.alive_count -= 1; }
        p.status = None;
        self.print(player);
    }

    pub fn set_status(&mut self, player: u8, status: &str) {
        self.player_mut(player).status = Some(status.to_string());
        self.print(player);
    }

    pub fn cure_status(&mut self, player: u8) {
        self.player_mut(player).status = None;
        self.print(player);
    }

    pub fn win(&mut self, winner: u8) {
        if !self.silent {
            let (s1, s2) = match winner {
                1 => ("[GOLD ●●●●●●●●]", "[dim  ........]"),
                2 => ("[dim  ........]", "[GOLD ●●●●●●●●]"),
                _ => ("[grey ████████]", "[grey ████████]"),
            };
            println!("[LED] P1: {s1}  P2: {s2}");
        }
    }

    fn player_mut(&mut self, player: u8) -> &mut PlayerLedDisplay {
        if player == 1 { &mut self.p1 } else { &mut self.p2 }
    }

    fn print(&self, player: u8) {
        if !self.silent {
            let label = if player == 1 { "P1" } else { "P2" };
            let state = if player == 1 { &self.p1 } else { &self.p2 };
            println!("[LED] {label}: {}", state.render_bar());
        }
    }
}

impl Default for HostLed {
    fn default() -> Self { Self::new() }
}
