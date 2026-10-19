use core::sync::atomic::{AtomicUsize, Ordering};

use defmt::info;
use embedded_alloc::Heap;

// The RP2350 has 520 KB of RAM and the single-screen UI heap-allocates its
// 320x480 RGB565 frame (300 KB), so the heap is sized for that. One frame in
// flight (st7796::IDLE) is what makes 384 KB enough.
pub const HEAP_SIZE: usize = 384 * 1024;

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
