/// Number of HP-bar LEDs to light (out of 8) for the given percentage.
pub fn hp_bar_count(pct: u8) -> usize {
    if pct == 0 { return 0; }
    ((pct as usize * 8 + 99) / 100).min(8)
}

/// HP-bar color as `(r, g, b)`: green > 50%, yellow > 25%, red ≤ 25%.
pub fn hp_bar_color(pct: u8) -> (u8, u8, u8) {
    if pct > 50      { (0, 180, 0) }
    else if pct > 25 { (180, 150, 0) }
    else             { (200, 0, 0) }
}

/// Parsed HP state from a battler health string (`"current/max"` or bare `"current"`).
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
        if self.max > 0 { (self.current as u32 * 100 / self.max as u32) as u8 } else { 0 }
    }
}
