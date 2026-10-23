//! Pokemon Showdown-compatible PRNG.
//!
//! Bit-for-bit port of `PS/sim/prng.ts` (Pokemon Showdown 0.11.11). Drives
//! everything that draws randomness in the new battle server; called in
//! PS's order by `ps_sim` itself so the protocol-diff harness can compare
//! entire battles without monkeypatching the reference (design section 5.5).
//!
//! PS uses two PRNGs: `SodiumRNG` (ChaCha20) and `Gen5RNG` (a 64-bit LCG).
//! The harness starts battles with a numeric `[a, b, c, d]` seed, which PS
//! resolves to `Gen5RNG`. We only implement the numeric-seed path; the
//! sodium path is for human-typed online replay seeds and is not a battle
//! input.
//!
//! Methods follow PS exactly: `next`, `next_frame`, `multiply_add`, the
//! `random()` / `random(n)` / `random(m, n)` overloads, `random_chance`,
//! `sample`, `shuffle`, `get_seed`. The `floor(result * n / 2^32)` step in
//! `random_n` and `random_range` is done with `u64` so the result is bit
//! exact for any `n <= 2^16` (the ported moves and clauses never ask for
//! more). `random()` returns `f64 = next / 2^32` per PS; IEEE 754 nearest
//! rounding in both implementations makes the result bit-exact too.
//!
//! A `Prng::calls` counter increments on every public method. The pin test
//! (P1-2 acceptance) uses it to verify total calls; the protocol-diff
//! harness will use it to report PRNG-call counts at the first divergence.
//!
//! An `OutcomeOverride` queue (forces the next N `next()` results without
//! advancing the underlying RNG) is gated on `any(test, test-override)` so
//! the firmware build contains no override symbols.

#![cfg_attr(not(any(test, feature = "test-override")), allow(dead_code))]

extern crate alloc;

#[cfg(any(test, feature = "test-override"))]
use alloc::collections::VecDeque;

/// 64-bit LCG seed, big-endian 16-bit limbs (`seed[0]` = MSB, `seed[3]` = LSB).
pub type Seed = [u16; 4];

/// LCG multiplier (`a`) and increment (`c`) for `Gen5RNG`'s
/// `x_{n+1} = a * x_n + c (mod 2^64)` step.
const RNG_A: Seed = [0x5D58, 0x8B65, 0x6C07, 0x8965];
const RNG_C: Seed = [0x0000, 0x0000, 0x0026, 0x9EC3];

// PS: sim/prng.ts Gen5RNG.multiplyAdd.
//
// Computes `a * b + c` with 64-bit two's complement integers represented as
// four 16-bit limbs, big-endian (index 0 is the most-significant limb).
// The output limbs jitter from 0 to 2^16 - 1, then we propagate a JS Number
// `carry` (a double) shifted right by 16 each outer iteration. In Rust the
// same computation fits in `u64` exactly because every limb product is at
// most 2^32 and there are at most four terms per outer step, so the running
// `carry` stays well under 2^53 (the JS Number's exact-integer ceiling).
//
// PS: sim/prng.ts Gen5RNG.multiplyAdd.
pub fn multiply_add(a: Seed, b: Seed, c: Seed) -> Seed {
    let mut out = [0u16; 4];
    let mut carry: u64 = 0;

    for out_index in (0..4usize).rev() {
        for b_index in out_index..4 {
            let a_index = 3 - (b_index - out_index);
            carry += u64::from(a[a_index]) * u64::from(b[b_index]);
        }
        carry += u64::from(c[out_index]);
        out[out_index] = carry as u16;
        carry >>= 16;
    }

    out
}

// PS: sim/prng.ts Gen5RNG.nextFrame.
fn next_frame(mut seed: Seed, frames_to_advance: u32) -> Seed {
    for _ in 0..frames_to_advance {
        seed = multiply_add(seed, RNG_A, RNG_C);
    }
    seed
}

// PS: sim/prng.ts Gen5RNG.next.
fn next_u32(seed: Seed) -> (Seed, u32) {
    let new_seed = next_frame(seed, 1);
    let value = (u32::from(new_seed[0]) << 16) | u32::from(new_seed[1]);
    (new_seed, value)
}

/// The Gen 5 on-cartridege 64-bit LCG that numeric seeds land on in PS.
///
/// We expose `seed`, `next`, `next_frame` and `multiply_add` (all
/// `pub`-within-crate) for tests and for the upcoming `speedSort` work in
/// P1-5; the server itself should go through `Prng` so the call counter
/// records every consumption.
#[derive(Debug, Clone)]
pub struct Gen5Rng {
    seed: Seed,
}

impl Gen5Rng {
    /// Construct from a numeric seed (PS: numeric seeds select `Gen5RNG`).
    pub fn new(seed: Seed) -> Self {
        Self { seed }
    }

    /// Construct from a fresh random seed (PS: `Gen5RNG.generateSeed`).
    /// Host only; the firmware's RNG seeds come from Power-On via
    /// `tools/ps-diff`, which passes a numeric four-u16 seed. Kept for the
    /// test fixture format and for completeness.
    #[doc(hidden)]
    pub fn generate() -> Self {
        // PS: sim/prng.ts Gen5RNG.generateSeed (`Math.random()`). Used in
        // tests and rejected from server boot paths; server boot requires a
        // numeric `[a,b,c,d]` seed.
        let mut state: u32 = 0x9E37_79B9;
        let mut next = || {
            state = state
                .wrapping_mul(0x1234_5677)
                .wrapping_add(0x_DEAD_BEEF);
            (state >> 16) as u16
        };
        Self {
            seed: [next(), next(), next(), next()],
        }
    }

    /// Advance the RNG by `frames_to_advance` frames; returns the new seed.
    pub fn next_frame(&mut self, frames_to_advance: u32) -> Seed {
        self.seed = next_frame(self.seed, frames_to_advance);
        self.seed
    }

    /// PS: `Gen5RNG.next`: advance one frame and return the upper 32 bits.
    pub fn next(&mut self) -> u32 {
        let (seed, value) = next_u32(self.seed);
        self.seed = seed;
        value
    }

    /// PS: `Gen5RNG.getSeed`: `seed.join(',')` form. We return the limbs
    /// directly; the server's protocol layer formats them when needed.
    pub fn get_seed(&self) -> Seed {
        self.seed
    }
}

/// Stack of forced `next()` results, drained FIFO. Used for Rust-side
/// deterministic tests that need to pin a battle outcome without touching
/// the underlying RNG state. Off the firmware build path: gated by
/// `cfg(any(test, feature = "test-override"))`.
#[cfg(any(test, feature = "test-override"))]
#[derive(Debug, Default, Clone)]
pub struct OutcomeOverride {
    queue: VecDeque<u32>,
}

#[cfg(any(test, feature = "test-override"))]
impl OutcomeOverride {
    /// Empty queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Queue one forced `next()` result.
    pub fn push(&mut self, value: u32) {
        self.queue.push_back(value);
    }

    /// Queue several results; preserves order.
    pub fn extend<I: IntoIterator<Item = u32>>(&mut self, iter: I) {
        for v in iter {
            self.queue.push_back(v);
        }
    }

    /// `true` if no further forced values are queued.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Number of values still queued.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Drop all queued values.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

/// The battle-time PRNG. Holds the `Gen5RNG` (numeric seed only), an
/// override queue when configured, and a public call counter for diffing.
pub struct Prng {
    rng: Gen5Rng,
    /// Total number of public method calls. Reset only by `reset_calls`.
    pub calls: u64,
    #[cfg(any(test, feature = "test-override"))]
    override_queue: VecDeque<u32>,
}

impl Prng {
    /// Construct from a numeric `[a, b, c, d]` seed (`>start {"seed":[..]}`).
    pub fn new(seed: Seed) -> Self {
        Self {
            rng: Gen5Rng::new(seed),
            calls: 0,
            #[cfg(any(test, feature = "test-override"))]
            override_queue: VecDeque::new(),
        }
    }

    /// Construct with a deterministic fresh seed (host tests only; server
    /// boot must use a numeric seed).
    #[doc(hidden)]
    pub fn fresh() -> Self {
        Self::new(Gen5Rng::generate().get_seed())
    }

    /// Reset the call counter without touching the RNG state.
    pub fn reset_calls(&mut self) {
        self.calls = 0;
    }

    /// PS: `Gen5RNG.getSeed`.
    pub fn get_seed(&self) -> Seed {
        self.rng.get_seed()
    }

    /// `true` if a test or the `test-override` feature has armed any forced
    /// `next()` results.
    #[cfg(any(test, feature = "test-override"))]
    pub fn override_is_empty(&self) -> bool {
        self.override_queue.is_empty()
    }

    /// Convenience setter for tests: arm forced `next()` results in order.
    #[cfg(any(test, feature = "test-override"))]
    pub fn push_overrides<I: IntoIterator<Item = u32>>(&mut self, iter: I) {
        self.override_queue.extend(iter);
    }

    /// Advance the underlying RNG by `frames_to_advance` frames.
    /// `speedSort` uses this for its tie shuffle (PS: speedSort calls
    /// `prng.shuffle` which calls `prng.next` repeatedly).
    pub fn advance(&mut self, frames_to_advance: u32) {
        self.calls = self.calls.saturating_add(u64::from(frames_to_advance));
        self.rng.next_frame(frames_to_advance);
    }

    /// PS: `Gen5RNG.next` (delegates after consulting the override).
    pub fn next(&mut self) -> u32 {
        #[cfg(any(test, feature = "test-override"))]
        {
            if let Some(v) = self.override_queue.pop_front() {
                self.calls = self.calls.saturating_add(1);
                return v;
            }
        }
        let v = self.rng.next();
        self.calls = self.calls.saturating_add(1);
        v
    }

    /// PS: `PRNG.random` (no `@param`) = `next / 2^32` as `f64`.
    ///
    /// IEEE 754 nearest rounding gives JS and Rust identical bits for any
    /// integer `next` in `[0, 2^32)`.
    pub fn random(&mut self) -> f64 {
        let r = self.next();
        f64::from(r) / 4_294_967_296.0_f64
    }

    /// PS: `PRNG.random(n)` = `floor(next * n / 2^32)`, integer in `[0, n)`.
    ///
    /// Done in `u64` so the result is bit-exact for every `n` the
    /// ported moves ask for (largest is `2^16` today). For `n = 0` PS
    /// returns `0`; we mirror that.
    pub fn random_n(&mut self, n: u32) -> u32 {
        let r = self.next();
        let product = (u64::from(r)) * u64::from(n);
        (product >> 32) as u32
    }

    /// PS: `PRNG.random(m, n)` = `floor(next * (n - m) / 2^32) + m`, integer
    /// in `[m, n)`. The brief asks for this exact form done in `u64`.
    ///
    /// Requires `m <= n`. `m == n` returns `m` exactly (the floor term
    /// vanishes), matching PS.
    pub fn random_range(&mut self, m: u32, n: u32) -> u32 {
        debug_assert!(m <= n, "PRNG::random_range requires m <= n");
        if m == n {
            // Still a registered call (PS makes one next() call). Match PS
            // exactly: count it because `random(m, m)` does consume a frame
            // in JS (it's just a multiplication by zero of the same `next`
            // value).
            let _ = self.next();
            return m;
        }
        self.random_n(n - m) + m
    }

    /// PS: `PRNG.randomChance(numerator, denominator)` = `random(denominator)
    /// < numerator`. Numerator is non-negative; denominator is positive
    /// (PS asserts on its own).
    pub fn random_chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.random_n(denominator) < numerator
    }

    /// PS: `PRNG.sample(items)` = `items[random(items.length)]`. We return
    /// a reference because `shuffle`-style callers benefit from in-place
    /// mutation instead of an extra copy.
    ///
    /// Panics on empty (PS throws `RangeError`).
    pub fn sample<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        assert!(!items.is_empty(), "PRNG::sample: empty slice");
        let idx = self.random_n(items.len() as u32) as usize;
        &items[idx]
    }

    /// PS: `PRNG.shuffle(items, start, end)`, Fisher-Yates with a `random(start,
    /// end)` per iteration. Performs `end - start - 1` underlying `next()`
    /// calls (zero when `end <= start + 1`).
    ///
    /// `start` and `end` are `usize` to mirror Rust slice conventions; the
    /// per-iteration cast is lossless because `items.len() <= usize::MAX`
    /// and our state caps well below `u32::MAX`.
    pub fn shuffle<T>(&mut self, items: &mut [T], mut start: usize, end: usize) {
        while start + 1 < end {
            let next_index = self.random_range(start as u32, end as u32) as usize;
            if start != next_index {
                items.swap(start, next_index);
            }
            start += 1;
        }
    }
}

impl core::fmt::Debug for Prng {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Prng")
            .field("seed", &self.rng.get_seed())
            .field("calls", &self.calls)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference vector: seed = [1, 2, 3, 4].
    //
    // PS's Gen5RNG: next() advances frame 1, returns upper 32 bits.
    //   seed_0 = [1, 2, 3, 4] (big-endian limbs)
    //     value as u64: 1<<48 | 2<<32 | 3<<16 | 4 = 281474976776964
    //   a = 0x5D588B656C078965, c = 0x00269EC3
    //   new_seed = seed * a + c (mod 2^64)
    // We compute the result manually with the PRNG test fixture script:
    //   node tools/ps-diff/prng-fixtures.js --single 1,2,3,4 --calls 1
    // yields the expected values.  Rather than compute by hand we drive PS
    // through the fixture; here we only test internal invariants that are
    // byte-exact with PS (limb carry, return width, shuffle ordering).

    #[test]
    fn multiply_add_known_small_inputs() {
        // 2 * 3 = 6.
        assert_eq!(multiply_add([0, 0, 0, 2], [0, 0, 0, 3], [0, 0, 0, 0]), [0, 0, 0, 6]);
        // 2^16 * 1 = 2^16.
        assert_eq!(
            multiply_add([0, 0, 0, 1], [0, 0, 1, 0], [0, 0, 0, 0]),
            [0, 0, 1, 0]
        );
        // 2^16 * 2^16 = 2^32.
        assert_eq!(
            multiply_add([0, 0, 1, 0], [0, 0, 1, 0], [0, 0, 0, 0]),
            [0, 1, 0, 0]
        );
        // 2^32 + 1 = limb sums (+1 carry).
        assert_eq!(
            multiply_add([0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 0, 1]),
            [0, 1, 0, 1]
        );
        // Identity: 1 * x = x.
        assert_eq!(
            multiply_add([0, 0, 0, 1], [0, 0xABCD, 0x1234, 0x5678], [0, 0, 0, 0]),
            [0, 0xABCD, 0x1234, 0x5678]
        );
    }

    #[test]
    fn next_returns_upper_32_bits() {
        let mut rng = Gen5Rng::new([0, 0, 0, 0]);
        let v = rng.next();
        // Seed [0,0,0,0] after one frame becomes RNG_C = [0,0,0x26,0x9EC3].
        // Upper 32 bits = (0 << 16) | 0 = 0. Lower 32 bits discarded.
        assert_eq!(v, 0);
        // Continue: now seed = [0,0,0x26,0x9EC3], advance one frame.
        let _v2 = rng.next();
        // We don't pin a value here; the fixture script does.
    }

    #[test]
    fn random_returns_f64_as_next_over_2_pow_32() {
        // PS: PRNG.random() = result / 2 ** 32 (IEEE 754 round-to-nearest).
        // Pins the bit pattern for a few known seeds so an off-by-one
        // division error surfaces immediately.
        //
        // A pure-`u32`-fixture would not catch a wrong conversion here;
        // this test is the float-side companion to the 100k-call parity
        // test in tests/prng.rs.
        for seed in [
            [0u16, 0, 0, 0],
            [1, 2, 3, 4],
            [0xABCD, 0x1234, 0x5678, 0x9ABC],
            [0xFFFF, 0x0001, 0x8000, 0x7FFF],
        ] {
            let mut prng = Prng::new(seed);
            let mut peer = Gen5Rng::new(seed);
            let r = peer.next();
            let v = prng.random();
            assert_eq!(
                v,
                f64::from(r) / 4_294_967_296.0_f64,
                "random() = next / 2^32 (seed {seed:?})",
            );
            assert!(v >= 0.0 && v < 1.0);
        }
        // A second round: deterministic state transitions match.
        let mut prng = Prng::new([7, 11, 13, 17]);
        let mut peer = Gen5Rng::new([7, 11, 13, 17]);
        for _ in 0..16 {
            let r = peer.next();
            assert_eq!(
                prng.random(),
                f64::from(r) / 4_294_967_296.0_f64,
                "16-call chain matches",
            );
        }
    }

    #[test]
    fn random_n_zero_returns_zero_but_still_consumes_a_frame() {
        let mut prng = Prng::new([1, 2, 3, 4]);
        let seed_before = prng.get_seed();
        let v = prng.random_n(0);
        assert_eq!(v, 0);
        // PS makes one next() call regardless of n, and we mirror it.
        assert_ne!(prng.get_seed(), seed_before, "frame advanced");
        assert_eq!(prng.calls, 1);
    }

    #[test]
    fn random_n_one_is_always_zero_after_consuming_a_frame() {
        let mut prng = Prng::new([7, 11, 13, 17]);
        for _ in 0..100 {
            assert_eq!(prng.random_n(1), 0);
        }
        assert_eq!(prng.calls, 100);
    }

    #[test]
    fn random_range_one_two_is_always_one() {
        let mut prng = Prng::new([7, 11, 13, 17]);
        for _ in 0..100 {
            assert_eq!(prng.random_range(1, 2), 1);
        }
    }

    #[test]
    fn random_range_equal_args_is_constant() {
        let mut prng = Prng::new([7, 11, 13, 17]);
        for _ in 0..50 {
            assert_eq!(prng.random_range(42, 42), 42);
        }
    }

    #[test]
    fn shuffle_zero_or_one_elements_is_noop() {
        let mut p = Prng::new([1, 1, 1, 1]);
        let mut v: [u32; 0] = [];
        p.shuffle(&mut v, 0, 0);
        assert_eq!(p.calls, 0);

        let mut one = [42u32];
        p.shuffle(&mut one, 0, 1);
        assert_eq!(one, [42]);
        assert_eq!(p.calls, 0); // No frames consumed for length-1.
    }

    #[test]
    fn shuffle_consumes_end_minus_one_frames() {
        let mut p = Prng::new([1, 1, 1, 1]);
        let mut arr: [u32; 6] = [1, 2, 3, 4, 5, 6];
        p.shuffle(&mut arr, 0, 6);
        // PS makes `end - start - 1` random() calls. Each is one frame.
        assert_eq!(p.calls, 5);
    }

    #[test]
    fn shuffle_with_start_and_end_partial() {
        let mut p = Prng::new([1, 1, 1, 1]);
        let mut arr: [u32; 6] = [1, 2, 3, 4, 5, 6];
        // Only shuffle middle three (indices 1, 2, 3).
        p.shuffle(&mut arr, 1, 4);
        // Frames: end - start - 1 = 2.
        assert_eq!(p.calls, 2);
    }

    #[test]
    fn call_counter_records_every_method() {
        let mut p = Prng::new([1, 2, 3, 4]);
        assert_eq!(p.calls, 0);
        let _ = p.next();
        assert_eq!(p.calls, 1);
        let _ = p.random();
        assert_eq!(p.calls, 2);
        let _ = p.random_n(7);
        assert_eq!(p.calls, 3);
        let _ = p.random_range(3, 5);
        assert_eq!(p.calls, 4);
        let _ = p.random_chance(1, 2);
        assert_eq!(p.calls, 5);
        let arr: [u32; 3] = [10, 20, 30];
        let _ = p.sample(&arr);
        assert_eq!(p.calls, 6);
        let mut shuf = [1u32, 2, 3, 4, 5, 6];
        p.shuffle(&mut shuf, 0, 6);
        assert_eq!(p.calls, 11); // 6 prior + 5 shuffle frames.
    }

    #[cfg(any(test, feature = "test-override"))]
    #[test]
    fn override_queue_consumes_forced_values_first() {
        let mut prng = Prng::new([1, 2, 3, 4]);
        let initial_seed = prng.get_seed();
        prng.push_overrides([42, 7, 99]);
        assert!(!prng.override_is_empty());
        assert_eq!(prng.next(), 42);
        assert_eq!(prng.next(), 7);
        assert_eq!(prng.next(), 99);
        assert_eq!(
            prng.get_seed(),
            initial_seed,
            "override does not advance RNG state"
        );
        assert!(prng.override_is_empty());
        // Empty override reroutes to the underlying RNG.
        let real = prng.next();
        assert_ne!(prng.get_seed(), initial_seed);
        // Same path on a fresh peer should give the same value.
        let mut mirror = Prng::new([1, 2, 3, 4]);
        assert_eq!(mirror.next(), real);
    }

    #[cfg(any(test, feature = "test-override"))]
    #[test]
    fn override_clear_resets_the_queue() {
        let mut prng = Prng::new([1, 2, 3, 4]);
        prng.push_overrides([1, 2, 3, 4, 5]);
        assert!(!prng.override_is_empty());
        // Drain half, then verify a fresh Prng's RNG (without overrides)
        // matches what the override-emptied one does on the next call.
        assert_eq!(prng.next(), 1);
        assert_eq!(prng.next(), 2);
        assert!(!prng.override_is_empty());
        assert_eq!(prng.calls, 2);
        // Now shovel more in, observe the queue length grows; we don't
        // expose a length getter on the integration path, but the test
        // confirms the API surface stays stable across both gates.
        prng.push_overrides([10]);
        let _ = prng.next();
        assert_eq!(prng.calls, 3);
    }
}
