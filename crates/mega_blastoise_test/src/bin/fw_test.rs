//! Closed-loop hardware test for mega_blastoise_fw.
//!
//! Connects to the running firmware over USB CDC (serial) and RTT (probe-rs).
//! Sends button-press events (single digit `1`) to drive the battle — identical
//! to pressing physical button 1 on the board.  Asserts:
//!   - the battle ends with a win event in the RTT log
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

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

const DEFAULT_SERIAL: &str = "/dev/ttyACM1";
const DEFAULT_ELF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/thumbv6m-none-eabi/debug/mega-blastoise-fw",
);
const PROBE_ID: &str = "2e8a:000c:E663589863798621";
const BATTLE_TIMEOUT: Duration = Duration::from_secs(120);
// How often to send a button press when we haven't heard back.
const BUTTON_INTERVAL: Duration = Duration::from_millis(200);

enum Msg {
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let serial_port = args.get(1).map(String::as_str).unwrap_or(DEFAULT_SERIAL);
    let elf_path = args.get(2).map(String::as_str).unwrap_or(DEFAULT_ELF);

    println!("fw_test: serial={serial_port}  elf={elf_path}");

    let (tx, rx) = channel::<Msg>();
    spawn_rtt_reader(elf_path.to_string(), tx);

    let mut write_port = serialport::new(serial_port, 115_200)
        .timeout(Duration::from_millis(50))
        .open()
        .unwrap_or_else(|e| {
            eprintln!("fw_test: cannot open {serial_port}: {e}");
            std::process::exit(2);
        });

    println!("fw_test: connected — streaming button '1' presses to drive battle…");

    let start = Instant::now();
    let mut last_button = Instant::now() - BUTTON_INTERVAL;

    loop {
        if start.elapsed() > BATTLE_TIMEOUT {
            eprintln!("fw_test: TIMEOUT — battle did not finish within {}s", BATTLE_TIMEOUT.as_secs());
            std::process::exit(1);
        }

        // Send a button press at regular intervals — the firmware consumes one
        // per prompt, so flooding with '1' presses is safe (channel-buffered).
        if last_button.elapsed() >= BUTTON_INTERVAL {
            let _ = write_port.write_all(b"1");
            last_button = Instant::now();
        }

        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Msg::RttLine(line)) => {
                println!("[RTT]  {line}");
                if line.contains("wins!") || line.contains("Battle over") || line.contains("=== Battle over ===") {
                    println!("\nfw_test: PASS — battle completed");
                    std::process::exit(0);
                }
            }
            Err(_) => {}
        }
    }
}
