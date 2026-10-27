/// Parsed HP state from a protocol health string (`"current/max"` or bare `"current"`).
#[derive(Clone, Copy)]
pub struct HpBarState {
    pub current: u16,
    pub max: u16,
}

impl HpBarState {
    pub const ZERO: Self = Self { current: 0, max: 0 };

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

    pub fn pct(&self) -> u8 {
        if self.max == 0 {
            return 0;
        }
        let pct = (self.current as u32 * 100 / self.max as u32) as u8;
        // Never round a living mon down to 0%: 0 means fainted everywhere
        // downstream.
        if pct == 0 && self.current > 0 { 1 } else { pct }
    }
}

impl core::fmt::Display for HpBarState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{} ({}%)", self.current, self.max, self.pct())
    }
}
