//! USB for board_v2: the CDC console (the device's CLI, runner edition) and the
//! phone bridge's vendor interface (WebUSB), on one cable.
//!
//! The bridge interface is vendor class 0xff with one bulk IN and one bulk
//! OUT endpoint, plus the WebUSB BOS capability and landing page, so Chrome
//! on an Android phone offers the bridge page and opens the board with a tap.
//! It carries newline-ended lines, nothing else; core's `online` decides what
//! they mean. `usb-vendor-only` drops the CDC console (the fallback if a
//! phone refuses the composite device).
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

// Without the CDC console its half of this file has no caller.
#![cfg_attr(feature = "usb-vendor-only", allow(unused))]

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
use embassy_usb::class::web_usb::{Config as WebUsbConfig, State as WebUsbState, Url, WebUsb};
use embassy_usb::driver::{Endpoint as _, EndpointIn as _, EndpointOut as _};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};
use mega_blastoise_core::device_runner::DeviceEvent;

embassy_rp::bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Console lines waiting for the host. Dropped when full: see module docs.
static LOG: Channel<CriticalSectionRawMutex, String, 16> = Channel::new();

/// Lines from the bridge page, for `DevicePort::link_recv`. A full queue stalls
/// the OUT endpoint, so the page's transfers wait: the flow control.
pub static LINK_IN: Channel<CriticalSectionRawMutex, String, 6> = Channel::new();
/// Lines for the bridge page. Dropped when full (no page reading).
static LINK_OUT: Channel<CriticalSectionRawMutex, String, 24> = Channel::new();

/// The longest bridge line kept: a Gen 3 request is about 3 KB.
const LINK_LINE_MAX: usize = 8192;
/// Where WebUSB-capable browsers send the player when the board is plugged in.
const LANDING_URL: &str = "https://alex-berliner.github.io/mega_blastoise/bridge/";
/// Bulk endpoint packet size (full speed).
const LINK_PACKET: usize = 64;

/// A line for the bridge page; dropped (and said so on RTT) if the queue is full.
pub fn link_send(line: &str) {
    if LINK_OUT.try_send(String::from(line)).is_err() {
        defmt::warn!("bridge: out queue full, dropped a line");
    }
}

#[embassy_executor::task]
async fn link_rx_task(mut ep: <UsbDriver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut) -> ! {
    let mut pkt = [0u8; LINK_PACKET];
    let mut line: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    loop {
        ep.wait_enabled().await;
        line.clear();
        loop {
            match ep.read(&mut pkt).await {
                Ok(n) => {
                    for &b in &pkt[..n] {
                        match b {
                            b'\n' => {
                                let text = String::from_utf8_lossy(&line).into_owned();
                                line.clear();
                                LINK_IN.send(text).await;
                            }
                            b'\r' => {}
                            _ if line.len() < LINK_LINE_MAX => line.push(b),
                            _ => {}
                        }
                    }
                }
                Err(_) => {
                    // The page closed or the cable moved: tell core so an
                    // online battle ends instead of waiting for ever.
                    let _ = LINK_IN.try_send(String::from("down"));
                    break;
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn link_tx_task(mut ep: <UsbDriver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn) -> ! {
    loop {
        ep.wait_enabled().await;
        while LINK_OUT.try_receive().is_ok() {} // stale lines from before this page
        'page: loop {
            let mut line = LINK_OUT.receive().await;
            line.push('\n');
            for chunk in line.as_bytes().chunks(LINK_PACKET) {
                if ep.write(chunk).await.is_err() {
                    break 'page;
                }
            }
        }
    }
}

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
        ":panic" => {
            let mut buf = [0u8; mega_blastoise_fw::panic_store::CAPACITY];
            match mega_blastoise_fw::panic_store::read(&mut buf) {
                Some(msg) => {
                    log_line("[panic] last stored crash:");
                    log_line(msg);
                }
                None => log_line("[panic] none stored"),
            }
        }
        ":panic clear" => {
            mega_blastoise_fw::panic_store::clear();
            log_line("[panic] cleared");
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

    let config_buf = Box::leak(Box::new([0u8; 256]));
    let bos_buf = Box::leak(Box::new([0u8; 256]));
    let msos_buf = Box::leak(Box::new([0u8; 256]));
    let ctrl_buf = Box::leak(Box::new([0u8; 64]));

    let mut builder = Builder::new(driver, config, config_buf, bos_buf, msos_buf, ctrl_buf);

    #[cfg(not(feature = "usb-vendor-only"))]
    let cdc = {
        let cdc_state = Box::leak(Box::new(State::new()));
        CdcAcmClass::new(&mut builder, cdc_state, 64)
    };

    // WebUSB: the BOS capability and the landing page URL (an interface of
    // its own without endpoints, which the page does not claim) ...
    let web_config = Box::leak(Box::new(WebUsbConfig {
        max_packet_size: LINK_PACKET as u16,
        landing_url: Some(Url::new(LANDING_URL)),
        vendor_code: 1,
    }));
    let web_state = Box::leak(Box::new(WebUsbState::new()));
    WebUsb::configure(&mut builder, web_state, web_config);
    // ... and the bridge's data interface: vendor class, one bulk pair.
    let (link_in, link_out) = {
        let mut func = builder.function(0xff, 0x00, 0x00);
        let mut iface = func.interface();
        let mut alt = iface.alt_setting(0xff, 0x00, 0x00, None);
        let ep_out = alt.endpoint_bulk_out(None, LINK_PACKET as u16);
        let ep_in = alt.endpoint_bulk_in(None, LINK_PACKET as u16);
        (ep_in, ep_out)
    };

    let usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>> =
        Box::leak(Box::new(builder.build()));

    spawner.spawn(usb_device_task(usb)).expect("usb device task");
    #[cfg(not(feature = "usb-vendor-only"))]
    {
        let (sender, receiver) = cdc.split();
        spawner.spawn(tx_task(sender)).expect("usb tx task");
        spawner.spawn(rx_task(receiver)).expect("usb rx task");
        defmt::info!("Device console ready on CDC");
    }
    spawner.spawn(link_rx_task(link_out)).expect("bridge rx task");
    spawner.spawn(link_tx_task(link_in)).expect("bridge tx task");
    defmt::info!("Phone bridge ready on the vendor interface");
}
