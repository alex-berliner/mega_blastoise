#![no_std]

pub mod mem_profile;
pub mod usb_cdc_line;

use rtt_target as _;
use panic_probe as _;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
