/// HP state for one active Pokémon.
#[derive(Clone, Copy)]
pub struct HpBarState {
    pub current: u16,
    pub max: u16,
}

impl HpBarState {
    pub const ZERO: Self = Self { current: 0, max: 0 };

    /// Parse battler health string: `"current/max"` or bare `"current"` (fainted = 0/1).
    pub fn parse(health: &str) -> Option<Self> {
        let health = health.trim();
        if let Some((cur, max)) = health.split_once('/') {
            Some(Self {
                current: cur.trim().parse().ok()?,
                max: max.trim().parse().ok()?,
            })
        } else {
            let current: u16 = health.parse().ok()?;
            Some(Self { current, max: current.max(1) })
        }
    }
}

impl defmt::Format for HpBarState {
    fn format(&self, f: defmt::Formatter) {
        let pct = if self.max > 0 {
            self.current as u32 * 100 / self.max as u32
        } else {
            0
        };
        defmt::write!(f, "{}/{} ({}%)", self.current, self.max, pct);
    }
}
