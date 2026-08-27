//! USB CDC console for board_v2 — the device's CLI, runner edition.
//!
//! RX: typed lines become [`DeviceEvent`]s for the shared runner. All
//! grammar lives in core (`p1 a` button lines, `p1 ok` lobby lines, battle
//! choice lines); this task only assembles bytes into lines, echoes them,
//! and intercepts the two platform commands:
//!
//!   :reset            reboot the chip
//!   :anim on|off      the runner's animation gate
//!
//! TX: every `line_out` from the port is mirrored here (RTT keeps getting
//! them too). Lines are dropped when no host is attached or the queue is
//! full — the console is an observer, never allowed to stall the game
//! (the CDC write-blocks-without-a-reader gotcha).
//!
//! NOTE: binds USBCTRL_IRQ, as does the v1 `usb` subsystem — `board_v2`
//! and `usb` are mutually exclusive features.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;

use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};
use mega_blastoise_core::device_runner::DeviceEvent;

embassy_rp::bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Console lines waiting for the host. Dropped when full: see module docs.
static LOG: Channel<CriticalSectionRawMutex, String, 16> = Channel::new();

/// Mirror one console line to the CDC host (drops when detached/behind).
pub fn log_line(line: &str) {
    let _ = LOG.try_send(String::from(line));
}

#[embassy_executor::task]
async fn usb_device_task(usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

async fn write_all(sender: &mut Sender<'static, UsbDriver<'static, USB>>, bytes: &[u8]) {
    for chunk in bytes.chunks(62) {
        if sender.write_packet(chunk).await.is_err() {
            return;
        }
    }
}

#[embassy_executor::task]
async fn tx_task(mut sender: Sender<'static, UsbDriver<'static, USB>>) -> ! {
    loop {
        let line = LOG.receive().await;
        if !sender.dtr() {
            continue; // no host: drop, never block the queue drain
        }
        write_all(&mut sender, line.as_bytes()).await;
        write_all(&mut sender, b"\r\n").await;
    }
}

fn dispatch(line: &str) {
    let cmd = line.trim();
    if cmd.is_empty() {
        return;
    }
    match cmd {
        ":reset" | ":restart" => {
            defmt::info!("console: :reset — rebooting");
            cortex_m::peripheral::SCB::sys_reset();
        }
        ":bl" | ":bootloader" => {
            defmt::info!("console: :bl — rebooting to BOOTSEL");
            // Datasheet 5.5.10.1: type BOOTSEL (0x0002) | NO_RETURN (0x0100),
            // 100 ms grace so this USB device detaches cleanly first.
            embassy_rp::rom_data::reboot(0x0102, 100, 0, 0);
        }
        ":anim off" => {
            let _ = crate::board_v2::INPUT.try_send(DeviceEvent::SetAnim(false));
            log_line("[anim] animations OFF");
        }
        ":anim on" => {
            let _ = crate::board_v2::INPUT.try_send(DeviceEvent::SetAnim(true));
            log_line("[anim] animations ON");
        }
        other => {
            let _ = crate::board_v2::INPUT.try_send(DeviceEvent::Line(String::from(other)));
        }
    }
}

#[embassy_executor::task]
async fn rx_task(mut receiver: Receiver<'static, UsbDriver<'static, USB>>) -> ! {
    let mut buf = [0u8; 64];
    let mut line = String::new();
    loop {
        receiver.wait_connection().await;
        line.clear();
        loop {
            let n = match receiver.read_packet(&mut buf).await {
                Ok(n) => n,
                Err(_) => break, // disconnected: back to wait_connection
            };
            for &b in &buf[..n] {
                match b {
                    b'\r' | b'\n' => {
                        if !line.is_empty() {
                            defmt::debug!("console rx: {=str}", line.as_str());
                            dispatch(&line);
                            line.clear();
                        }
                    }
                    0x08 | 0x7f => {
                        line.pop();
                    }
                    0x20..=0x7e => {
                        // Cap runaway lines; the longest real command is short.
                        if line.len() < 120 {
                            line.push(b as char);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Bring up the CDC console: device task + echo-free RX + LOG drain TX.
pub fn init(usb_periph: Peri<'static, USB>, spawner: &Spawner) {
    let driver = UsbDriver::new(usb_periph, UsbIrqs);

    let mut config = UsbConfig::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("Device console");
    config.serial_number = Some("1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let cdc_state = Box::leak(Box::new(State::new()));
    let config_buf = Box::leak(Box::new([0u8; 256]));
    let bos_buf = Box::leak(Box::new([0u8; 256]));
    let msos_buf = Box::leak(Box::new([0u8; 256]));
    let ctrl_buf = Box::leak(Box::new([0u8; 64]));

    let mut builder = Builder::new(driver, config, config_buf, bos_buf, msos_buf, ctrl_buf);
    let cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);
    let usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>> =
        Box::leak(Box::new(builder.build()));

    spawner.spawn(usb_device_task(usb)).expect("usb device task");
    let (sender, receiver) = cdc.split();
    spawner.spawn(tx_task(sender)).expect("usb tx task");
    spawner.spawn(rx_task(receiver)).expect("usb rx task");
    defmt::info!("Device console ready on CDC");
}
