#![no_std]

pub mod hp_bar;
pub mod mem_profile;
pub mod usb_cdc_line;

use cortex_m_rt::ExceptionFrame;
use rtt_target as _;

#[cfg(all(feature = "rp2040", feature = "rp2350"))]
compile_error!("features rp2040 and rp2350 are mutually exclusive — pick one chip");
#[cfg(not(any(feature = "rp2040", feature = "rp2350")))]
compile_error!("no chip selected — build with the default rp2040, or --no-default-features --features rp2350,...");

/// The RP2350 boot ROM refuses to start an image without an IMAGE_DEF block in
/// the first 4K of flash (memory-rp2350.x places the section). Living in the
/// lib rather than each binary works because every binary links the lib for
/// the panic handler, and codegen-units=1 makes the lib one object, so the
/// linker always pulls this in with it.
#[cfg(feature = "rp2350")]
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: embassy_rp::block::ImageDef = embassy_rp::block::ImageDef::secure_exe();

/// Drive GP25 (Pico onboard LED) high by writing directly to SIO registers.
/// Safe to call in a panic handler — no Embassy HAL required.
#[cfg(feature = "rp2040")]
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

/// Same, at the RP2350's addresses: IO_BANK0 moved to 0x4002_8000, the SIO
/// GPIO registers gained HI twins (OUT_SET 0x18, OE_SET 0x38), and pads now
/// reset ISOLATED — an isolated pad ignores its output driver, so clear ISO
/// (bit 8) first or nothing reaches the pin.
#[cfg(feature = "rp2350")]
fn drive_error_led() {
    unsafe {
        // PADS_BANK0 GPIO25: clear ISO.
        let pad = 0x4003_8068_u32 as *mut u32;
        pad.write_volatile(pad.read_volatile() & !(1 << 8));
        // IO_BANK0 GPIO25_CTRL: FUNCSEL = 5 (SIO).
        (0x4002_80CC_u32 as *mut u32).write_volatile(5);
        // SIO GPIO_OE_SET, then GPIO_OUT_SET.
        (0xD000_0038_u32 as *mut u32).write_volatile(1 << 25);
        (0xD000_0018_u32 as *mut u32).write_volatile(1 << 25);
    }
}

/// Crash-message persistence: SRAM9 is a direct-mapped 4K bank the linker
/// never places anything in, and a SYSRESETREQ does not clear RAM — so a
/// panic leaves its message there, reboots, and the next boot (or the
/// console's `:panic`) reads it back.
///
/// This replaced `udf()`: the lockup state udf left the core in also wedged
/// the SWD debug port, which made every crash look like a broken probe
/// cable — and left the device frozen instead of recovering to the lobby.
pub mod panic_store {
    const MAGIC: u32 = 0xC0DE_DEAD;
    const BASE: usize = 0x2008_1000; // SRAM9
    const CAP: usize = 1024;

    struct Writer {
        len: usize,
    }

    impl core::fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for &b in s.as_bytes() {
                if self.len >= CAP {
                    break;
                }
                unsafe { ((BASE + 8 + self.len) as *mut u8).write_volatile(b) };
                self.len += 1;
            }
            Ok(())
        }
    }

    /// Store a message. Called from the panic path only.
    pub fn store(args: core::fmt::Arguments<'_>) {
        use core::fmt::Write;
        let mut w = Writer { len: 0 };
        let _ = write!(w, "{}", args);
        unsafe {
            ((BASE + 4) as *mut u32).write_volatile(w.len as u32);
            (BASE as *mut u32).write_volatile(MAGIC);
        }
    }

    /// The stored message from the previous run, if any. Does not clear.
    pub fn read(buf: &mut [u8; CAP]) -> Option<&str> {
        unsafe {
            if (BASE as *const u32).read_volatile() != MAGIC {
                return None;
            }
            let len = ((BASE + 4) as *const u32).read_volatile() as usize;
            if len > CAP {
                return None;
            }
            for (i, slot) in buf.iter_mut().take(len).enumerate() {
                *slot = ((BASE + 8 + i) as *const u8).read_volatile();
            }
            core::str::from_utf8(&buf[..len]).ok()
        }
    }

    pub fn clear() {
        unsafe { (BASE as *mut u32).write_volatile(0) };
    }

    pub const CAPACITY: usize = CAP;
}

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    defmt::error!("HardFault! PC=0x{:08x} LR=0x{:08x} PSR=0x{:08x}",
        ef.pc(), ef.lr(), ef.xpsr());
    panic_store::store(format_args!(
        "HardFault PC=0x{:08x} LR=0x{:08x} PSR=0x{:08x}",
        ef.pc(),
        ef.lr(),
        ef.xpsr()
    ));
    cortex_m::peripheral::SCB::sys_reset()
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    defmt::error!("{}", defmt::Display2Format(info));
    panic_store::store(format_args!("{}", info));
    cortex_m::peripheral::SCB::sys_reset()
}

#[defmt::panic_handler]
fn defmt_panic() -> ! {
    cortex_m::interrupt::disable();
    drive_error_led();
    panic_store::store(format_args!("defmt panic"));
    cortex_m::peripheral::SCB::sys_reset()
}
