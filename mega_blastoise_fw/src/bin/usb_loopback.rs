//! CDC ACM loopback: bytes are echoed back; **every completed line is sent to the host as CRLF**
//! (`\r\n`) regardless of whether the host sent `\r`, `\n`, or `\r\n`.
//!
//! **Line detection:** flush on `\r` **or** `\n`. After `\r`, the next `\n` is absorbed (Windows /
//! CRLF) so one logical newline never produces two RTT lines or two echoed CRLFs. A lone `\n`
//! (Unix) still ends the line.
//!
//! Complete lines are logged over defmt (RTT on SWD) as human-readable UTF-8.
//!
//! Build / flash:
//! `cargo build --bin usb_loopback`
//! `cargo run --bin usb_loopback` (uses `.cargo/config` probe-rs runner)

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn usb_task(usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

const LINE_BUF_CAP: usize = 256;
const TX_CHUNK: usize = 64;

fn log_line_to_rtt(line: &[u8]) {
    if line.is_empty() {
        return;
    }
    match core::str::from_utf8(line) {
        Ok(s) => info!("cdc line: {}", s),
        Err(_) => info!("cdc line (non-utf8) {} B: {=[u8]:02x}", line.len(), line),
    }
}

async fn write_all(
    sender: &mut embassy_usb::class::cdc_acm::Sender<'static, UsbDriver<'static, USB>>,
    mut data: &[u8],
) {
    while !data.is_empty() {
        let n = data.len().min(TX_CHUNK);
        let _ = sender.write_packet(&data[..n]).await;
        data = &data[n..];
    }
}

async fn finish_line(
    line: &[u8],
    line_len: &mut usize,
    sender: &mut embassy_usb::class::cdc_acm::Sender<'static, UsbDriver<'static, USB>>,
) {
    log_line_to_rtt(&line[..*line_len]);
    *line_len = 0;
    write_all(sender, b"\r\n").await;
}

/// Echo with CRLF on every completed line; see module docs.
async fn cdc_echo(
    mut sender: embassy_usb::class::cdc_acm::Sender<'static, UsbDriver<'static, USB>>,
    mut receiver: embassy_usb::class::cdc_acm::Receiver<'static, UsbDriver<'static, USB>>,
) -> ! {
    let mut buf = [0u8; 64];
    let mut line = [0u8; LINE_BUF_CAP];
    let mut line_len = 0usize;
    // After `\r`, ignore one `\n` so `\r\n` is a single line end (not two).
    let mut skip_next_lf = false;

    loop {
        match receiver.read_packet(&mut buf).await {
            Ok(n) => {
                for &b in &buf[..n] {
                    if skip_next_lf {
                        if b == b'\n' {
                            skip_next_lf = false;
                            continue;
                        }
                        skip_next_lf = false;
                    }
                    match b {
                        b'\r' => {
                            finish_line(&line, &mut line_len, &mut sender).await;
                            skip_next_lf = true;
                        }
                        b'\n' => {
                            finish_line(&line, &mut line_len, &mut sender).await;
                        }
                        _ => {
                            write_all(&mut sender, &[b]).await;
                            if line_len < LINE_BUF_CAP {
                                line[line_len] = b;
                                line_len += 1;
                            } else {
                                info!(
                                    "cdc line buffer full ({} B), discarding partial line",
                                    LINE_BUF_CAP
                                );
                                line_len = 0;
                            }
                        }
                    }
                }
            }
            Err(_) => {
                info!("cdc RX stalled — wait for host");
                receiver.wait_connection().await;
                info!("cdc RX ready");
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    info!("USB CDC loopback — echo uses CRLF per line; lines logged on RTT");

    let p = embassy_rp::init(Default::default());

    let driver = UsbDriver::new(p.USB, Irqs);
    let mut config = UsbConfig::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("USB loopback test");
    config.serial_number = Some("loopback");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let cdc_state = Box::leak(Box::new(State::new()));
    let config_buf = Box::leak(Box::new([0u8; 256]));
    let bos_buf = Box::leak(Box::new([0u8; 256]));
    let msos_buf = Box::leak(Box::new([0u8; 256]));
    let ctrl_buf = Box::leak(Box::new([0u8; 64]));

    let mut builder = Builder::new(driver, config, config_buf, bos_buf, msos_buf, ctrl_buf);
    let cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);
    let usb = Box::leak(Box::new(builder.build()));
    spawner.spawn(usb_task(usb)).unwrap();
    let (sender, receiver) = cdc.split();
    cdc_echo(sender, receiver).await;
}
