//! Single-source xorshift64 RNG used for all battle randomness.

#[derive(Clone, Copy, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed } }
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
