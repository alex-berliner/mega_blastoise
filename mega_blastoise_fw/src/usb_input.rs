extern crate alloc;

use alloc::string::String;

use battler::Request;
use defmt::info;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use crate::mem_profile::heap_snapshot;
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
                                let line = String::from(self.partial.trim());
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

    async fn write_choice_prompt_1_to_4(&mut self, n: usize) {
        match n {
            1 => self.write("Pick move [1]: ").await,
            2 => self.write("Pick move [1-2]: ").await,
            3 => self.write("Pick move [1-3]: ").await,
            _ => self.write("Pick move [1-4]: ").await,
        }
    }
}

impl<'d> BattleInput for UsbBattleInput<'d> {
    async fn read_choice(&mut self, player_id: &str, request: &Request) -> String {
        info!("read_choice player={}", defmt::Display2Format(player_id));
        let _ = player_id;

        match request {
            Request::Turn(turn) => {
                info!("request turn active_slots={}", turn.active.len());
                heap_snapshot("prompt_turn_enter");
                let mut parts = alloc::vec::Vec::new();
                for (active_idx, mon) in turn.active.iter().enumerate() {
                    let n = mon.moves.len().min(4);
                    info!("turn slot={} available_moves={}", active_idx, n);
                    for (i, m) in mon.moves.iter().take(4).enumerate() {
                        info!(
                            "  move {}: {} pp={}/{} disabled={}",
                            i + 1,
                            defmt::Display2Format(&m.name),
                            m.pp,
                            m.max_pp,
                            m.disabled
                        );
                    }
                    self.write("\r\n=== Choose move ===\r\n").await;
                    loop {
                        self.write_choice_prompt_1_to_4(n).await;
                        let line = self.read_line().await;
                        info!("input line={}", defmt::Display2Format(&line));
                        if let Ok(btn) = line.parse::<usize>() {
                            if btn >= 1 && btn <= n {
                                let slot = btn - 1;
                                let m = &mon.moves[slot];
                                if m.disabled || m.pp == 0 {
                                    self.write("That move cannot be used.\r\n").await;
                                    continue;
                                }
                                parts.push(format_move_choice(slot));
                                info!("accepted move slot={} name={}", slot, defmt::Display2Format(&m.name));
                                break;
                            }
                        }
                        self.write("Enter a valid move number.\r\n").await;
                    }
                }
                let out = join_choice_parts(&parts);
                heap_snapshot("prompt_turn_exit");
                out
            }
            Request::Switch(sw) => {
                info!("request switch slots_to_fill={}", sw.needs_switch.len());
                heap_snapshot("prompt_switch_enter");
                let mut parts = alloc::vec::Vec::new();
                for _ in &sw.needs_switch {
                    self.write("\r\n=== Switch (bench 1-6) ===\r\n").await;
                    loop {
                        self.write("Pick slot [1-6]: ").await;
                        let line = self.read_line().await;
                        info!("input line={}", defmt::Display2Format(&line));
                        if let Ok(n) = line.parse::<usize>() {
                            if n >= 1 && n <= 6 {
                                parts.push(format_switch_choice(n - 1));
                                info!("accepted switch slot={}", n - 1);
                                break;
                            }
                        }
                        self.write("Enter a number 1-6.\r\n").await;
                    }
                }
                let out = join_choice_parts(&parts);
                heap_snapshot("prompt_switch_exit");
                out
            }
            Request::TeamPreview(_) => {
                info!("request team_preview => random");
                String::from("random")
            }
            Request::LearnMove(_) => {
                info!("request learn_move => pass");
                String::from("pass")
            }
        }
    }
}
