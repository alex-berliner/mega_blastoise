extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request};
use cortex_m::peripheral::SCB;
use crate::battle_effects::ANIM_ENABLED;
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_move_choice, format_prompt, format_switch_choice, join_choice_parts, ActivePrompt,
    InputBus, InputSource, PlayerAction,
};
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

use crate::pico_battle_input::PicoBattleInput;

pub enum LobbyUsbCmd {
    ReadyP1,
    ReadyP2,
    ReadyBoth,
    /// `:ready ai` — start the real battle with P2 as AI.
    VsAi,
    /// `:demo` — restart the AI vs AI demo loop.
    Demo,
    StopDemo,
    Unknown,
}

pub struct UsbBattleInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
    partial: String,
    /// Last non-empty line submitted at a prompt; Enter on an empty line resends it.
    last_typed_line: Option<String>,
    /// Player data from the most recent Turn prompt, reused when a Switch prompt follows.
    last_player_data: Option<PlayerBattleData>,
    /// Which players are AI-controlled this battle (reset each lobby).
    ai_players: [bool; 2],
    /// RNG state for AI choices.
    ai_rng: u64,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self {
            sender,
            receiver,
            partial: String::new(),
            last_typed_line: None,
            last_player_data: None,
            ai_players: [false, false],
            ai_rng: 0x9e3779b97f4a7c15,
        }
    }

    /// Configure which players are AI for the upcoming battle.
    pub fn set_ai_players(&mut self, ai: [bool; 2], seed: u64) {
        self.ai_players = ai;
        self.ai_rng = seed ^ 0xbad_c0ffee_dead;
    }

    fn ai_next_u64(&mut self) -> u64 {
        self.ai_rng = self.ai_rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.ai_rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn make_ai_choice(&mut self, request: &Request, player_data: Option<&PlayerBattleData>) -> String {
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 { parts.push(String::from("pass")); continue; }
                    let slot = (self.ai_next_u64() as usize) % n;
                    parts.push(format_move_choice(slot));
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let valid: Vec<usize> = match player_data {
                    Some(pd) => pd.mons.iter().enumerate()
                        .filter(|(_, m)| !m.active && m.hp > 0)
                        .map(|(i, _)| i)
                        .collect(),
                    None => alloc::vec![1, 2],
                };
                let mut parts = Vec::new();
                for _ in &sw.needs_switch {
                    let idx = if valid.is_empty() { 0 }
                              else { valid[(self.ai_next_u64() as usize) % valid.len()] };
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }

    pub async fn run(&mut self, bus: &InputBus) {
        self.run_inner(bus, None).await;
    }

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
            let ActivePrompt { player_id, request, player_data, .. } = prompt;
            defmt::debug!("usb: prompt received for {}", player_id.as_str());
            let player_idx = if player_id.as_str() == "p1" { 0 } else { 1 };
            let choice = if self.ai_players[player_idx] {
                self.write_dbg(&alloc::format!("[AI] auto-choosing for {}", player_id.as_str())).await;
                self.make_ai_choice(&request, player_data.as_ref())
            } else {
                let btns = buttons.as_mut().map(|b| &mut **b);
                self.handle(&player_id, &request, player_data, btns).await
            };
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
                self.last_player_data = player_data.clone();
                self.write("\r\n").await;
                self.write_multiline(&format_prompt(player_id, request, player_data.as_ref())).await;

                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        self.write_ok("No moves available — passing automatically").await;
                        parts.push(String::from("pass"));
                        continue;
                    }
                    if mon_req.locked_into_move {
                        self.write_ok("Locked into recharge — submitting automatically").await;
                        parts.push(format_move_choice(0));
                        continue;
                    }

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
                                    btns.wait_move(player_id, n, is_usable),
                                )
                                .await
                                {
                                    Either::First(line) => {
                                        let trimmed = line.trim();
                                        if let Some(rest) = trimmed.strip_prefix('s').or_else(|| trimmed.strip_prefix('S')) {
                                            match rest.parse::<usize>() {
                                                Ok(slot_n) if slot_n >= 1 && slot_n <= 6 => {
                                                    if mon_req.trapped {
                                                        self.write_err("Rejected — Pokémon is trapped, cannot switch").await;
                                                        continue 'move_input;
                                                    }
                                                    self.write_ok(&alloc::format!("Switching in slot {}", slot_n)).await;
                                                    parts.push(format_switch_choice(slot_n - 1));
                                                    break 'move_input;
                                                }
                                                _ => {
                                                    self.write_err(&alloc::format!(
                                                        "Rejected — \"{}\" not valid (move: 1-{}, switch: s1-s6)", trimmed, n
                                                    )).await;
                                                    continue 'move_input;
                                                }
                                            }
                                        }
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
                                                    "Rejected — \"{}\" not valid (move: 1-{}, switch: s1-s6)", trimmed, n
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
                                if let Some(rest) = trimmed.strip_prefix('s').or_else(|| trimmed.strip_prefix('S')) {
                                    match rest.parse::<usize>() {
                                        Ok(slot_n) if slot_n >= 1 && slot_n <= 6 => {
                                            if mon_req.trapped {
                                                self.write_err("Rejected — Pokémon is trapped, cannot switch").await;
                                                continue 'move_input;
                                            }
                                            self.write_ok(&alloc::format!("Switching in slot {}", slot_n)).await;
                                            parts.push(format_switch_choice(slot_n - 1));
                                            break 'move_input;
                                        }
                                        _ => {
                                            self.write_err(&alloc::format!(
                                                "Rejected — \"{}\" not valid (move: 1-{}, switch: s1-s6)", trimmed, n
                                            )).await;
                                            continue 'move_input;
                                        }
                                    }
                                }
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
                                            "Rejected — \"{}\" not valid (move: 1-{}, switch: s1-s6)",
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
                self.write_multiline(&format_prompt(player_id, request, self.last_player_data.as_ref())).await;

                let mut parts = Vec::new();
                for (i, &fainted_slot) in sw.needs_switch.iter().enumerate() {
                    self.writef(&alloc::format!(
                        "Replacement {} of {} (for team slot {}):\r\n",
                        i + 1, sw.needs_switch.len(), fainted_slot
                    )).await;
                    'switch_input: loop {
                        self.write("Send in party slot [1-6]: ").await;

                        let team_idx = match buttons.as_mut() {
                            Some(btns) => {
                                match select(self.read_line(), btns.wait_switch(player_id)).await {
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

    // ── I/O primitives ────────────────────────────────────────────────────────

    async fn write(&mut self, s: &str) {
        self.writef(s).await
    }

    /// Write a `\n`-delimited string with `\r\n` line endings.
    async fn write_multiline(&mut self, s: &str) {
        for line in s.split('\n') {
            if !line.is_empty() {
                self.writef(line).await;
                self.write("\r\n").await;
            }
        }
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

    /// Read a line from USB with backspace, CRLF, and RTT mirror.
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
                                    if let Some(msg) = handle_meta_cmd(line.trim()) {
                                        self.write(msg).await;
                                        self.write("\r\n").await;
                                        continue;
                                    }
                                    return line;
                                }
                            }
                            b'\n' => {
                                log_usb_rx_line_str_to_rtt(self.partial.as_str());
                                write_crlf(&mut self.sender).await;
                                if let Some(line) = self.take_completed_line() {
                                    if let Some(msg) = handle_meta_cmd(line.trim()) {
                                        self.write(msg).await;
                                        self.write("\r\n").await;
                                        continue;
                                    }
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

    // ── Lobby interface ───────────────────────────────────────────────────────

    /// Write a lobby status/info line (adds \r\n).
    pub async fn write_lobby_line(&mut self, msg: &str) {
        self.writeln(msg).await;
    }

    /// Write the current ready state to USB.
    pub async fn write_lobby_ready_status(&mut self, p1_ready: bool, p2_ready: bool) {
        let p1 = if p1_ready { "READY" } else { "     " };
        let p2 = if p2_ready { "READY" } else { "     " };
        self.writeln(&alloc::format!(
            "P1: [{}]   P2: [{}]   (:ready p1 / :ready p2 / :ready)",
            p1, p2
        )).await;
    }

    /// Read a lobby command from USB. Returns as soon as a line is submitted.
    pub async fn read_lobby_cmd(&mut self) -> LobbyUsbCmd {
        let line = self.read_line().await;
        match line.trim() {
            ":ready" => LobbyUsbCmd::ReadyBoth,
            ":ready p1" => LobbyUsbCmd::ReadyP1,
            ":ready p2" => LobbyUsbCmd::ReadyP2,
            ":ready ai" | ":ready both ai" => LobbyUsbCmd::VsAi,
            ":demo" => LobbyUsbCmd::Demo,
            ":s" | ":stop" => LobbyUsbCmd::StopDemo,
            _ => LobbyUsbCmd::Unknown,
        }
    }

}

/// Handle meta-commands that are valid at any time (lobby or battle).
/// Returns Some(ack message) if handled, None to pass the line through as normal input.
/// Side-effects (reset, flag toggle) happen before returning.
fn handle_meta_cmd(line: &str) -> Option<&'static str> {
    match line {
        ":reset" => { SCB::sys_reset(); }
        ":anim off" => { ANIM_ENABLED.store(false, core::sync::atomic::Ordering::Relaxed); return Some("[anim] animations OFF"); }
        ":anim on"  => { ANIM_ENABLED.store(true,  core::sync::atomic::Ordering::Relaxed); return Some("[anim] animations ON"); }
        _ => return None,
    }
    None
}

impl InputSource for UsbBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        UsbBattleInput::run(self, bus).await
    }
}

// ── UsbButtonInput ────────────────────────────────────────────────────────────

/// Minimal [`ButtonSource`] over USB CDC serial.
///
/// Reads a single digit (1–n) from the serial port and returns the 0-based slot.
/// No display, no move lists — the OLED (or host terminal) shows what to press.
/// Pairs with [`mega_blastoise_core::ButtonController`] which handles all
/// battle-protocol logic.
pub struct UsbButtonInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
}

impl<'d> UsbButtonInput<'d> {
    pub fn new(
        sender: Sender<'d, Driver<'d, USB>>,
        receiver: Receiver<'d, Driver<'d, USB>>,
    ) -> Self {
        Self { sender, receiver }
    }

    /// Read bytes from USB until we get a digit in 1..=max_btn; return 0-based index.
    async fn read_button(&mut self, max_btn: usize) -> usize {
        let mut buf = [0u8; 64];
        loop {
            self.receiver.wait_connection().await;
            match self.receiver.read_packet(&mut buf).await {
                Ok(n) => {
                    for &b in &buf[..n] {
                        if b >= b'1' && b <= b'0' + max_btn as u8 {
                            return (b - b'1') as usize;
                        }
                    }
                }
                Err(_) => {
                    self.receiver.wait_connection().await;
                }
            }
        }
    }
}

impl mega_blastoise_core::ButtonSource for UsbButtonInput<'_> {
    async fn wait_action(&mut self, _player_id: &str, n_moves: usize) -> PlayerAction {
        let mut saw_s = false;
        let mut buf = [0u8; 64];
        loop {
            self.receiver.wait_connection().await;
            match self.receiver.read_packet(&mut buf).await {
                Ok(n) => {
                    for &b in &buf[..n] {
                        if saw_s {
                            if b >= b'1' && b <= b'6' {
                                return PlayerAction::Switch((b - b'1') as usize);
                            }
                            saw_s = false;
                        }
                        if b == b's' || b == b'S' {
                            saw_s = true;
                        } else if b >= b'1' && b <= b'0' + n_moves as u8 {
                            return PlayerAction::Move((b - b'1') as usize);
                        }
                    }
                }
                Err(_) => {
                    self.receiver.wait_connection().await;
                }
            }
        }
    }

    async fn wait_switch(&mut self, _player_id: &str) -> usize {
        self.read_button(6).await
    }
}
