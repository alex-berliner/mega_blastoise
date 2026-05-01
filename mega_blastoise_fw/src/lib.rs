#![no_std]

pub mod mem_profile;

use rtt_target as _;
use panic_probe as _;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
