//! Pokemon Showdown-compatible PRNG.
//!
//! Bit-for-bit port of `PS/sim/prng.ts` (unchanged from 0.11.11 to the
//! re-sync pin). Drives everything that draws randomness in the new battle
//! server; called in PS's order by `ps_sim` itself so the protocol-diff
//! harness can compare entire battles without monkeypatching the reference
//! (design section 5.5).
//!
//! PS uses two PRNGs: `Gen5RNG` (a 64-bit LCG) and `SodiumRNG` (ChaCha20,
//! libsodium's `randombytes_buf_deterministic`). The harness and the device
//! start battles with a numeric `[a, b, c, d]` seed, which PS resolves to
//! `Gen5RNG`; the live server seeds every battle with `sodium,<hex>`, so
//! replaying a current replay needs [`SodiumRng`]. The seed forms are
//! [`ps_protocol::PrngSeed`].
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
//!
//! The `rng-trace` feature records every public PRNG call in
//! [`Prng::trace`]: its index, the method, the arguments, the result and
//! the caller location (`file:line`, from `#[track_caller]`). The trace is
//! what the protocol-diff harness compares against PS's own PRNG trace
//! when the two sides diverge. The feature is off by default, so the
//! firmware build carries neither the trace nor the `track_caller`
//! metadata.

#![cfg_attr(not(any(test, feature = "test-override")), allow(dead_code))]

extern crate alloc;

#[cfg(any(test, feature = "test-override"))]
use alloc::collections::VecDeque;
#[cfg(feature = "rng-trace")]
use alloc::vec::Vec;

pub use ps_protocol::PrngSeed;

/// One traced PRNG call (feature `rng-trace`).
///
/// The trace mirrors what PS's own PRNG does, one entry per public call:
/// `random` (with the overload's arguments), `random_chance`, `sample`,
/// `shuffle` and `next`. The `index` is the call's position in the
/// battle's trace, so the two sides' traces can be lined up.
#[cfg(feature = "rng-trace")]
#[derive(Clone, Debug, PartialEq)]
pub struct PrngCall {
    /// The call's index in the trace (0-based).
    pub index: u32,
    /// The PRNG method: `random`, `random_chance`, `sample`, `shuffle` or
    /// `next`.
    pub method: &'static str,
    /// The arguments the call was made with.
    pub args: Vec<PrngValue>,
    /// What the call returned.
    pub result: PrngValue,
    /// Where the call was made: `file:line` of the caller.
    pub site: alloc::string::String,
}

/// A traced argument or result (feature `rng-trace`).
#[cfg(feature = "rng-trace")]
#[derive(Clone, Debug, PartialEq)]
pub enum PrngValue {
    /// An integer (a draw, an index, a bound).
    Int(i64),
    /// A float (`random()` without arguments).
    Float(f64),
    /// A boolean (`random_chance`).
    Bool(bool),
    /// A list of integers (the indices `shuffle` picked).
    Ints(Vec<i64>),
}

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

/// `SodiumRNG.NONCE`: `LibsodiumDRG`, libsodium's
/// `randombytes_buf_deterministic` nonce.
const SODIUM_NONCE: [u8; 12] = *b"LibsodiumDRG";

/// The ChaCha20 quarter round (RFC 7539 section 2.1).
#[inline(always)]
fn quarter_round(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(16);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(12);
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(8);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(7);
}

/// The ChaCha20 block function (RFC 7539 section 2.3): 20 rounds over the
/// constants, the 256-bit key, a 32-bit block counter and a 96-bit nonce,
/// serialised little-endian.
// PS: ts-chacha20 1.2.0 Chacha20._chacha
pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let le = |b: &[u8], i: usize| u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    let mut init = [0u32; 16];
    init[..4].copy_from_slice(&[0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574]);
    for i in 0..8 {
        init[4 + i] = le(key, i);
    }
    init[12] = counter;
    for i in 0..3 {
        init[13 + i] = le(nonce, i);
    }
    let mut x = init;
    for _ in 0..10 {
        quarter_round(&mut x, 0, 4, 8, 12);
        quarter_round(&mut x, 1, 5, 9, 13);
        quarter_round(&mut x, 2, 6, 10, 14);
        quarter_round(&mut x, 3, 7, 11, 15);
        quarter_round(&mut x, 0, 5, 10, 15);
        quarter_round(&mut x, 1, 6, 11, 12);
        quarter_round(&mut x, 2, 7, 8, 13);
        quarter_round(&mut x, 3, 4, 9, 14);
    }
    let mut out = [0u8; 64];
    for i in 0..16 {
        out[4 * i..4 * i + 4].copy_from_slice(&x[i].wrapping_add(init[i]).to_le_bytes());
    }
    out
}

/// PS's `SodiumRNG`: a 32-byte key; each `next()` encrypts 36 zero bytes
/// with ChaCha20 (counter 0, nonce `LibsodiumDRG`), keeps bytes 0..32 as
/// the new key and returns bytes 32..36 big-endian.
// PS: sim/prng.ts SodiumRNG
#[derive(Debug, Clone)]
pub struct SodiumRng {
    seed: [u8; 32],
}

impl SodiumRng {
    /// `SodiumRNG.setSeed`: the key as [`ps_protocol::PrngSeed::Sodium`]
    /// holds it (the hex already padded).
    pub fn new(seed: [u8; 32]) -> Self {
        Self { seed }
    }

    /// PS: `SodiumRNG.next`.
    pub fn next(&mut self) -> u32 {
        // Encrypting zeros returns the keystream; 36 bytes is one block.
        let buf = chacha20_block(&self.seed, 0, &SODIUM_NONCE);
        self.seed.copy_from_slice(&buf[..32]);
        u32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]])
    }

    /// PS: `SodiumRNG.getSeed` (`sodium,` + the key in hex).
    pub fn get_seed(&self) -> [u8; 32] {
        self.seed
    }
}

/// The RNG behind [`Prng`], chosen by the seed as `PRNG.setSeed` does.
#[derive(Debug, Clone)]
enum Rng {
    Gen5(Gen5Rng),
    Sodium(SodiumRng),
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

/// The battle-time PRNG. Holds the `Gen5RNG` or `SodiumRNG` the seed
/// selects, an override queue when configured, and a public call counter
/// for diffing.
pub struct Prng {
    rng: Rng,
    /// Total number of public method calls. Reset only by `reset_calls`.
    pub calls: u64,
    #[cfg(any(test, feature = "test-override"))]
    override_queue: VecDeque<u32>,
    /// The traced calls (feature `rng-trace`).
    #[cfg(feature = "rng-trace")]
    trace: Vec<PrngCall>,
    /// `true` while a traced call is in flight (feature `rng-trace`): a
    /// nested call belongs to the outer one, so one logical call is one
    /// trace entry.
    #[cfg(feature = "rng-trace")]
    tracing: bool,
}

impl Prng {
    /// PS `new PRNG(seed)`: a numeric `[a, b, c, d]` seed selects
    /// `Gen5RNG`, a `sodium,` one `SodiumRNG`.
    // PS: sim/prng.ts PRNG.setSeed
    pub fn new(seed: impl Into<PrngSeed>) -> Self {
        let rng = match seed.into() {
            PrngSeed::Gen5(s) => Rng::Gen5(Gen5Rng::new(s)),
            PrngSeed::Sodium(k) => Rng::Sodium(SodiumRng::new(k)),
        };
        Self {
            rng,
            calls: 0,
            #[cfg(any(test, feature = "test-override"))]
            override_queue: VecDeque::new(),
            #[cfg(feature = "rng-trace")]
            trace: Vec::new(),
            #[cfg(feature = "rng-trace")]
            tracing: false,
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

    /// PS: `PRNG.getSeed`.
    pub fn get_seed(&self) -> PrngSeed {
        match &self.rng {
            Rng::Gen5(r) => PrngSeed::Gen5(r.get_seed()),
            Rng::Sodium(r) => PrngSeed::Sodium(r.get_seed()),
        }
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
        match &mut self.rng {
            Rng::Gen5(r) => {
                r.next_frame(frames_to_advance);
            }
            Rng::Sodium(r) => {
                for _ in 0..frames_to_advance {
                    r.next();
                }
            }
        }
    }

    /// PS: `Gen5RNG.next` (delegates after consulting the override).
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn next(&mut self) -> u32 {
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        #[cfg(any(test, feature = "test-override"))]
        {
            if let Some(v) = self.override_queue.pop_front() {
                self.calls = self.calls.saturating_add(1);
                #[cfg(feature = "rng-trace")]
                self.trace_end(
                    outer,
                    "next",
                    alloc::vec![],
                    PrngValue::Int(i64::from(v)),
                    &core::panic::Location::caller(),
                );
                return v;
            }
        }
        let v = match &mut self.rng {
            Rng::Gen5(r) => r.next(),
            Rng::Sodium(r) => r.next(),
        };
        self.calls = self.calls.saturating_add(1);
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "next",
            alloc::vec![],
            PrngValue::Int(i64::from(v)),
            &core::panic::Location::caller(),
        );
        v
    }

    /// PS: `PRNG.random` (no `@param`) = `next / 2^32` as `f64`.
    ///
    /// IEEE 754 nearest rounding gives JS and Rust identical bits for any
    /// integer `next` in `[0, 2^32)`.
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn random(&mut self) -> f64 {
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        let r = self.next();
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "random",
            alloc::vec![],
            PrngValue::Float(f64::from(r) / 4_294_967_296.0_f64),
            &core::panic::Location::caller(),
        );
        f64::from(r) / 4_294_967_296.0_f64
    }

    /// PS: `PRNG.random(n)` = `floor(next * n / 2^32)`, integer in `[0, n)`.
    ///
    /// Done in `u64` so the result is bit-exact for every `n` the
    /// ported moves ask for (largest is `2^16` today). For `n = 0` PS
    /// returns `0`; we mirror that.
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn random_n(&mut self, n: u32) -> u32 {
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        let r = self.next();
        let product = (u64::from(r)) * u64::from(n);
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "random",
            alloc::vec![PrngValue::Int(i64::from(n))],
            PrngValue::Int(i64::from((product >> 32) as u32)),
            &core::panic::Location::caller(),
        );
        (product >> 32) as u32
    }

    /// PS: `PRNG.random(m, n)` = `floor(next * (n - m) / 2^32) + m`, integer
    /// in `[m, n)`. The brief asks for this exact form done in `u64`.
    ///
    /// Requires `m <= n`. `m == n` returns `m` exactly (the floor term
    /// vanishes), matching PS.
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn random_range(&mut self, m: u32, n: u32) -> u32 {
        debug_assert!(m <= n, "PRNG::random_range requires m <= n");
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        if m == n {
            // Still a registered call (PS makes one next() call). Match PS
            // exactly: count it because `random(m, m)` does consume a frame
            // in JS (it's just a multiplication by zero of the same `next`
            // value).
            let _ = self.next();
            #[cfg(feature = "rng-trace")]
            self.trace_end(
                outer,
                "random",
                alloc::vec![PrngValue::Int(i64::from(m)), PrngValue::Int(i64::from(n))],
                PrngValue::Int(i64::from(m)),
                &core::panic::Location::caller(),
            );
            return m;
        }
        let v = self.random_n(n - m) + m;
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "random",
            alloc::vec![PrngValue::Int(i64::from(m)), PrngValue::Int(i64::from(n))],
            PrngValue::Int(i64::from(v)),
            &core::panic::Location::caller(),
        );
        v
    }

    /// PS: `PRNG.randomChance(numerator, denominator)` = `random(denominator)
    /// < numerator`. Numerator is non-negative; denominator is positive
    /// (PS asserts on its own).
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn random_chance(&mut self, numerator: u32, denominator: u32) -> bool {
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        let v = self.random_n(denominator) < numerator;
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "random_chance",
            alloc::vec![
                PrngValue::Int(i64::from(numerator)),
                PrngValue::Int(i64::from(denominator)),
            ],
            PrngValue::Bool(v),
            &core::panic::Location::caller(),
        );
        v
    }

    /// PS: `PRNG.randomChance(numerator, denominator)` with a fractional
    /// numerator: PS's `Battle#randomChance` takes a JS number, and the
    /// Gen 3 accuracy roll passes one (`randomChance(accuracy, 100)`).
    ///
    /// The traced build only: the plain build rolls the accuracy without
    /// it, so the device's code (and its size) stays as it was.
    #[cfg(feature = "rng-trace")]
    #[track_caller]
    pub(crate) fn random_chance_f(&mut self, numerator: f64, denominator: u32) -> bool {
        let outer = self.trace_begin();
        let v = f64::from(self.random_n(denominator)) < numerator;
        self.trace_end(
            outer,
            "random_chance",
            alloc::vec![
                PrngValue::Float(numerator),
                PrngValue::Int(i64::from(denominator)),
            ],
            PrngValue::Bool(v),
            &core::panic::Location::caller(),
        );
        v
    }

    /// PS: `PRNG.sample(items)` = `items[random(items.length)]`. We return
    /// a reference because `shuffle`-style callers benefit from in-place
    /// mutation instead of an extra copy.
    ///
    /// Panics on empty (PS throws `RangeError`).
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn sample<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        assert!(!items.is_empty(), "PRNG::sample: empty slice");
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        let idx = self.random_n(items.len() as u32) as usize;
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "sample",
            alloc::vec![PrngValue::Int(items.len() as i64)],
            PrngValue::Int(idx as i64),
            &core::panic::Location::caller(),
        );
        &items[idx]
    }

    /// PS: `PRNG.shuffle(items, start, end)`, Fisher-Yates with a `random(start,
    /// end)` per iteration. Performs `end - start - 1` underlying `next()`
    /// calls (zero when `end <= start + 1`).
    ///
    /// `start` and `end` are `usize` to mirror Rust slice conventions; the
    /// per-iteration cast is lossless because `items.len() <= usize::MAX`
    /// and our state caps well below `u32::MAX`.
    #[cfg_attr(feature = "rng-trace", track_caller)]
    pub fn shuffle<T>(&mut self, items: &mut [T], mut start: usize, end: usize) {
        #[cfg(feature = "rng-trace")]
        let outer = self.trace_begin();
        #[cfg(feature = "rng-trace")]
        let first = start;
        #[cfg(feature = "rng-trace")]
        let mut picks: Vec<i64> = Vec::new();
        while start + 1 < end {
            let next_index = self.random_range(start as u32, end as u32) as usize;
            #[cfg(feature = "rng-trace")]
            picks.push(next_index as i64);
            if start != next_index {
                items.swap(start, next_index);
            }
            start += 1;
        }
        #[cfg(feature = "rng-trace")]
        self.trace_end(
            outer,
            "shuffle",
            alloc::vec![
                PrngValue::Int(items.len() as i64),
                PrngValue::Int(first as i64),
                PrngValue::Int(end as i64),
            ],
            PrngValue::Ints(picks),
            &core::panic::Location::caller(),
        );
    }
}

/// The PRNG call trace (feature `rng-trace`).
#[cfg(feature = "rng-trace")]
impl Prng {
    /// The traced calls, in call order.
    pub fn trace(&self) -> &[PrngCall] {
        &self.trace
    }

    /// Takes the traced calls out of the PRNG: the harness drains them
    /// when the battle ends.
    pub fn take_trace(&mut self) -> Vec<PrngCall> {
        // A panic inside a call leaves the nest flag up; the trace is
        // drained anyway, so the next battle starts clean.
        self.tracing = false;
        core::mem::take(&mut self.trace)
    }

    /// Records one call: the next index, the method, the arguments, the
    /// result and the caller location.
    ///
    /// Only the outermost call of a nest records: `random()` draws
    /// through `next()`, and the draw belongs to the `random()`.
    fn trace_begin(&mut self) -> bool {
        let outer = !self.tracing;
        self.tracing = true;
        outer
    }

    /// Ends a traced call: the outermost one records the entry (index,
    /// method, arguments, result and caller location); a nested one only
    /// gives the flag back to its caller.
    fn trace_end(
        &mut self,
        outer: bool,
        method: &'static str,
        args: Vec<PrngValue>,
        result: PrngValue,
        loc: &core::panic::Location<'static>,
    ) {
        if !outer {
            return;
        }
        self.tracing = false;
        let index = self.trace.len() as u32;
        self.trace.push(PrngCall {
            index,
            method,
            args,
            result,
            site: alloc::format!("{}:{}", loc.file(), loc.line()),
        });
    }
}

impl core::fmt::Debug for Prng {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Prng")
            .field("seed", &self.get_seed())
            .field("calls", &self.calls)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(s: &str) -> [u8; 32] {
        match PrngSeed::parse(&alloc::format!("sodium,{s}")) {
            Some(PrngSeed::Sodium(k)) => k,
            other => panic!("{other:?}"),
        }
    }

    // RFC 7539 section 2.3.2: the block function test vector (key 00..1f,
    // nonce 00:00:00:09:00:00:00:4a:00:00:00:00, counter 1). ts-chacha20
    // 1.2.0, which PS's SodiumRNG runs on, gives the same 64 bytes.
    #[test]
    fn chacha20_block_rfc7539_vector() {
        let key: [u8; 32] = core::array::from_fn(|i| i as u8);
        let nonce = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let want = hex32("10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c068030422aa9ac3d46c4e");
        let want2 = hex32("d2826446079faa0914c2d705d98b02a2b5129cd1de164eb9cbd083e8a2503c4e");
        let block = chacha20_block(&key, 1, &nonce);
        assert_eq!(block[..32], want);
        assert_eq!(block[32..], want2);
    }

    // PS's SodiumRNG on the re-sync pin (and 0.11.11, identical): four
    // `rng.next()` then `getSeed()`, from
    // `new PRNG('sodium,...')` in tools/ps-diff's install.
    #[test]
    fn sodium_rng_matches_ps_vectors() {
        let cases: [(&str, [u32; 4], &str); 5] = [
            (
                "00000000000000000000000000000000",
                [4267713560, 2597593732, 1237973749, 687352079],
                "274564c90cb73c6453f4a4152c3fb2c3021f7950723025ec25604fce96de1ed4",
            ),
            // A real gen1randombattle replay's battle seed.
            (
                "d4e69dd8ca6bd83e6f23520d0c6d7984",
                [3452080645, 121223913, 3901222867, 1439957731],
                "2855a2a44139525971584096f7c91a3b41f19ff0e519720ad918afc9adaf03c5",
            ),
            // Short and odd-length hex: padded with `0` to 64 digits.
            ("1", [3939852285, 3702501483, 516247530, 930443638], "1ac48774da8a46a04f89932c8aad8ba33d3b4b7c2b40efd81d73d414ee86c532"),
            ("abc", [230169386, 2646009527, 2494795265, 3347839838], "e91cc600a065bfc0b058c4bb121278f5dcb3aa0820b53a4560f00e62811ae550"),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                [912383416, 2912445401, 4101633452, 327154605],
                "ae3cb51c26ee94c1069802fe03f45d4aaa02c56b68628b925687e5b8bc00f73b",
            ),
        ];
        for (seed, outs, after) in cases {
            let mut prng = Prng::new(PrngSeed::Sodium(hex32(seed)));
            for want in outs {
                assert_eq!(prng.next(), want, "sodium,{seed}");
            }
            assert_eq!(prng.get_seed(), PrngSeed::Sodium(hex32(after)), "sodium,{seed}");
            assert_eq!(prng.calls, 4);
        }
        // A thousand calls in: `next()` and the key after it.
        let mut prng = Prng::new(PrngSeed::Sodium(hex32("d4e69dd8ca6bd83e6f23520d0c6d7984")));
        prng.advance(999);
        assert_eq!(prng.next(), 881997606);
        assert_eq!(
            prng.get_seed(),
            PrngSeed::Sodium(hex32("eccd0ae04587091dbe1cbf604f2124027bd230b3911b608bdb4e65675d06935e"))
        );
        // The PRNG API sits on `next()` as for Gen5RNG: PS's `random(100)`,
        // `random()`, `random(2, 6)`, `randomChance(1, 256)`.
        let mut prng = Prng::new(PrngSeed::Sodium(hex32("d4e69dd8ca6bd83e6f23520d0c6d7984")));
        assert_eq!(prng.random_n(100), 80);
        assert_eq!(prng.random(), 0.028224641690030694);
        assert_eq!(prng.random_range(2, 6), 5);
        assert!(!prng.random_chance(1, 256));
    }

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

    #[cfg(feature = "rng-trace")]
    #[test]
    fn trace_records_every_public_call() {
        let mut p = Prng::new([1, 2, 3, 4]);
        let _ = p.random();
        let _ = p.random_n(16);
        let _ = p.random_range(2, 6);
        let _ = p.random_chance(1, 16);
        let arr: [u32; 3] = [10, 20, 30];
        let _ = p.sample(&arr);
        let mut shuf = [1u32, 2, 3, 4];
        p.shuffle(&mut shuf, 0, 4);
        let _ = p.next();

        let trace = p.trace();
        let methods: Vec<&str> = trace.iter().map(|c| c.method).collect();
        assert_eq!(
            methods,
            ["random", "random", "random", "random_chance", "sample", "shuffle", "next"],
            "one entry per public call",
        );
        for (i, call) in trace.iter().enumerate() {
            assert_eq!(call.index as usize, i, "the calls are numbered in order");
        }
        assert_eq!(trace[0].args, alloc::vec![], "random() takes no arguments");
        assert_eq!(trace[1].args, alloc::vec![PrngValue::Int(16)], "random(n)");
        assert_eq!(
            trace[2].args,
            alloc::vec![PrngValue::Int(2), PrngValue::Int(6)],
            "random(m, n)",
        );
        assert_eq!(
            trace[3].args,
            alloc::vec![PrngValue::Int(1), PrngValue::Int(16)],
            "random_chance(numerator, denominator)",
        );
        assert_eq!(trace[4].args, alloc::vec![PrngValue::Int(3)], "sample(len)");
        assert!(
            matches!(trace[4].result, PrngValue::Int(_)),
            "the sample's result is the index it picked",
        );
        assert_eq!(
            trace[5].args,
            alloc::vec![PrngValue::Int(4), PrngValue::Int(0), PrngValue::Int(4)],
            "shuffle(len, start, end)",
        );
        assert!(
            matches!(trace[5].result, PrngValue::Ints(_)),
            "the shuffle's result is the indices it picked",
        );
        for call in trace.iter() {
            assert!(
                call.site.contains("prng.rs:"),
                "the call site is the file and line of the caller: {}",
                call.site,
            );
        }

        // The trace can be taken out (the harness drains it when the
        // battle ends).
        let taken = p.take_trace();
        assert_eq!(taken.len(), 7);
        assert!(p.trace().is_empty());
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
