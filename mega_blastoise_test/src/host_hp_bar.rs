use std::fmt;
use mega_blastoise_core::HpBarState;

pub struct HostHpBarState {
    pub current: u16,
    pub max: u16,
}

impl HostHpBarState {
    pub const ZERO: Self = Self { current: 0, max: 0 };

    pub fn parse(health: &str) -> Option<Self> {
        HpBarState::parse(health).map(|hp| Self { current: hp.current, max: hp.max })
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
