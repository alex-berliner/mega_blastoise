//! Closed-loop hardware test for mega_blastoise_fw.
//!
//! Connects to the running firmware over USB CDC (serial) and RTT (probe-rs).
//! Plays a full battle by always choosing the first available move/slot, then asserts:
//!   - every command is accepted (no [!!] rejections)
//!   - the battle ends with a win event
//!
//! Requires:
//!   - mega_blastoise_fw already flashed and running on the RP2040
//!   - probe-rs in PATH (with access to the debug probe)
//!
//! Usage:
//!   cargo run -p mega-blastoise-test --bin fw_test -- [serial_port] [elf_path]
//!
//! Defaults:
//!   serial_port  /dev/ttyACM1
//!   elf_path     ../mega_blastoise_fw/target/thumbv6m-none-eabi/debug/mega-blastoise-fw

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

const DEFAULT_SERIAL: &str = "/dev/ttyACM1";
const DEFAULT_ELF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/thumbv6m-none-eabi/debug/mega-blastoise-fw",
);
// Probe serial from .cargo/config.toml runner line.
const PROBE_ID: &str = "2e8a:000c:E663589863798621";
// How long to wait for any activity before declaring a hang.
const STEP_TIMEOUT: Duration = Duration::from_secs(30);
// Timeout on each serial read; partial-line detection fires after one quiet window.
const SERIAL_READ_TIMEOUT: Duration = Duration::from_millis(50);

enum Msg {
    /// Complete \n-terminated line from USB CDC (stripped of \r\n).
    UsbLine(String),
    /// Partial content that has been sitting without a newline for one read window.
    /// Used to detect the move/switch prompts that the firmware writes without a trailing CRLF.
    UsbPartial(String),
    /// A line from the probe-rs RTT stream.
    RttLine(String),
}

fn spawn_rtt_reader(elf: String, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let child = Command::new("probe-rs")
            .args([
                "attach",
                "--chip",
                "RP2040",
                "--probe",
                PROBE_ID,
                "--speed",
                "8000",
                "--log-format",
                "oneline",
                &elf,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("fw_test: could not spawn probe-rs: {e}  (RTT unavailable)");
                return;
            }
        };

        let reader = BufReader::new(child.stdout.take().unwrap());
        for line in reader.lines().flatten() {
            if tx.send(Msg::RttLine(line)).is_err() {
                break;
            }
        }
        let _ = child.wait();
    });
}

fn spawn_serial_reader(mut port: Box<dyn serialport::SerialPort>, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let mut accum = String::new();
        let mut buf = [0u8; 256];
        loop {
            match port.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    accum.push_str(&String::from_utf8_lossy(&buf[..n]));
                    // Emit all complete lines.
                    while let Some(pos) = accum.find('\n') {
                        let line = accum[..pos].trim_end_matches('\r').to_string();
                        accum = accum[pos + 1..].to_string();
                        if tx.send(Msg::UsbLine(line)).is_err() {
                            return;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // If there's partial content that looks like an input prompt, emit it.
                    // The move prompt "Move [1-4]: " and switch prompt "Send in party slot [1-6]: "
                    // are both written without a trailing CRLF, so they never become complete lines.
                    if !accum.is_empty() && accum.ends_with(": ") {
                        if tx.send(Msg::UsbPartial(accum.clone())).is_err() {
                            return;
                        }
                    }
                }
                Err(_) => return,
            }
        }
    });
}

// Parses "[N] ..." → N from bench-slot lines like "    [3] Blastoise — HP 201/201 (100%)  <-- available"
fn parse_slot_number(line: &str) -> Option<usize> {
    let t = line.trim();
    if !t.starts_with('[') {
        return None;
    }
    let close = t.find(']')?;
    t[1..close].trim().parse().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let serial_port = args.get(1).map(String::as_str).unwrap_or(DEFAULT_SERIAL);
    let elf_path = args.get(2).map(String::as_str).unwrap_or(DEFAULT_ELF);

    println!("fw_test: serial={serial_port}  elf={elf_path}");

    let (tx, rx) = channel::<Msg>();

    // Open the serial port twice: one end goes to the reader thread, the other stays here for
    // writing commands.  serialport::try_clone() duplicates the file descriptor.
    let read_port = serialport::new(serial_port, 115_200)
        .timeout(SERIAL_READ_TIMEOUT)
        .open()
        .unwrap_or_else(|e| {
            eprintln!("fw_test: cannot open {serial_port}: {e}");
            std::process::exit(2);
        });
    let mut write_port = read_port
        .try_clone()
        .expect("clone serial port for writing");

    spawn_serial_reader(read_port, tx.clone());
    spawn_rtt_reader(elf_path.to_string(), tx.clone());

    // ── Test state ─────────────────────────────────────────────────────────────

    // Bench slots seen with "<-- available" since the last SWITCH REQUIRED header.
    let mut available_slots: Vec<usize> = Vec::new();
    // The last partial we acted on; prevents re-sending the same prompt on repeated timeouts.
    let mut last_partial: Option<String> = None;
    // Number of commands accepted by the firmware.
    let mut ok_count: usize = 0;
    // Last time we received any message — used to detect hangs.
    let mut last_activity = Instant::now();

    println!("fw_test: connected — waiting for battle prompts…");

    loop {
        // Enforce the step timeout regardless of whether recv blocks.
        if last_activity.elapsed() > STEP_TIMEOUT {
            eprintln!(
                "fw_test: TIMEOUT — no activity for {}s (ok_count={})",
                STEP_TIMEOUT.as_secs(),
                ok_count
            );
            std::process::exit(1);
        }

        let msg = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(m) => m,
            Err(_) => continue,
        };
        last_activity = Instant::now();

        match msg {
            // ── Complete USB lines ─────────────────────────────────────────────
            Msg::UsbLine(line) => {
                println!("[USB]  {line}");

                // Any complete line resets partial dedup so a fresh prompt is recognised.
                last_partial = None;

                // Track available bench slots for the upcoming switch prompt.
                if line.contains("SWITCH REQUIRED") {
                    available_slots.clear();
                } else if line.contains("<-- available") {
                    if let Some(slot) = parse_slot_number(&line) {
                        available_slots.push(slot);
                    }
                }

                if line.starts_with("[OK]") {
                    ok_count += 1;
                    println!("fw_test: ✓ accepted command #{ok_count}");
                } else if line.starts_with("[!!]") {
                    eprintln!("fw_test: FAIL — firmware rejected a command: {line}");
                    std::process::exit(1);
                } else if line.starts_with("[EVT]") {
                    let event = line.trim_start_matches("[EVT]").trim();
                    if event.contains("wins!") || event.contains("Battle over") {
                        if ok_count == 0 {
                            eprintln!("fw_test: FAIL — battle ended but zero commands were accepted");
                            std::process::exit(1);
                        }
                        println!(
                            "\nfw_test: PASS — battle completed normally ({ok_count} commands accepted)"
                        );
                        std::process::exit(0);
                    }
                }
            }

            // ── Input prompts (partial lines without CRLF) ────────────────────
            Msg::UsbPartial(partial) => {
                // Skip if we already reacted to this exact partial content.
                if last_partial.as_deref() == Some(partial.as_str()) {
                    continue;
                }
                last_partial = Some(partial.clone());
                println!("[USB~] {}", partial.trim_end());

                if partial.contains("Move [") {
                    // Always pick slot 1.  If it's rejected (disabled / no PP), the firmware
                    // re-prompts and last_partial will be cleared by the [!!] line, so we'll
                    // try slot 1 again.  For the demo teams this should never be rejected.
                    println!("fw_test: → sending move \"1\"");
                    let _ = write_port.write_all(b"1\r\n");
                } else if partial.contains("Send in party slot") {
                    // Pick the first bench slot that was marked available; default to 2 if
                    // the list is empty (shouldn't happen in a normal battle).
                    let slot = available_slots.first().copied().unwrap_or(2);
                    println!("fw_test: → sending switch slot \"{slot}\"");
                    let _ = write_port.write_all(format!("{slot}\r\n").as_bytes());
                    available_slots.clear();
                }
            }

            // ── RTT lines (informational; logged but not used to drive logic) ──
            Msg::RttLine(line) => {
                println!("[RTT]  {line}");
            }
        }
    }
}
