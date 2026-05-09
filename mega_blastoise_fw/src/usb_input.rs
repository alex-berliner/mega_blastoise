extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request};
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_move_choice, format_prompt, format_switch_choice, join_choice_parts, ActivePrompt,
    InputBus, InputSource, PlayerAction,
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
    /// True until we drain echo garbage once before the first user input read.
    needs_drain: bool,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self {
            sender,
            receiver,
            partial: String::new(),
            last_typed_line: None,
            last_player_data: None,
            needs_drain: true,
        }
    }

    pub async fn run(&mut self, bus: &InputBus) {
        self.run_inner(bus, None).await;
    }

    /// Inner event/prompt loop.  Pass `Some(buttons)` to race USB input against physical
    /// button presses; `None` for USB-only operation.
    /// Discard any USB input that arrived during the ECHO window when the host
    /// first opened the CDC port.  The tty line discipline echoes received bytes
    /// back to the device until the host's `tcsetattr` disables ECHO; those bytes
    /// can arrive in the firmware's RX buffer before the first prompt is shown.
    async fn drain_usb_input(&mut self) {
        let mut buf = [0u8; 64];
        loop {
            match select(
                self.receiver.read_packet(&mut buf),
                Timer::after(Duration::from_millis(50)),
            )
            .await
            {
                Either::First(Ok(_)) => {}
                Either::First(Err(_)) => {}
                Either::Second(_) => break,
            }
        }
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
                self.last_player_data = player_data.clone();
                self.write("\r\n").await;
                self.write_multiline(&format_prompt(player_id, request, player_data.as_ref())).await;

                // Best-effort drain on first turn: catches any echo bytes that arrived
                // early (during the host's brief ECHO-on window at _open_serial).
                // Late-arriving echo is handled by strip_echo_prefix in the parser.
                if self.needs_drain {
                    self.needs_drain = false;
                    self.drain_usb_input().await;
                }

                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        self.write_ok("No moves available — passing automatically").await;
                        parts.push(String::from("pass"));
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
                                        let trimmed = Self::recover_user_input(line.trim());
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
                                let trimmed = Self::recover_user_input(line.trim());
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

    async fn write_move_prompt(&mut self, n: usize) {
        self.writef(&alloc::format!("Move [1-{}]: ", n)).await;
    }

    /// Recover the user's command from a line that may have echo garbage prepended.
    ///
    /// The host tty ECHO window (os.open → tcsetattr) can cause firmware output to
    /// be echoed back and merged with the user's input in one line, e.g.
    /// "[EVT] Red sent os2" = echo("[EVT] Red sent o") + user("s2").
    ///
    /// User commands are always 1–2 chars: a digit (move) or "s/S" + digit (switch).
    /// Recovery strategy:
    ///  1. Strip any known output prefix ([EVT], [OK], etc.).
    ///  2. If the result is still >3 chars (contaminated), extract from the tail.
    fn recover_user_input(s: &str) -> &str {
        // Step 1: strip output prefix.
        const PREFIXES: &[&str] = &["[EVT] ", "[OK]  ", "[!!]  ", "[>>]  "];
        let s = {
            let mut out = s;
            for p in PREFIXES {
                if let Some(rest) = s.strip_prefix(p) {
                    out = rest.trim_start();
                    break;
                }
            }
            out
        };

        // Step 2: if still long, try to extract from the tail (user input appends at end).
        if s.len() > 3 {
            let b = s.as_bytes();
            let n = b.len();
            // "s[1-6]" or "S[1-6]" suffix.
            if n >= 2 && (b[n - 2] == b's' || b[n - 2] == b'S') && b[n - 1].is_ascii_digit() {
                return &s[n - 2..];
            }
            // Single digit suffix.
            if b[n - 1].is_ascii_digit() {
                return &s[n - 1..];
            }
        }
        s
    }

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
