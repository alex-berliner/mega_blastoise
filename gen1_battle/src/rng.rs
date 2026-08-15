//! Single-source xorshift64 RNG used for all battle randomness.
//!
//! The RNG can also carry a per-seat script of forced outcomes, which is how
//! the Showdown parity and fuzz suites drive whole turns down the same branch
//! they forced on the reference simulator. In play `force` is None and every
//! roll is a real roll.

/// One seat's forced outcomes for a scripted turn. None on a field means
/// that roll happens normally.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeatForce {
    pub hit: Option<bool>,
    pub crit: Option<bool>,
    /// The 217..=255 damage factor.
    pub roll: Option<u8>,
    /// Secondary-class chances: status/confusion/flinch procs, stat-drop
    /// chances, Twineedle's poison.
    pub secondary: Option<bool>,
    /// Full paralysis.
    pub immobile: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct Rng {
    state: u64,
    /// Scripted outcomes for the parity suites; None in play.
    pub force: Option<[SeatForce; 2]>,
    /// Whose action is resolving, so forced channels resolve per seat.
    pub acting: usize,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed }, force: None, acting: 0 }
    }

    fn seat(&self) -> Option<SeatForce> {
        self.force.map(|f| f[self.acting.min(1)])
    }

    pub fn forced_hit(&self) -> Option<bool> {
        self.seat().and_then(|s| s.hit)
    }

    pub fn forced_crit(&self) -> Option<bool> {
        self.seat().and_then(|s| s.crit)
    }

    pub fn forced_roll(&self) -> Option<u8> {
        self.seat().and_then(|s| s.roll)
    }

    pub fn forced_secondary(&self) -> Option<bool> {
        self.seat().and_then(|s| s.secondary)
    }

    pub fn forced_immobile(&self) -> Option<bool> {
        self.seat().and_then(|s| s.immobile)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a byte in `0..=255`.
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    /// Returns a value in `0..n` (n must be > 0).
    pub fn range(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }

    /// 50/50 coin flip.
    pub fn coin(&mut self) -> bool {
        (self.next_u64() & 1) == 0
    }
}
