//! Hardware recovery line for the mega_blastoise rig.
//!
//! When an RP2040 (the player Pico, or the Pico running the debug probe)
//! wedges so hard that nothing software-side can reach it, this ESP32 yanks
//! its `RUN` pin low for a moment to force a clean power-on reset.
//!
//! ## Wiring (DOIT ESP32 DevKit V1)
//!
//! ```text
//!   ESP32 GPIO25 ───────────────► Pico   RUN  (pin 30)
//!   ESP32 GPIO26 ───────────────► Probe  RUN  (pin 30 of the probe Pico)
//!   ESP32 GND    ───────────────► common GND  (REQUIRED — shared ground)
//! ```
//!
//! The GPIOs are **open-drain**: they can only pull `RUN` low or float
//! (Hi-Z). They never drive high. So if this ESP32 is unplugged, crashed,
//! mid-reset, or still in the bootloader, every line is Hi-Z and each
//! RP2040's internal ~50 kΩ `RUN` pull-up keeps it running normally. The
//! fail-safe ("ESP32 down ⇒ Pis stay on") is structural, not firmware.
//!
//! GPIO25/26 are deliberate: on the ESP32 they come up as Hi-Z inputs at
//! power-on, are not strapping pins, and don't glitch during boot — so the
//! ESP32's own power-up can't spuriously reset the targets.
//!
//! ## Trigger protocol
//!
//! Magic bytes on UART0 @ 115200 (the same port you flash/monitor — the
//! onboard CP2102 USB bridge; the original ESP32 has no native USB):
//!
//! | byte  | action                |
//! |-------|-----------------------|
//! | `p`   | reset the Pico        |
//! | `d`   | reset the debug probe |
//! | `b`   | reset both            |
//!
//! Any other byte is ignored (so stray console noise is harmless). Each
//! accepted command pulses the line low for [`PULSE_MS`] then releases it,
//! and echoes a one-line ack so the host knows it landed.

#![no_std]
#![no_main]

use embedded_io::{Read, Write};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, OutputOpenDrain, Pull},
    main,
    uart::{Config as UartConfig, Uart},
};

/// How long to hold `RUN` low. RP2040 needs only microseconds; 100 ms is a
/// generous, unambiguous pulse that survives any debounce/RC on the line.
const PULSE_MS: u32 = 100;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Idle = Level::High in open-drain == not driven == Hi-Z. The target's
    // internal RUN pull-up does the actual pulling-high.
    let mut pico = OutputOpenDrain::new(peripherals.GPIO25, Level::High, Pull::None);
    let mut probe = OutputOpenDrain::new(peripherals.GPIO26, Level::High, Pull::None);

    // UART0 on its default pins (GPIO1 TX / GPIO3 RX) — that's what the
    // onboard USB-UART bridge is wired to.
    let mut uart = Uart::new(peripherals.UART0, UartConfig::default())
        .expect("UART0 init")
        .with_tx(peripherals.GPIO1)
        .with_rx(peripherals.GPIO3);

    let delay = Delay::new();

    let _ = uart.write_all(b"esp32_reset ready: p=pico d=probe b=both\r\n");

    let mut byte = [0u8; 1];
    loop {
        // Blocking single-byte read. Errors (framing/overrun from line
        // noise or a host (dis)connecting) are non-fatal — just retry.
        if uart.read(&mut byte).is_err() {
            continue;
        }
        match byte[0] {
            b'p' => {
                pulse(&mut pico, &delay);
                let _ = uart.write_all(b"RST pico\r\n");
            }
            b'd' => {
                pulse(&mut probe, &delay);
                let _ = uart.write_all(b"RST probe\r\n");
            }
            b'b' => {
                // Hold both low together so they come back up in lockstep.
                pico.set_low();
                probe.set_low();
                delay.delay_millis(PULSE_MS);
                pico.set_high();
                probe.set_high();
                let _ = uart.write_all(b"RST both\r\n");
            }
            _ => {} // ignore stray bytes / console chatter
        }
    }
}

/// Drive one `RUN` line low for [`PULSE_MS`], then release it to Hi-Z.
fn pulse(line: &mut OutputOpenDrain<'_>, delay: &Delay) {
    line.set_low();
    delay.delay_millis(PULSE_MS);
    line.set_high(); // open-drain "high" = release to Hi-Z
}
