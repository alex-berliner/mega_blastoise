extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use gen1_battle::{PlayerBattleData, Request};
use cortex_m::peripheral::SCB;
use crate::battle_effects::ANIM_ENABLED;
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_lobby_status, format_move_choice, format_prompt, format_switch_choice, join_choice_parts,
    parse_lobby_cmd, parse_switch_line, parse_turn_line,
    ActivePrompt, InputBus, InputSource, RandomAi, TurnChoice, TEAM_SEED_SALT,
};
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

use crate::pico_battle_input::PicoBattleInput;
#[cfg(feature = "oled")]
use crate::subsystems::oled::read_shadow_fb;

pub use mega_blastoise_core::LobbyCmd as LobbyUsbCmd;

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
    /// AI choice engine for AI-controlled players.
    ai: RandomAi,
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
            ai: RandomAi::new(TEAM_SEED_SALT),
        }
    }

    /// Configure which players are AI for the upcoming battle.
    pub fn set_ai_players(&mut self, ai: [bool; 2], seed: u64) {
        self.ai_players = ai;
        self.ai = RandomAi::new(seed ^ 0xbad_c0ffee_dead);
        self.last_typed_line = None;
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
                self.ai.make_choice(&request, player_data.as_ref())
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

                        // Get raw input: button press wins immediately; USB line goes to parse.
                        let choice = match buttons.as_mut() {
                            Some(btns) => {
                                let is_usable = |i: usize| {
                                    !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0
                                };
                                match select(self.read_line(), btns.wait_move(player_id, n, is_usable)).await {
                                    Either::First(line) => {
                                        match parse_turn_line(line.trim(), n) {
                                            Ok(c) => c,
                                            Err(msg) => {
                                                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                                                continue 'move_input;
                                            }
                                        }
                                    }
                                    Either::Second(slot) => {
                                        self.partial.clear();
                                        self.write_ok(&alloc::format!(
                                            "Button — {} (slot {})", mon_req.moves[slot].name, slot
                                        )).await;
                                        parts.push(format_move_choice(slot));
                                        break 'move_input;
                                    }
                                }
                            }
                            None => {
                                let line = self.read_line().await;
                                match parse_turn_line(line.trim(), n) {
                                    Ok(c) => c,
                                    Err(msg) => {
                                        self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                                        continue 'move_input;
                                    }
                                }
                            }
                        };

                        // Common validation for USB-parsed choices.
                        match choice {
                            TurnChoice::Move(slot) => {
                                let m = &mon_req.moves[slot];
                                if m.disabled {
                                    self.write_err(&alloc::format!("Rejected — {} is disabled", m.name)).await;
                                    continue 'move_input;
                                }
                                if m.pp == 0 {
                                    self.write_err(&alloc::format!("Rejected — {} has no PP", m.name)).await;
                                    continue 'move_input;
                                }
                                self.write_ok(&alloc::format!("Accepted — {} (slot {})", m.name, slot)).await;
                                parts.push(format_move_choice(slot));
                            }
                            TurnChoice::Switch(idx) => {
                                if mon_req.trapped {
                                    self.write_err("Rejected — Pokémon is trapped, cannot switch").await;
                                    continue 'move_input;
                                }
                                self.write_ok(&alloc::format!("Switching in slot {}", idx + 1)).await;
                                parts.push(format_switch_choice(idx));
                            }
                        }
                        break 'move_input;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::Switch(sw) => {
                // Prefer the fresh player_data attached to this Switch request (it reflects
                // the faint that triggered the switch, e.g. Goldeen at 0 HP).  Fall back to
                // last_player_data only if the engine sent None.
                if player_data.is_some() {
                    self.last_player_data = player_data.clone();
                }
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
                                        match parse_switch_line(line.trim()) {
                                            Ok(idx) => idx,
                                            Err(msg) => {
                                                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                                                continue 'switch_input;
                                            }
                                        }
                                    }
                                    Either::Second(idx) => {
                                        self.partial.clear();
                                        self.write_ok(&alloc::format!("Button — switching in slot {}", idx + 1)).await;
                                        parts.push(format_switch_choice(idx));
                                        break 'switch_input;
                                    }
                                }
                            }
                            None => {
                                let line = self.read_line().await;
                                match parse_switch_line(line.trim()) {
                                    Ok(idx) => idx,
                                    Err(msg) => {
                                        self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                                        continue 'switch_input;
                                    }
                                }
                            }
                        };

                        self.write_ok(&alloc::format!("Accepted — switching in slot {}", team_idx + 1)).await;
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
                                    if let Some(p) = oled_dump_player(line.trim()) {
                                        self.write_oled_dump(p).await;
                                        continue;
                                    }
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
                                    if let Some(p) = oled_dump_player(line.trim()) {
                                        self.write_oled_dump(p).await;
                                        continue;
                                    }
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

    /// Dump one OLED framebuffer as ASCII art (half-block chars) over USB.
    async fn write_oled_dump(&mut self, player: u8) {
        #[cfg(feature = "oled")]
        {
            let fb = read_shadow_fb(player);
            self.writeln(&alloc::format!("[P{} OLED 128×64]", player)).await;
            for row in 0..32usize {
                let mut line = alloc::string::String::with_capacity(128 * 3);
                for col in 0..128usize {
                    let top    = (fb[row * 2    ][col >> 3] >> (7 - (col & 7))) & 1 == 1;
                    let bottom = (fb[row * 2 + 1][col >> 3] >> (7 - (col & 7))) & 1 == 1;
                    line.push_str(match (top, bottom) {
                        (false, false) => " ",
                        (true,  false) => "▀",
                        (false, true)  => "▄",
                        (true,  true)  => "█",
                    });
                }
                self.writeln(&line).await;
            }
            self.writeln("[end]").await;
        }
        #[cfg(not(feature = "oled"))]
        self.writeln("[oled feature not enabled]").await;
    }

    // ── Lobby interface ───────────────────────────────────────────────────────

    /// Write a lobby status/info line (adds \r\n).
    pub async fn write_lobby_line(&mut self, msg: &str) {
        self.writeln(msg).await;
    }

    /// Write the current ready state to USB.
    pub async fn write_lobby_ready_status(&mut self, p1_ready: bool, p2_ready: bool) {
        self.writeln(&format_lobby_status(p1_ready, p2_ready)).await;
    }

    /// Read a lobby command from USB. Returns as soon as a line is submitted.
    pub async fn read_lobby_cmd(&mut self) -> LobbyUsbCmd {
        // Discard any partial input accumulated before this call (e.g. stray
        // chars echoed between countdown steps) so they don't corrupt the command.
        if !self.partial.is_empty() {
            self.write("\r\033[K").await;
            self.partial.clear();
        }
        let line = self.read_line().await;
        parse_lobby_cmd(line.trim())
    }

}

/// If `line` is an `:oled` dump command, return the player number to dump (1 or 2).
/// `:oled` and `:oled p1` → 1; `:oled p2` → 2.
fn oled_dump_player(line: &str) -> Option<u8> {
    match line {
        ":oled" | ":oled p1" | ":oled 1" => Some(1),
        ":oled p2" | ":oled 2"           => Some(2),
        _ => None,
    }
}

/// Handle meta-commands that are valid at any time (lobby or battle).
/// Returns Some(ack message) if handled, None to pass the line through as normal input.
/// Side-effects (reset, flag toggle) happen before returning.
fn handle_meta_cmd(line: &str) -> Option<&'static str> {
    match line {
        ":reset"    => SCB::sys_reset(),
        ":anim off" => { ANIM_ENABLED.store(false, core::sync::atomic::Ordering::Relaxed); Some("[anim] animations OFF") }
        ":anim on"  => { ANIM_ENABLED.store(true,  core::sync::atomic::Ordering::Relaxed); Some("[anim] animations ON") }
        _           => None,
    }
}

impl InputSource for UsbBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        UsbBattleInput::run(self, bus).await
    }
}

