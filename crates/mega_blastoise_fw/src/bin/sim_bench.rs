//! The battle server's budget on the board (task P2-4, design section 6,
//! `docs/living/showdown-split-budget.md`).
//!
//! Plays the 200 seeded random battles per ported gen of
//! `crates/ps_sim/tests/common/bench_driver.rs` (the same file the host
//! probe `crates/ps_sim/tests/budget.rs` plays) straight on `ps_sim`, and
//! reports over RTT:
//! - the DWT cycle count per turn: p50, p99 and max, and the same in
//!   microseconds at 150 MHz (`embassy_rp::init`'s default clock);
//! - the heap's `used()` before and after every command, which must be
//!   equal (the number of commands where they differ must be 0);
//! - the stack high-water mark: 32 KB below the stack pointer are painted
//!   before each battle and scanned after it.
//!
//! Run: `cargo run --release --bin sim_bench` here (`probe-rs run --preset
//! pico2`). Until a board exists the numbers come from the host proxies
//! instead (`scripts/sim-budget.sh`), which also builds this bench for its
//! flash numbers; feature `sim-bench-baseline` compiles the battles out for
//! the baseline.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use cortex_m::peripheral::DWT;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Timer;
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::{init_heap, HEAP};
use rtt_target::{rtt_init, set_defmt_channel};

#[cfg(not(feature = "sim-bench-baseline"))]
#[path = "../../../ps_sim/tests/common/bench_driver.rs"]
mod driver;

/// The bench's gens and battles, as the driver has them (the baseline
/// keeps the loop shape without the driver).
#[cfg(not(feature = "sim-bench-baseline"))]
use driver::{BATTLES_PER_GEN, GENS};
#[cfg(feature = "sim-bench-baseline")]
const BATTLES_PER_GEN: u32 = 200;
#[cfg(feature = "sim-bench-baseline")]
const GENS: [u8; 2] = [1, 3];

/// Core clock, for the microsecond figures.
const CLOCK_MHZ: u32 = 150;
/// Bytes painted below the stack pointer before each battle.
const PAINT_BYTES: usize = 32 * 1024;
const PAINT: u32 = 0xA5A5_5A5A;
/// Turn samples kept per gen (the host sweep plays about 13,000 Gen 1 turns).
const SAMPLE_CAP: usize = 16 * 1024;

/// Design section 6: at most 16 KB per battle. The host test checks the
/// host sizes; this checks the device's at compile time.
#[cfg(not(feature = "sim-bench-baseline"))]
const _: () = assert!(core::mem::size_of::<ps_sim::Battle>() <= 16 * 1024);

/// `size_of` on this target of `Battle`, `Side`, `Pokemon`, `EffectState`,
/// for `scripts/sim-budget.sh` to read out of the ELF.
#[cfg(not(feature = "sim-bench-baseline"))]
#[no_mangle]
#[used]
pub static SIM_BENCH_SIZES: [u32; 4] = [
    core::mem::size_of::<ps_sim::Battle>() as u32,
    core::mem::size_of::<ps_sim::Side>() as u32,
    core::mem::size_of::<ps_sim::Pokemon>() as u32,
    core::mem::size_of::<ps_sim::EffectState>() as u32,
];

/// Cycles per turn and the heap check, per command.
#[cfg_attr(feature = "sim-bench-baseline", allow(dead_code))]
struct BoardProbe {
    start: u32,
    heap_before: usize,
    /// Cycles of the commands of the current turn so far.
    acc: u32,
    samples: Vec<u32>,
    /// Turns past [`SAMPLE_CAP`] (timed, not kept).
    dropped: u32,
    /// Commands whose `used()` after differs from before.
    heap_moved: u32,
    commands: u32,
}

impl BoardProbe {
    fn new() -> BoardProbe {
        BoardProbe {
            start: 0,
            heap_before: 0,
            acc: 0,
            samples: Vec::with_capacity(SAMPLE_CAP),
            dropped: 0,
            heap_moved: 0,
            commands: 0,
        }
    }
}

#[cfg(not(feature = "sim-bench-baseline"))]
impl driver::Probe for BoardProbe {
    fn enter(&mut self) {
        self.heap_before = HEAP.used();
        self.start = DWT::cycle_count();
    }

    fn exit(&mut self, exit: driver::Exit) {
        let cycles = DWT::cycle_count().wrapping_sub(self.start);
        if HEAP.used() != self.heap_before {
            self.heap_moved += 1;
        }
        self.commands += 1;
        match exit {
            driver::Exit::Setup => {}
            driver::Exit::Pending => self.acc = self.acc.wrapping_add(cycles),
            driver::Exit::Turn => {
                let turn = self.acc.wrapping_add(cycles);
                self.acc = 0;
                if self.samples.len() < SAMPLE_CAP {
                    self.samples.push(turn);
                } else {
                    self.dropped += 1;
                }
            }
        }
    }
}

/// The stack pointer.
#[inline(always)]
fn sp() -> usize {
    cortex_m::register::msp::read() as usize
}

/// Paints [`PAINT_BYTES`] below the stack pointer (less a margin for this
/// frame); returns the window's base.
#[inline(never)]
fn paint() -> usize {
    let top = (sp() - 256) & !3;
    let base = top - PAINT_BYTES;
    for a in (base..top).step_by(4) {
        // SAFETY: free stack below the live frames; the bench leaves well
        // over 32 KB of RAM to the stack (the heap and RTT are the .bss).
        unsafe { core::ptr::write_volatile(a as *mut u32, PAINT) };
    }
    base
}

/// The deepest painted byte overwritten, as bytes below the window's top.
#[inline(never)]
fn high_water(base: usize) -> usize {
    let top = base + PAINT_BYTES;
    for a in (base..top).step_by(4) {
        // SAFETY: the window `paint` wrote.
        if unsafe { core::ptr::read_volatile(a as *const u32) } != PAINT {
            return top - a;
        }
    }
    0
}

/// The `q`-quantile (nearest rank, in per mille) of sorted `v`.
fn quantile(v: &[u32], per_mille: usize) -> u32 {
    if v.is_empty() {
        return 0;
    }
    let rank = (v.len() * per_mille).div_ceil(1000).clamp(1, v.len());
    v[rank - 1]
}

/// Plays the gen's battles under `probe`; returns the stack high-water
/// mark and how many battles ended.
#[cfg(not(feature = "sim-bench-baseline"))]
fn run_gen(gen: u8, probe: &mut BoardProbe) -> (usize, u32) {
    let mut stack = 0;
    let mut ended = 0;
    for i in 0..BATTLES_PER_GEN {
        let mut b = Box::new(driver::new_battle(gen, i));
        let base = paint();
        let o = driver::play(&mut b, gen, i, probe);
        stack = stack.max(high_water(base));
        ended += o.ended as u32;
    }
    (stack, ended)
}

/// The baseline: the same reporting over no battles.
#[cfg(feature = "sim-bench-baseline")]
fn run_gen(gen: u8, probe: &mut BoardProbe) -> (usize, u32) {
    let base = paint();
    for i in 0..BATTLES_PER_GEN {
        let heap_before = HEAP.used();
        let start = DWT::cycle_count();
        let _ = core::hint::black_box((gen, i, Box::new(i)));
        probe.samples.push(DWT::cycle_count().wrapping_sub(start));
        probe.heap_moved += (HEAP.used() != heap_before) as u32;
        probe.commands += 1;
    }
    (high_water(base), BATTLES_PER_GEN)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    let mut cp = cortex_m::Peripherals::take().unwrap();
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    #[cfg(not(feature = "sim-bench-baseline"))]
    {
        let s = core::hint::black_box(&SIM_BENCH_SIZES);
        info!("sim_bench: size_of Battle {} Side {} Pokemon {} EffectState {}", s[0], s[1], s[2], s[3]);
    }
    #[cfg(feature = "sim-bench-baseline")]
    info!("sim_bench: baseline build (battles compiled out)");

    for gen in GENS {
        let mut probe = BoardProbe::new();
        let (stack, ended) = run_gen(gen, &mut probe);
        let mut samples = core::mem::take(&mut probe.samples);
        samples.sort_unstable();
        let (p50, p99, max) = (quantile(&samples, 500), quantile(&samples, 990), samples.last().copied().unwrap_or(0));
        info!(
            "sim_bench gen {}: {} battles ({} ended), {} turns ({} not kept), {} commands",
            gen,
            BATTLES_PER_GEN,
            ended,
            samples.len() as u32 + probe.dropped,
            probe.dropped,
            probe.commands
        );
        info!(
            "sim_bench gen {}: cycles per turn p50 {} p99 {} max {}; us at {} MHz p50 {} p99 {} max {}",
            gen,
            p50,
            p99,
            max,
            CLOCK_MHZ,
            p50 / CLOCK_MHZ,
            p99 / CLOCK_MHZ,
            max / CLOCK_MHZ
        );
        info!(
            "sim_bench gen {}: heap used() moved in {} commands (must be 0); stack high-water {} B",
            gen, probe.heap_moved, stack
        );
    }
    info!("sim_bench: done");
    loop {
        Timer::after_secs(60).await;
    }
}
