extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{MonBattleData, PlayerBattleData, Request};
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::USB;
use embassy_time::{Duration, Timer};
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus,
    InputSource,
};
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

use crate::pico_battle_input::PicoBattleInput;

pub struct UsbBattleInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
    partial: String,
    /// Last non-empty line submitted at a prompt; Enter on an empty line resends it.
    last_typed_line: Option<String>,
    /// Player data from the most recent Turn prompt, reused when a Switch prompt follows.
    last_player_data: Option<PlayerBattleData>,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self {
            sender,
            receiver,
            partial: String::new(),
            last_typed_line: None,
            last_player_data: None,
        }
    }

    pub async fn run(&mut self, bus: &InputBus) {
        self.run_inner(bus, None).await;
    }

    /// Inner event/prompt loop.  Pass `Some(buttons)` to race USB input against physical
    /// button presses; `None` for USB-only operation.
    pub(crate) async fn run_inner(
        &mut self,
        bus: &InputBus,
        mut buttons: Option<&mut PicoBattleInput<'_>>,
    ) {
        self.write("=== Battle CLI ready — waiting for first prompt ===\r\n").await;
        loop {
            let prompt = loop {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => {
                        while let Ok(line) = bus.log.try_receive() {
                            self.write_event(&line).await;
                        }
                        break p;
                    }
                    Either::Second(line) => {
                        self.write_event(&line).await;
                    }
                }
            };
            let ActivePrompt { player_id, request, player_data } = prompt;
            defmt::debug!("usb: prompt received for {}", player_id.as_str());
            let btns = buttons.as_mut().map(|b| &mut **b);
            let choice = self.handle(&player_id, &request, player_data, btns).await;
            self.write_dbg(&alloc::format!("Submitting to engine: \"{}\"", choice)).await;
            bus.choices.send(choice).await;

            while let Ok(line) = bus.log.try_receive() {
                self.write_event(&line).await;
            }
        }
    }

    async fn handle(
        &mut self,
        player_id: &str,
        request: &Request,
        player_data: Option<PlayerBattleData>,
        mut buttons: Option<&mut PicoBattleInput<'_>>,
    ) -> String {
        match request {
            Request::Turn(turn) => {
                // Cache for Switch prompts that follow this Turn prompt.
                self.last_player_data = player_data.clone();

                self.write("\r\n").await;
                self.write("══════════════════════════════════\r\n").await;

                if let Some(pd) = &player_data {
                    self.write_player_state(pd).await;
                }

                self.write("──────────────────────────────────\r\n").await;

                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);

                    let mon_name = player_data.as_ref()
                        .and_then(|pd| pd.mons.iter().find(|m| m.player_team_position == mon_req.team_position))
                        .map(|m| m.summary.name.as_str())
                        .unwrap_or("?");

                    let label = Self::player_label(player_id);
                    self.writef(&alloc::format!(
                        "{} ({}) — choose move for {}\r\n", label, player_id, mon_name
                    )).await;

                    if n == 0 {
                        self.write_ok("No moves available — passing automatically").await;
                        parts.push(String::from("pass"));
                        continue;
                    }

                    for i in 0..n {
                        let m = &mon_req.moves[i];
                        let usable = !m.disabled && m.pp > 0;
                        let state = if m.disabled { " [DISABLED]" } else if m.pp == 0 { " [NO PP]" } else { "" };
                        self.writef(&alloc::format!(
                            "  [{}] {:<20}  PP {}/{}{}",
                            i + 1, m.name, m.pp, m.max_pp, state
                        )).await;
                        if usable {
                            self.write("  <-- available\r\n").await;
                        } else {
                            self.write("\r\n").await;
                        }
                    }

                    self.drain_rx().await;
                    'move_input: loop {
                        self.write_move_prompt(n).await;

                        // Accept input from USB serial or physical button (first wins).
                        let slot = match buttons.as_mut() {
                            Some(btns) => {
                                let is_usable = |i: usize| {
                                    !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0
                                };
                                match select(
                                    self.read_line(),
                                    btns.wait_move(n, is_usable),
                                )
                                .await
                                {
                                    Either::First(line) => {
                                        let trimmed = line.trim();
                                        match trimmed.parse::<usize>() {
                                            Ok(btn) if btn >= 1 && btn <= n => {
                                                let slot = btn - 1;
                                                let m = &mon_req.moves[slot];
                                                if m.disabled {
                                                    self.write_err(&alloc::format!(
                                                        "Rejected — {} is disabled", m.name
                                                    ))
                                                    .await;
                                                    continue 'move_input;
                                                }
                                                if m.pp == 0 {
                                                    self.write_err(&alloc::format!(
                                                        "Rejected — {} has no PP", m.name
                                                    ))
                                                    .await;
                                                    continue 'move_input;
                                                }
                                                self.write_ok(&alloc::format!(
                                                    "Accepted — {} (slot {})", m.name, slot
                                                ))
                                                .await;
                                                slot
                                            }
                                            Ok(btn) => {
                                                self.write_err(&alloc::format!(
                                                    "Rejected — {} out of range, enter 1-{}", btn, n
                                                ))
                                                .await;
                                                continue 'move_input;
                                            }
                                            Err(_) => {
                                                self.write_err(&alloc::format!(
                                                    "Rejected — \"{}\" is not a number", trimmed
                                                ))
                                                .await;
                                                continue 'move_input;
                                            }
                                        }
                                    }
                                    Either::Second(slot) => {
                                        // Physical button press; clean up any partial USB input.
                                        self.partial.clear();
                                        self.write_ok(&alloc::format!(
                                            "Button — {} (slot {})",
                                            mon_req.moves[slot].name, slot
                                        ))
                                        .await;
                                        slot
                                    }
                                }
                            }
                            None => {
                                // USB-only path.
                                let line = self.read_line().await;
                                let trimmed = line.trim();
                                match trimmed.parse::<usize>() {
                                    Ok(btn) if btn >= 1 && btn <= n => {
                                        let slot = btn - 1;
                                        let m = &mon_req.moves[slot];
                                        if m.disabled {
                                            self.write_err(&alloc::format!(
                                                "Rejected — {} is disabled, pick another", m.name
                                            ))
                                            .await;
                                            continue 'move_input;
                                        }
                                        if m.pp == 0 {
                                            self.write_err(&alloc::format!(
                                                "Rejected — {} has no PP remaining, pick another",
                                                m.name
                                            ))
                                            .await;
                                            continue 'move_input;
                                        }
                                        self.write_ok(&alloc::format!(
                                            "Accepted — {} (slot {})", m.name, slot
                                        ))
                                        .await;
                                        slot
                                    }
                                    Ok(btn) => {
                                        self.write_err(&alloc::format!(
                                            "Rejected — {} is out of range, enter 1-{}", btn, n
                                        ))
                                        .await;
                                        continue 'move_input;
                                    }
                                    Err(_) => {
                                        self.write_err(&alloc::format!(
                                            "Rejected — \"{}\" is not a number, enter 1-{}",
                                            trimmed, n
                                        ))
                                        .await;
                                        continue 'move_input;
                                    }
                                }
                            }
                        };

                        parts.push(format_move_choice(slot));
                        break 'move_input;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::Switch(sw) => {
                self.write("\r\n").await;
                self.write("══════════════════════════════════\r\n").await;
                self.writef(&alloc::format!(
                    "SWITCH REQUIRED — {} slot(s) need a replacement\r\n",
                    sw.needs_switch.len()
                )).await;

                if let Some(player) = self.last_player_data.clone() {
                    self.write_bench_for_switch(&player).await;
                }
                self.write("──────────────────────────────────\r\n").await;

                let mut parts = Vec::new();
                for (i, &fainted_slot) in sw.needs_switch.iter().enumerate() {
                    self.writef(&alloc::format!(
                        "Replacement {} of {} (for team slot {}):\r\n",
                        i + 1, sw.needs_switch.len(), fainted_slot
                    )).await;
                    self.drain_rx().await;
                    'switch_input: loop {
                        self.write("Send in party slot [1-6]: ").await;

                        let team_idx = match buttons.as_mut() {
                            Some(btns) => {
                                match select(self.read_line(), btns.wait_switch()).await {
                                    Either::First(line) => {
                                        let trimmed = line.trim();
                                        match trimmed.parse::<usize>() {
                                            Ok(n) if n >= 1 && n <= 6 => {
                                                self.write_ok(&alloc::format!(
                                                    "Accepted — switching in slot {}", n
                                                ))
                                                .await;
                                                n - 1
                                            }
                                            Ok(n) => {
                                                self.write_err(&alloc::format!(
                                                    "Rejected — {} out of range, enter 1-6", n
                                                ))
                                                .await;
                                                continue 'switch_input;
                                            }
                                            Err(_) => {
                                                self.write_err(&alloc::format!(
                                                    "Rejected — \"{}\" is not a number", trimmed
                                                ))
                                                .await;
                                                continue 'switch_input;
                                            }
                                        }
                                    }
                                    Either::Second(idx) => {
                                        self.partial.clear();
                                        self.write_ok(&alloc::format!(
                                            "Button — switching in slot {}", idx + 1
                                        ))
                                        .await;
                                        idx
                                    }
                                }
                            }
                            None => {
                                let line = self.read_line().await;
                                let trimmed = line.trim();
                                match trimmed.parse::<usize>() {
                                    Ok(n) if n >= 1 && n <= 6 => {
                                        self.write_ok(&alloc::format!(
                                            "Accepted — switching in slot {}", n
                                        ))
                                        .await;
                                        n - 1
                                    }
                                    Ok(n) => {
                                        self.write_err(&alloc::format!(
                                            "Rejected — {} is out of range, enter 1-6", n
                                        ))
                                        .await;
                                        continue 'switch_input;
                                    }
                                    Err(_) => {
                                        self.write_err(&alloc::format!(
                                            "Rejected — \"{}\" is not a number, enter 1-6",
                                            trimmed
                                        ))
                                        .await;
                                        continue 'switch_input;
                                    }
                                }
                            }
                        };

                        parts.push(format_switch_choice(team_idx));
                        break 'switch_input;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::TeamPreview(_) => {
                self.write_dbg("Team preview — using random order").await;
                String::from("random")
            }
            Request::LearnMove(_) => {
                self.write_dbg("Learn move — passing").await;
                String::from("pass")
            }
        }
    }

    // ── Display helpers ───────────────────────────────────────────────────────

    async fn write_player_state(&mut self, player: &PlayerBattleData) {
        let label = Self::player_label(&player.id);
        let actives: Vec<&MonBattleData> = player.mons.iter().filter(|m| m.active).collect();
        for m in &actives {
            let status = m.status.as_deref().unwrap_or("ok");
            let item = m.item.as_deref().unwrap_or("none");
            let pct = if m.max_hp > 0 { m.hp * 100 / m.max_hp } else { 0 };
            self.writef(&alloc::format!(
                "{} — {} ({})  HP {}/{} ({}%)  status: {}  item: {}\r\n",
                label, m.summary.name, m.species, m.hp, m.max_hp, pct, status, item
            )).await;
            let b = &m.boosts;
            if b.atk != 0 || b.def != 0 || b.spa != 0 || b.spd != 0 || b.spe != 0 {
                self.writef(&alloc::format!(
                    "  boosts  atk:{:+}  def:{:+}  spa:{:+}  spd:{:+}  spe:{:+}\r\n",
                    b.atk, b.def, b.spa, b.spd, b.spe
                )).await;
            }
        }

        // Bench — alive and fainted separately.
        let bench_alive: Vec<&MonBattleData> =
            player.mons.iter().filter(|m| !m.active && m.hp > 0).collect();
        let bench_fainted: Vec<&MonBattleData> =
            player.mons.iter().filter(|m| !m.active && m.hp == 0).collect();
        if !bench_alive.is_empty() {
            let s: Vec<String> = bench_alive
                .iter()
                .map(|m| {
                    let pct = if m.max_hp > 0 { m.hp * 100 / m.max_hp } else { 0 };
                    alloc::format!("{} {}/{}({}%)", m.summary.name, m.hp, m.max_hp, pct)
                })
                .collect();
            self.writef(&alloc::format!("  bench: {}\r\n", s.join("  "))).await;
        }
        if !bench_fainted.is_empty() {
            let s: Vec<String> = bench_fainted
                .iter()
                .map(|m| alloc::format!("{} [fnt]", m.summary.name))
                .collect();
            self.writef(&alloc::format!("  fainted: {}\r\n", s.join("  "))).await;
        }
    }

    async fn write_bench_for_switch(&mut self, player: &PlayerBattleData) {
        let label = Self::player_label(&player.id);
        self.writef(&alloc::format!("  {} party:\r\n", label)).await;
        for (i, m) in player.mons.iter().enumerate() {
            let slot = i + 1;
            if m.active {
                self.writef(&alloc::format!(
                    "    [{}] {} — active (HP {}/{})\r\n",
                    slot, m.summary.name, m.hp, m.max_hp
                )).await;
            } else if m.hp == 0 {
                self.writef(&alloc::format!(
                    "    [{}] {} — fainted\r\n",
                    slot, m.summary.name
                )).await;
            } else {
                let pct = m.hp * 100 / m.max_hp.max(1);
                self.writef(&alloc::format!(
                    "    [{}] {} — HP {}/{} ({}%)  <-- available\r\n",
                    slot, m.summary.name, m.hp, m.max_hp, pct
                )).await;
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

    /// Battle event line from the engine (e.g. damage, faint, move used).
    async fn write_event(&mut self, s: &str) {
        self.write("[EVT] ").await;
        self.writeln(s).await;
    }

    /// Successful input acknowledgement — USB display + RTT debug mirror.
    async fn write_ok(&mut self, s: &str) {
        defmt::debug!("[OK]  {}", defmt::Display2Format(s));
        self.write("[OK]  ").await;
        self.writeln(s).await;
    }

    /// Input rejection with reason — USB display + RTT warn mirror.
    async fn write_err(&mut self, s: &str) {
        defmt::warn!("[!!]  {}", defmt::Display2Format(s));
        self.write("[!!]  ").await;
        self.writeln(s).await;
    }

    /// Debug / informational line — USB display + RTT debug mirror.
    async fn write_dbg(&mut self, s: &str) {
        defmt::debug!("[>>]  {}", defmt::Display2Format(s));
        self.write("[>>]  ").await;
        self.writeln(s).await;
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

    /// Read a line from USB with local echo, backspace, CRLF, and RTT mirror.
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
                                self.partial.pop();
                            }
                            b if b >= 0x20 => {
                                self.partial.push(b as char);
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

    /// Drain any bytes sitting in the USB RX FIFO.
    /// Called after displaying a prompt to discard host-echoed TX bytes before
    /// reading user input. Uses a short timeout so the drain ends once the
    /// echo burst (one USB round-trip, ~2 ms) has passed.
    async fn drain_rx(&mut self) {
        let mut buf = [0u8; 64];
        loop {
            match select(
                self.receiver.read_packet(&mut buf),
                Timer::after(Duration::from_millis(5)),
            )
            .await
            {
                Either::First(Ok(_)) => {}
                _ => break,
            }
        }
        self.partial.clear();
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
