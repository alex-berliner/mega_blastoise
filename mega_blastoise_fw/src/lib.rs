#![no_std]

pub mod hp_bar;
pub mod mem_profile;
pub mod usb_cdc_line;

use cortex_m_rt::ExceptionFrame;
use rtt_target as _;

/// Drive GP25 (Pico onboard LED) high by writing directly to SIO registers.
/// Safe to call in a panic handler — no Embassy HAL required.
fn drive_error_led() {
    unsafe {
        // Configure GP25 for SIO function (FUNCSEL = 5).
        (0x4001_40CC_u32 as *mut u32).write_volatile(5);
        // Enable output.
        (0xD000_0024_u32 as *mut u32).write_volatile(1 << 25);
        // Set output HIGH.
        (0xD000_0014_u32 as *mut u32).write_volatile(1 << 25);
    }
}

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    defmt::error!("HardFault! PC=0x{:08x} LR=0x{:08x} PSR=0x{:08x}",
        ef.pc(), ef.lr(), ef.xpsr());
    loop { cortex_m::asm::wfi(); }
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    defmt::error!("{}", defmt::Display2Format(info));
    cortex_m::asm::udf()
}

#[defmt::panic_handler]
fn defmt_panic() -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    cortex_m::asm::udf()
}
