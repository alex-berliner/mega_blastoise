use std::fmt;

/// Host mirror of `mega_blastoise_fw::hp_bar::HpBarState`.
/// Uses `Display` instead of `defmt::Format`.
#[derive(Clone, Copy)]
pub struct HostHpBarState {
    pub current: u16,
    pub max: u16,
}

impl HostHpBarState {
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

    pub fn pct(&self) -> u32 {
        if self.max > 0 { self.current as u32 * 100 / self.max as u32 } else { 0 }
    }
}

impl fmt::Display for HostHpBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{} ({}%)", self.current, self.max, self.pct())
    }
}
