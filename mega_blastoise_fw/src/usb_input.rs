extern crate alloc;

use alloc::string::{String, ToString};

use battler::Request;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{format_move_choice, format_switch_choice, join_choice_parts, BattleInput};

pub struct UsbBattleInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
    partial: String,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self { sender, receiver, partial: String::new() }
    }

    async fn write(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let mut start = 0;
        while start < bytes.len() {
            let end = (start + 63).min(bytes.len());
            let _ = self.sender.write_packet(&bytes[start..end]).await;
            start = end;
        }
    }

    async fn read_line(&mut self) -> String {
        self.receiver.wait_connection().await;
        let mut buf = [0u8; 64];
        loop {
            match self.receiver.read_packet(&mut buf).await {
                Ok(n) => {
                    for &b in &buf[..n] {
                        match b {
                            b'\r' | b'\n' => {
                                let line = self.partial.trim().to_string();
                                self.partial.clear();
                                if !line.is_empty() {
                                    return line;
                                }
                            }
                            b'\x08' | b'\x7f' => { self.partial.pop(); }
                            _ => self.partial.push(b as char),
                        }
                    }
                }
                Err(_) => {
                    self.partial.clear();
                    self.receiver.wait_connection().await;
                }
            }
        }
    }
}

impl<'d> BattleInput for UsbBattleInput<'d> {
    async fn read_choice(&mut self, player_id: &str, request: &Request) -> String {
        let label = match player_id { "p1" => "Red", "p2" => "Blue", _ => player_id };

        match request {
            Request::Turn(turn) => {
                let mut parts = alloc::vec::Vec::new();
                for mon in &turn.active {
                    let n = mon.moves.len().min(4);
                    let mut menu = alloc::format!("\r\n=== {label} — choose move ===\r\n");
                    for (i, m) in mon.moves.iter().take(4).enumerate() {
                        let dis = if m.disabled || m.pp == 0 { " (disabled)" } else { "" };
                        menu.push_str(&alloc::format!(
                            "  [{}] {}  PP {}/{}{}\r\n", i + 1, m.name, m.pp, m.max_pp, dis
                        ));
                    }
                    self.write(&menu).await;
                    loop {
                        self.write(&alloc::format!("{label} pick move [1-{n}]: ")).await;
                        let line = self.read_line().await;
                        if let Ok(btn) = line.parse::<usize>() {
                            if btn >= 1 && btn <= n {
                                let slot = btn - 1;
                                let m = &mon.moves[slot];
                                if m.disabled || m.pp == 0 {
                                    self.write("That move cannot be used.\r\n").await;
                                    continue;
                                }
                                parts.push(format_move_choice(slot));
                                break;
                            }
                        }
                        self.write(&alloc::format!("Enter a number 1-{n}.\r\n")).await;
                    }
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let mut parts = alloc::vec::Vec::new();
                for _ in &sw.needs_switch {
                    self.write(&alloc::format!("\r\n=== {label} — switch (bench 1-6) ===\r\n")).await;
                    loop {
                        self.write(&alloc::format!("{label} pick slot [1-6]: ")).await;
                        let line = self.read_line().await;
                        if let Ok(n) = line.parse::<usize>() {
                            if n >= 1 && n <= 6 {
                                parts.push(format_switch_choice(n - 1));
                                break;
                            }
                        }
                        self.write("Enter a number 1-6.\r\n").await;
                    }
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => "random".to_string(),
            Request::LearnMove(_) => "pass".to_string(),
        }
    }
}
