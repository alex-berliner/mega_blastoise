extern crate alloc;

use alloc::string::String;

use battler::{MonBattleData, Request};
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus,
    InputSource,
};
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

pub struct UsbBattleInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
    partial: String,
    /// Last non-empty line submitted at a prompt; Enter on an empty line resends it.
    last_typed_line: Option<String>,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self {
            sender,
            receiver,
            partial: String::new(),
            last_typed_line: None,
        }
    }

    pub async fn run(&mut self, bus: &InputBus) {
        loop {
            // While waiting for the next prompt, relay any log lines that arrive.
            let prompt = loop {
                match select(bus.prompt.wait(), bus.log.receive()).await {
                    Either::First(p) => break p,
                    Either::Second(line) => {
                        self.writeln(&line).await;
                    }
                }
            };
            let ActivePrompt { player_id, request } = prompt;
            let choice = self.handle(&player_id, &request).await;
            bus.choices.send(choice).await;

            // Turn processing pushes log lines before the next prompt is signaled; drain them
            // here so they are not skipped when `prompt.wait()` wins the next select.
            while let Ok(line) = bus.log.try_receive() {
                self.writeln(&line).await;
            }
        }
    }

    async fn handle(&mut self, player_id: &str, request: &Request) -> String {
        use defmt::info;
        info!("");
        match request {
            Request::Turn(turn) => {
                self.write("\r\n").await;
                self.write("══════════════════════════════════\r\n").await;

                // Show both sides' active Pokémon from allies list.
                for player in &turn.allies {
                    let label = Self::player_label(&player.id);
                    let actives: alloc::vec::Vec<&MonBattleData> =
                        player.mons.iter().filter(|m| m.active).collect();
                    for m in &actives {
                        let status = m.status.as_deref().unwrap_or("—");
                        let item = m.item.as_deref().unwrap_or("—");
                        self.writef(&alloc::format!(
                            "{} — {} ({})  HP {}/{}  status: {}  item: {}\r\n",
                            label, m.summary.name, m.species, m.hp, m.max_hp, status, item
                        )).await;
                        // Boosts
                        let b = &m.boosts;
                        if b.atk != 0 || b.def != 0 || b.spa != 0 || b.spd != 0 || b.spe != 0 {
                            self.writef(&alloc::format!(
                                "  boosts  atk:{:+}  def:{:+}  spa:{:+}  spd:{:+}  spe:{:+}\r\n",
                                b.atk, b.def, b.spa, b.spd, b.spe
                            )).await;
                        }
                    }

                    // Show bench (non-active, alive)
                    let bench: alloc::vec::Vec<&MonBattleData> =
                        player.mons.iter().filter(|m| !m.active && m.hp > 0).collect();
                    if !bench.is_empty() {
                        let bench_str: alloc::vec::Vec<String> = bench
                            .iter()
                            .map(|m| alloc::format!("{} {}/{}", m.summary.name, m.hp, m.max_hp))
                            .collect();
                        self.writef(&alloc::format!(
                            "  bench: {}\r\n", bench_str.join("  ")
                        )).await;
                    }
                }

                self.write("──────────────────────────────────\r\n").await;

                let mut parts = alloc::vec::Vec::new();
                for (slot_idx, mon_req) in turn.active.iter().enumerate() {
                    let n = mon_req.moves.len().min(4);

                    // Find this mon's data in allies for its name.
                    let mon_name = turn.allies.iter()
                        .find(|p| p.id == player_id)
                        .and_then(|p| p.mons.iter().find(|m| m.player_team_position == mon_req.team_position))
                        .map(|m| m.summary.name.as_str())
                        .unwrap_or("?");

                    let label = Self::player_label(player_id);
                    self.writef(&alloc::format!(
                        "{} ({}) — pick a move for {}\r\n", label, player_id, mon_name
                    )).await;

                    if n == 0 {
                        self.write("  (no moves available — passing)\r\n").await;
                        parts.push(String::from("pass"));
                        continue;
                    }

                    for i in 0..n {
                        let m = &mon_req.moves[i];
                        let flag = if m.disabled || m.pp == 0 { " ✗" } else { "" };
                        self.writef(&alloc::format!(
                            "  [{}] {:20}  PP {}/{}{}\r\n",
                            i + 1, m.name, m.pp, m.max_pp, flag
                        )).await;
                    }

                    let _ = slot_idx;
                    loop {
                        self.write_move_prompt(n).await;
                        let line = self.read_line().await;
                        if let Ok(btn) = line.trim().parse::<usize>() {
                            if btn >= 1 && btn <= n {
                                let slot = btn - 1;
                                let m = &mon_req.moves[slot];
                                if m.disabled || m.pp == 0 {
                                    self.write("  That move cannot be used.\r\n").await;
                                    continue;
                                }
                                self.writef(&alloc::format!(
                                    "  → {}\r\n", m.name
                                )).await;
                                parts.push(format_move_choice(slot));
                                break;
                            }
                        }
                        self.writef(&alloc::format!(
                            "  Enter a number 1–{}.\r\n", n
                        )).await;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::Switch(sw) => {
                self.write("\r\n══ Switch required ══\r\n").await;
                let mut parts = alloc::vec::Vec::new();
                for _ in &sw.needs_switch {
                    loop {
                        self.write("Pick party slot to send in [1-6]: ").await;
                        let line = self.read_line().await;
                        if let Ok(n) = line.trim().parse::<usize>() {
                            if n >= 1 && n <= 6 {
                                self.writef(&alloc::format!("  → slot {}\r\n", n)).await;
                                parts.push(format_switch_choice(n - 1));
                                break;
                            }
                        }
                        self.write("  Enter a number 1–6.\r\n").await;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::TeamPreview(_) => {
                self.write("[team preview — using random order]\r\n").await;
                String::from("random")
            }
            Request::LearnMove(_) => {
                self.write("[learn move — passing]\r\n").await;
                String::from("pass")
            }
        }
    }

    // ── I/O primitives ────────────────────────────────────────────────────────

    async fn write(&mut self, s: &str) {
        self.writef(s).await
    }

    async fn writef(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let mut start = 0;
        while start < bytes.len() {
            let end = (start + 63).min(bytes.len());
            let _ = self.sender.write_packet(&bytes[start..end]).await;
            start = end;
        }
    }

    async fn writeln(&mut self, s: &str) {
        self.writef(s).await;
        self.write("\r\n").await;
    }

    /// Same line discipline as `usb_loopback`: `\r` / `\n` end a line; `\n` after `\r` is absorbed.
    fn take_completed_line(&mut self) -> Option<String> {
        let line = String::from(self.partial.trim());
        self.partial.clear();
        if !line.is_empty() {
            self.last_typed_line = Some(line.clone());
            return Some(line);
        }
        if let Some(last) = self.last_typed_line.clone() {
            return Some(last);
        }
        None
    }

    /// Read a line from USB with local echo, backspace, CRLF echo, and RTT log (same as loopback).
    async fn read_line(&mut self) -> String {
        self.receiver.wait_connection().await;
        let mut buf = [0u8; 64];
        let mut skip_next_lf = false;
        loop {
            match self.receiver.read_packet(&mut buf).await {
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
                                log_usb_rx_line_str_to_rtt(self.partial.as_str());
                                write_crlf(&mut self.sender).await;
                                skip_next_lf = true;
                                if let Some(line) = self.take_completed_line() {
                                    return line;
                                }
                            }
                            b'\n' => {
                                log_usb_rx_line_str_to_rtt(self.partial.as_str());
                                write_crlf(&mut self.sender).await;
                                if let Some(line) = self.take_completed_line() {
                                    return line;
                                }
                            }
                            b'\x08' | b'\x7f' => {
                                if self.partial.pop().is_some() {
                                    let _ = self.sender.write_packet(b"\x08 \x08").await;
                                }
                            }
                            b if b >= 0x20 => {
                                self.partial.push(b as char);
                                let _ = self.sender.write_packet(&[b]).await;
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => {
                    self.partial.clear();
                    skip_next_lf = false;
                    self.receiver.wait_connection().await;
                }
            }
        }
    }

    async fn write_move_prompt(&mut self, n: usize) {
        self.writef(&alloc::format!("Move [1-{}]: ", n)).await;
    }

    fn player_label(id: &str) -> &'static str {
        match id {
            "p1" => "Red",
            "p2" => "Blue",
            _ => "?",
        }
    }
}

impl InputSource for UsbBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        UsbBattleInput::run(self, bus).await
    }
}
