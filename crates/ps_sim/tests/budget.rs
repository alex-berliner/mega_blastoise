//! The memory and speed budget, on the host (task P2-4; design section 6,
//! `docs/architecture/showdown-split-budget.md`).
//!
//! Every number here is a host proxy, and the gates are soft until a board
//! exists: the RP2350 measurement (`sim_bench` over RTT) is deferred.
//!
//! - [`state_fits_the_budget`]: `size_of` of `Battle`, `Side`, `Pokemon`
//!   and `EffectState` against their ceilings. The host sizes bound the
//!   device's (host pointers are wider); `sim_bench` repeats the check at
//!   compile time for thumbv8m.
//! - [`no_allocation_inside_a_turn_gen1`] (and `_gen3`): a counting global allocator, armed
//!   around every command (`step` plus `send_updates`) of 500 seeded random
//!   battles per ported gen, counts zero allocations.
//! - [`host_turn_times`]: wall time per turn over the bench's 200 seeded
//!   battles per gen (p50, p99, max, in ns). Run it `--release`
//!   (`scripts/sim-budget.sh` does); a debug build plays 20 battles per
//!   gen as a smoke run.
//! - [`stack_high_water`]: the deepest stack one command reaches, by
//!   painting the stack below the driver and scanning for the lowest
//!   overwritten word after each command (the method the board bench uses).
//!
//! Gen 1 has every effect ported. Gen 3 has its move pipeline only: its
//! teams draw from the ported pools, and a battle that reaches an unported
//! handler stops there (`unported: ...`); the turns it played count.
//!
//! The lines starting `BUDGET ` are what `scripts/sim-budget.sh` reads.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

use ps_sim::{Battle, EffectState, Pokemon, Side};

#[path = "common/bench_driver.rs"]
mod driver;

use driver::{Exit, Probe, BATTLES_PER_GEN, GENS};

/// Battles per gen of the timing and stack probes: the bench's 200 in a
/// release build (what `scripts/sim-budget.sh` runs); a 20-battle smoke run
/// in a debug build (`check.sh`'s workspace tests), whose numbers mean
/// nothing.
const PROBE_BATTLES: u32 = if cfg!(debug_assertions) { 20 } else { BATTLES_PER_GEN };

// ---- the ceilings ----------------------------------------------------------

/// Design section 6: at most 16 KB of RAM per battle, fixed capacity.
const BATTLE_MAX: usize = 16 * 1024;
/// The shares of that 16 KB the state is held to: a side (six Pokemon and
/// the side's own state), one Pokemon (its 12 volatiles included) and one
/// effect state. Set at P2-4 from the measured sizes with a few percent of
/// headroom, so any growth that would eat the battle's remaining margin
/// fails here first; P1-6's report lists the reductions that would lower
/// them.
const SIDE_MAX: usize = 6 * 1024;
const POKEMON_MAX: usize = 896;
const EFFECT_STATE_MAX: usize = 40;

#[test]
fn state_fits_the_budget() {
    let sizes = [
        ("Battle", core::mem::size_of::<Battle>(), BATTLE_MAX),
        ("Side", core::mem::size_of::<Side>(), SIDE_MAX),
        ("Pokemon", core::mem::size_of::<Pokemon>(), POKEMON_MAX),
        ("EffectState", core::mem::size_of::<EffectState>(), EFFECT_STATE_MAX),
    ];
    for (name, size, max) in sizes {
        println!("BUDGET size {name} {size} {max}");
    }
    for (name, size, max) in sizes {
        assert!(size <= max, "size_of::<{name}>() = {size} B, over its {max} B ceiling");
    }
}

// ---- the counting allocator --------------------------------------------------

/// Counts the allocations of the current thread while armed.
struct Counting;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    // `try_with`: the allocator also runs while thread locals are torn down.
    let _ = ARMED.try_with(|a| {
        if a.get() {
            let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
        }
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// ---- playing -------------------------------------------------------------------

/// Plays battle `i` of `gen` under `probe`; `Err` with the panic text when
/// it stopped (Gen 3's unported handlers). The panic hook is silenced
/// around it by [`quiet`].
fn run<P: Probe>(gen: u8, i: u32, probe: &mut P) -> Result<driver::Outcome, String> {
    let mut b = Box::new(driver::new_battle(gen, i));
    panic::catch_unwind(AssertUnwindSafe(|| driver::play(&mut b, gen, i, probe))).map_err(|e| {
        e.downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    })
}

thread_local! {
    static QUIET: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` with the panic message of this thread's stops silenced (they
/// are counted, not printed). One hook for the whole binary, so tests
/// running in parallel do not swap hooks under each other.
fn quiet<T>(f: impl FnOnce() -> T) -> T {
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !QUIET.with(|q| q.get()) {
                default(info);
            }
        }));
    });
    QUIET.with(|q| q.set(true));
    let out = f();
    QUIET.with(|q| q.set(false));
    out
}

/// What a sweep saw, per gen.
#[derive(Default)]
struct Sweep {
    battles: u32,
    ended: u32,
    stopped: u32,
    /// The first stop's message.
    first_stop: String,
    turns: u32,
    commands: u32,
}

impl Sweep {
    fn add(&mut self, r: &Result<driver::Outcome, String>) {
        self.battles += 1;
        match r {
            Ok(o) => {
                self.ended += o.ended as u32;
                self.turns += o.turns;
                self.commands += o.commands;
            }
            Err(e) => {
                self.stopped += 1;
                if self.first_stop.is_empty() {
                    self.first_stop = e.clone();
                }
            }
        }
    }

    fn print(&self, what: &str, gen: u8) {
        println!(
            "{what} gen {gen}: {} battles, {} ended, {} stopped unported{}; {} turns, {} commands (stopped battles not counted)",
            self.battles,
            self.ended,
            self.stopped,
            if self.stopped > 0 { format!(" (first: {})", self.first_stop) } else { String::new() },
            self.turns,
            self.commands
        );
    }
}

// ---- zero allocations inside a turn -------------------------------------------------

/// Arms the counter around each command; the allocations of commands that
/// finished (a stop's panic payload allocates) add up in `allocs`.
#[derive(Default)]
struct AllocProbe {
    allocs: u64,
    commands: u64,
}

impl Probe for AllocProbe {
    fn enter(&mut self) {
        ALLOCS.with(|n| n.set(0));
        ARMED.with(|a| a.set(true));
    }
    fn exit(&mut self, _: Exit) {
        ARMED.with(|a| a.set(false));
        self.allocs += ALLOCS.with(|n| n.get());
        self.commands += 1;
    }
}

/// Battles per gen of the allocation check.
const ALLOC_BATTLES: u32 = 500;

/// The allocation check over one gen's battles (one test per gen, so the
/// two run in parallel).
fn no_allocation_in(gen: u8) {
    let (allocs, commands) = quiet(|| {
        let mut sweep = Sweep::default();
        let mut probe = AllocProbe::default();
        for i in 0..ALLOC_BATTLES {
            let r = run(gen, i, &mut probe);
            ARMED.with(|a| a.set(false));
            sweep.add(&r);
        }
        sweep.print("allocations", gen);
        println!("BUDGET allocs gen{gen} {} {} {}", probe.allocs, probe.commands, sweep.battles);
        (probe.allocs, probe.commands)
    });
    assert_eq!(allocs, 0, "gen {gen}: {allocs} allocations inside {commands} commands");
}

/// The instrument: an allocation inside the window counts, one outside
/// does not.
#[test]
fn the_counter_counts() {
    let mut probe = AllocProbe::default();
    let outside = std::hint::black_box(Vec::<u8>::with_capacity(8));
    probe.enter();
    let inside = std::hint::black_box(Vec::<u8>::with_capacity(8));
    probe.exit(Exit::Turn);
    drop((outside, inside));
    assert_eq!(probe.allocs, 1);
}

#[test]
fn no_allocation_inside_a_turn_gen1() {
    no_allocation_in(1);
}

#[test]
fn no_allocation_inside_a_turn_gen3() {
    no_allocation_in(3);
}

// ---- host time per turn ---------------------------------------------------------

/// Wall time per turn: the commands of one round of requests, summed.
#[derive(Default)]
struct TimeProbe {
    started: Option<Instant>,
    acc: u64,
    turns: Vec<u64>,
}

impl Probe for TimeProbe {
    fn enter(&mut self) {
        self.started = Some(Instant::now());
    }
    fn exit(&mut self, exit: Exit) {
        let ns = self.started.take().map(|t| t.elapsed().as_nanos() as u64).unwrap_or(0);
        match exit {
            Exit::Setup => {}
            Exit::Pending => self.acc += ns,
            Exit::Turn => {
                self.turns.push(self.acc + ns);
                self.acc = 0;
            }
        }
    }
}

/// The `q` quantile of sorted `v` (nearest rank).
fn quantile(v: &[u64], q: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    let rank = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[rank - 1]
}

#[test]
fn host_turn_times() {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    quiet(|| {
        for gen in GENS {
            let mut sweep = Sweep::default();
            let mut all = Vec::new();
            for i in 0..PROBE_BATTLES {
                let mut probe = TimeProbe::default();
                let r = run(gen, i, &mut probe);
                sweep.add(&r);
                // A stopped battle's turns before the stop are real turns.
                all.extend_from_slice(&probe.turns);
            }
            all.sort_unstable();
            sweep.print("turn times", gen);
            let (p50, p99, max) = (quantile(&all, 0.5), quantile(&all, 0.99), all.last().copied().unwrap_or(0));
            println!("turn times gen {gen} ({profile}): {} turns, p50 {p50} ns, p99 {p99} ns, max {max} ns", all.len());
            println!("BUDGET time gen{gen} {profile} {} {p50} {p99} {max}", all.len());
        }
    });
}

// ---- stack high-water mark ---------------------------------------------------------

/// Bytes painted below the probe's frame. A command that reaches the bottom
/// reports the whole window (then the window is too small).
const STACK_WINDOW: usize = 256 * 1024;
/// Left unpainted just below the probe's frame: `enter`'s own frame and
/// the x86-64 red zone.
const STACK_GAP: usize = 512;
const PAINT: u64 = 0xA5A5_5A5A_C3C3_3C3C;

/// Paints the stack below its own frame in `enter` and finds, in `exit`,
/// the lowest word a command overwrote. `enter` and the command are called
/// from the same frame of `driver::play`, so the depth is the command's
/// stack (`step`, `send_updates` and everything under them), less `enter`'s
/// small frame.
struct StackProbe {
    /// The painted window's top (exclusive) and how far down the last
    /// command dirtied it (so `enter` repaints only that).
    top: usize,
    dirty: usize,
    /// The deepest `>player` command (team setup, `start`, the leads).
    setup_max: usize,
    /// Per battle command (choices and turns).
    deepest: Vec<u64>,
}

impl StackProbe {
    fn new() -> StackProbe {
        StackProbe { top: 0, dirty: STACK_WINDOW, setup_max: 0, deepest: Vec::new() }
    }
}

impl Probe for StackProbe {
    #[inline(never)]
    fn enter(&mut self) {
        let marker = 0u8;
        let here = core::hint::black_box(&marker) as *const u8 as usize;
        let top = (here - STACK_GAP) & !7;
        let len = if top == self.top { self.dirty } else { STACK_WINDOW };
        self.top = top;
        let base = (top - len) as *mut u64;
        for w in 0..len / 8 {
            // SAFETY: the window lies below this frame (and its red zone)
            // inside the thread's stack mapping ([`stack_high_water`] runs
            // on a thread with a stack many times the window), in memory no
            // live frame uses.
            unsafe { core::ptr::write_volatile(base.add(w), PAINT) };
        }
    }

    #[inline(never)]
    fn exit(&mut self, exit: Exit) {
        let base = (self.top - STACK_WINDOW) as *const u64;
        let words = STACK_WINDOW / 8;
        // The lowest overwritten word, scanning up from the bottom.
        let mut lowest = words;
        for w in 0..words {
            // SAFETY: the window `enter` painted, below every live frame.
            if unsafe { core::ptr::read_volatile(base.add(w)) } != PAINT {
                lowest = w;
                break;
            }
        }
        let depth = (words - lowest) * 8;
        self.dirty = depth.max(8);
        if exit == Exit::Setup {
            self.setup_max = self.setup_max.max(depth);
        } else {
            self.deepest.push(depth as u64);
        }
    }
}

#[test]
fn stack_high_water() {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let report = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            quiet(|| {
                let mut lines = Vec::new();
                for gen in GENS {
                    let mut sweep = Sweep::default();
                    let mut probe = StackProbe::new();
                    for i in 0..PROBE_BATTLES {
                        let r = run(gen, i, &mut probe);
                        sweep.add(&r);
                    }
                    probe.deepest.sort_unstable();
                    let p50 = quantile(&probe.deepest, 0.5);
                    let p99 = quantile(&probe.deepest, 0.99);
                    let max = probe.deepest.last().copied().unwrap_or(0);
                    let full = max as usize >= STACK_WINDOW || probe.setup_max >= STACK_WINDOW;
                    lines.push(format!(
                        "stack gen {gen} ({profile}): {} choice commands, p50 {p50} B, p99 {p99} B, max {max} B; \
                         setup (>player) max {} B; below the driver{}",
                        probe.deepest.len(),
                        probe.setup_max,
                        if full { " (the whole window: raise STACK_WINDOW)" } else { "" }
                    ));
                    lines.push(format!(
                        "BUDGET stack gen{gen} {profile} {} {p50} {p99} {max} {}",
                        probe.deepest.len(),
                        probe.setup_max
                    ));
                }
                lines
            })
        })
        .unwrap()
        .join()
        .unwrap();
    for l in report {
        println!("{l}");
    }
}
