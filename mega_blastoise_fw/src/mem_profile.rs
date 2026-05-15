use core::sync::atomic::{AtomicUsize, Ordering};

use defmt::info;
use embedded_alloc::Heap;

// gen1_battle peaks at <10 KB of heap (vs battler's 115 KB which forced the
// old 128 KB). 64 KB keeps ~20× headroom over peak; the freed 64 KB goes to
// the RTT buffer (see main.rs) with stack budget unchanged.
pub const HEAP_SIZE: usize = 64 * 1024;

#[global_allocator]
pub static HEAP: Heap = Heap::empty();

static PEAK_USED: AtomicUsize = AtomicUsize::new(0);

pub fn init_heap() {
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
    unsafe { HEAP.init(core::ptr::addr_of!(HEAP_MEM) as usize, HEAP_SIZE) }
}

pub fn heap_snapshot(tag: &str) {
    let used = HEAP.used();
    let free = HEAP.free();

    let prev_peak = PEAK_USED.load(Ordering::Relaxed);
    if used > prev_peak {
        PEAK_USED.store(used, Ordering::Relaxed);
    }
    let peak = PEAK_USED.load(Ordering::Relaxed);

    info!(
        "heap[{=str}] used={} free={} peak={} total={}",
        tag, used, free, peak, HEAP_SIZE
    );
}
