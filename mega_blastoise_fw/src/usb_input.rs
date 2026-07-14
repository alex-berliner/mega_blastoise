extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use cortex_m::peripheral::SCB;
use crate::battle_effects::ANIM_ENABLED;
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_time::Instant;
use embassy_usb::class::cdc_acm::{Receiver, Sender};
use mega_blastoise_core::{
    format_lobby_status, parse_lobby_cmd, parse_team_spec, ActivePrompt, ChoiceCollector,
    CollectEffect, ControlMode, ControlsSelect, InputBus, InputSource, PlayerChoice, RandomAi,
    SlotOptions, BATTLE_HELP, COLLECT_TICK_MS, LOBBY_HELP, TEAM_SEED_SALT,
};
use gen1_battle::MonData;
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

use crate::pico_battle_input::{PadScan, PicoBattleInput};
#[cfg(feature = "oled")]
use crate::subsystems::oled::{read_shadow_fb, send as oled_send, wait_fb_change};

pub use mega_blastoise_core::LobbyCmd as LobbyUsbCmd;

pub struct UsbBattleInput<'d> {
    sender: Sender<'d, Driver<'d, USB>>,
    receiver: Receiver<'d, Driver<'d, USB>>,
    partial: String,
    /// Last non-empty line submitted at a prompt; Enter on an empty line resends it.
    last_typed_line: Option<String>,
    /// Which players are AI-controlled this battle (reset each lobby).
    ai_players: [bool; 2],
    /// Per-player control scheme for this battle (chosen at battle start).
    modes: [ControlMode; 2],
    /// AI choice engine for AI-controlled players.
    ai: RandomAi,
    /// Most recently parsed `:team` upload, consumed by the lobby on the next
    /// `LobbyEvent::TeamUpload`. `(player_index, team)`.
    pending_lobby_team: Option<(u8, alloc::vec::Vec<MonData>)>,
}

impl<'d> UsbBattleInput<'d> {
    pub fn new(sender: Sender<'d, Driver<'d, USB>>, receiver: Receiver<'d, Driver<'d, USB>>) -> Self {
        Self {
            sender,
            receiver,
            partial: String::new(),
            last_typed_line: None,
            ai_players: [false, false],
            modes: [ControlMode::Normal; 2],
            ai: RandomAi::new(TEAM_SEED_SALT),
            pending_lobby_team: None,
        }
    }

    /// Configure the control schemes chosen at battle start.
    pub fn set_modes(&mut self, modes: [ControlMode; 2]) {
        self.modes = modes;
    }

    /// The battle-start controls picker: buttons + the same typed grammar as
    /// battle input (`p1 concealed`, `p1 ok`, `:press pN <btn>` sims).
    pub async fn run_controls_select(
        &mut self,
        mut buttons: Option<&mut PicoBattleInput<'_>>,
        ai: [bool; 2],
    ) -> [ControlMode; 2] {
        let mut fx: Vec<CollectEffect> = Vec::new();
        let mut cs = ControlsSelect::new(ai, &mut fx);
        self.apply_effects(&mut fx).await;
        let mut scan = PadScan::default();
        loop {
            match buttons.as_mut() {
                Some(btns) => {
                    match select3(
                        self.read_line(),
                        btns.next_pad_event(&mut scan),
                        embassy_time::Timer::after_millis(COLLECT_TICK_MS),
                    )
                    .await
                    {
                        Either3::First(line) => cs.typed_line(line.trim(), &mut fx),
                        Either3::Second(ev) => cs.pad_event(ev, &mut fx),
                        Either3::Third(()) => {}
                    }
                }
                None => {
                    match select(
                        self.read_line(),
                        embassy_time::Timer::after_millis(COLLECT_TICK_MS),
                    )
                    .await
                    {
                        Either::First(line) => cs.typed_line(line.trim(), &mut fx),
                        Either::Second(()) => {}
                    }
                }
            }
            let done = cs.tick(Instant::now().as_millis());
            self.apply_effects(&mut fx).await;
            if done {
                break;
            }
        }
        cs.take_modes()
    }

    /// Take the most recently uploaded `:team` payload, if any.
    pub fn take_pending_team(&mut self) -> Option<(u8, alloc::vec::Vec<MonData>)> {
        self.pending_lobby_team.take()
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
            // ── Gather the whole prompt batch (1 or 2 players) before acting,
            //    so both boards can be driven simultaneously. ─────────────────
            let first = loop {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => {
                        while let Ok(line) = bus.log.try_receive() {
                            self.write_event(&line).await;
                        }
                        break p;
                    }
                    Either::Second(line) => self.write_event(&line).await,
                }
            };
            let batch_total = first.batch_total.max(1);
            let mut prompts: Vec<ActivePrompt> = Vec::with_capacity(batch_total);
            prompts.push(first);
            while prompts.len() < batch_total {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => prompts.push(p),
                    Either::Second(line) => self.write_event(&line).await,
                }
            }

            // ── Shared collection: ALL semantics live in core's
            //    ChoiceCollector, identical to the web client. This loop only
            //    does raw IO: typed lines, matrix pad events, the tick clock,
            //    and applying the collector's effects. ──────────────────────
            let mut batch: Vec<SlotOptions> = Vec::with_capacity(prompts.len());
            for p in &prompts {
                let mut slot = SlotOptions::from_prompt(p);
                let idx = if p.player_id.as_str() == "p1" { 0 } else { 1 };
                if self.ai_players[idx] {
                    slot.set_ai_choice(self.ai.make_choice(&p.request, p.player_data.as_ref()));
                } else if self.modes[idx] == ControlMode::Concealed {
                    // Fresh randomized layouts every combat turn.
                    slot.set_concealed(Instant::now().as_millis() ^ (idx as u64) << 33);
                }
                batch.push(slot);
            }
            let mut fx: Vec<CollectEffect> = Vec::new();
            let mut col = ChoiceCollector::new(batch, &mut fx);
            self.apply_effects(&mut fx).await;

            let mut scan = PadScan::default();
            scan.instant_switch = [
                self.modes[0] == ControlMode::Concealed && !self.ai_players[0],
                self.modes[1] == ControlMode::Concealed && !self.ai_players[1],
            ];
            loop {
                match buttons.as_mut() {
                    Some(btns) => {
                        match select3(
                            self.read_line(),
                            btns.next_pad_event(&mut scan),
                            embassy_time::Timer::after_millis(COLLECT_TICK_MS),
                        )
                        .await
                        {
                            Either3::First(line) => col.typed_line(line.trim(), Instant::now().as_millis(), &mut fx),
                            Either3::Second(ev) => {
                                col.pad_event(ev, Instant::now().as_millis(), &mut fx)
                            }
                            Either3::Third(()) => {}
                        }
                    }
                    None => {
                        match select(
                            self.read_line(),
                            embassy_time::Timer::after_millis(COLLECT_TICK_MS),
                        )
                        .await
                        {
                            Either::First(line) => col.typed_line(line.trim(), Instant::now().as_millis(), &mut fx),
                            Either::Second(()) => {}
                        }
                    }
                }
                let done = col.tick(Instant::now().as_millis(), &mut fx);
                self.apply_effects(&mut fx).await;
                if done {
                    break;
                }
            }

            // ── Submit choices, tagged by player (the runner routes by id). ──
            for (player_id, choice) in col.take_choices() {
                let choice = if choice.is_empty() { String::from("pass") } else { choice };
                self.write_dbg(&alloc::format!("Submitting to engine ({}): \"{}\"", player_id.as_str(), choice.as_str())).await;
                bus.choices.send(PlayerChoice { player_id, choice }).await;
            }

            while let Ok(line) = bus.log.try_receive() {
                self.write_event(&line).await;
            }
        }
    }

    /// Map the collector's effects onto USB output and the OLED channel.
    async fn apply_effects(&mut self, fx: &mut Vec<CollectEffect>) {
        for e in fx.drain(..) {
            match e {
                CollectEffect::Oled(_cmd) => {
                    #[cfg(feature = "oled")]
                    oled_send(_cmd);
                }
                CollectEffect::Ok(m) => self.write_ok(&m).await,
                CollectEffect::Err(m) => self.write_err(&m).await,
                CollectEffect::Dbg(m) => self.write_dbg(&m).await,
                CollectEffect::Text(t) => {
                    self.write("\r\n").await;
                    self.write_multiline(&t).await;
                }
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
        // No host terminal attached (DTR low) → drop the output. An undrained
        // CDC IN endpoint pends write_packet forever, which would wedge the
        // battle loop when the console disconnects mid-battle.
        if !self.sender.dtr() {
            return;
        }
        let bytes = s.as_bytes();
        let mut start = 0;
        while start < bytes.len() {
            let end = (start + 63).min(bytes.len());
            let write = self.sender.write_packet(&bytes[start..end]);
            // Timeout guards the DTR-high-but-not-reading case (frozen host
            // process): drop the rest of the line rather than stall the game.
            if embassy_time::with_timeout(embassy_time::Duration::from_millis(500), write)
                .await
                .is_err()
            {
                return;
            }
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
            // While waiting for input, also watch for OLED framebuffer changes
            // (`:oled auto on`) and print the half-block dump inline. The
            // select result is bound to a variable so the read_packet future
            // (which borrows self.receiver) is dropped before write_oled_dump
            // needs &mut self.
            #[cfg(feature = "oled")]
            let res = match select(self.receiver.read_packet(&mut buf), wait_fb_change()).await {
                Either::First(r) => r,
                Either::Second(mask) => {
                    if mask & 1 != 0 { self.write_oled_dump(1).await; }
                    if mask & 2 != 0 { self.write_oled_dump(2).await; }
                    continue;
                }
            };
            #[cfg(not(feature = "oled"))]
            let res = self.receiver.read_packet(&mut buf).await;
            match res {
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
                                    if is_help_cmd(line.trim()) {
                                        self.write_help().await;
                                        continue;
                                    }
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
                                    if is_help_cmd(line.trim()) {
                                        self.write_help().await;
                                        continue;
                                    }
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

    /// Print the device command list (`:help` / `:h` / `?`).
    async fn write_help(&mut self) {
        self.writeln("[help] Device commands").await;
        self.writeln("  Lobby:").await;
        for l in LOBBY_HELP {
            self.writeln(&alloc::format!("    {}", l)).await;
        }
        self.writeln("  Any time:").await;
        for l in META_HELP {
            self.writeln(&alloc::format!("    {}", l)).await;
        }
        self.writeln("  In battle:").await;
        for l in BATTLE_HELP {
            self.writeln(&alloc::format!("    {}", l)).await;
        }
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
        let trimmed = line.trim();
        let cmd = parse_lobby_cmd(trimmed);
        // Feedback for mistyped commands, but only for lines that *look* like
        // commands: replying to arbitrary junk (USB noise, our own output
        // echoed back by a cooked tty) could feed back into ourselves forever.
        if cmd == LobbyUsbCmd::Unknown && trimmed.starts_with(':') {
            self.writeln("[??] unknown command (:help for commands)").await;
        }
        if cmd == LobbyUsbCmd::UploadTeam {
            match parse_team_spec(trimmed) {
                Some((player, team)) => {
                    let n = team.len();
                    self.pending_lobby_team = Some((player, team));
                    self.writeln(&alloc::format!(
                        "Team uploaded for p{} ({} mon)",
                        player + 1,
                        n
                    ))
                    .await;
                }
                None => {
                    self.writeln(
                        "Bad :team syntax. Use: :team p1 species:move:move,species:...",
                    )
                    .await;
                }
            }
        }
        cmd
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

/// `:help` / `:h` / `?` — handled in `read_line` like the other meta-commands,
/// so it works at any time (lobby or battle).
fn is_help_cmd(line: &str) -> bool {
    matches!(line, ":help" | ":h" | "?")
}

/// Help lines for the meta-commands handled by [`handle_meta_cmd`] and
/// [`oled_dump_player`]. Keep in sync with their match arms; lobby commands
/// live in `LOBBY_HELP` next to `parse_lobby_cmd` in mega-blastoise-core.
const META_HELP: &[&str] = &[
    ":help / :h / ?    this list",
    ":oled [p1|p2]     dump an OLED framebuffer as ASCII art",
    ":oled auto on|off auto-dump framebuffers whenever they change",
    ":anim on|off      battle animations",
    ":oledlog on|off   OLED framebuffer RTT dump",
    ":reset            reboot the device",
];

/// Handle meta-commands that are valid at any time (lobby or battle).
/// Returns Some(ack message) if handled, None to pass the line through as normal input.
/// Side-effects (reset, flag toggle) happen before returning.
fn handle_meta_cmd(line: &str) -> Option<&'static str> {
    match line {
        ":reset"    => SCB::sys_reset(),
        ":anim off" => { ANIM_ENABLED.store(false, core::sync::atomic::Ordering::Relaxed); Some("[anim] animations OFF") }
        ":anim on"  => { ANIM_ENABLED.store(true,  core::sync::atomic::Ordering::Relaxed); Some("[anim] animations ON") }
        ":oled auto on" => {
            #[cfg(feature = "oled")]
            crate::subsystems::oled::set_usb_auto_dump(true);
            Some("[oled] auto dump ON")
        }
        ":oled auto off" => {
            #[cfg(feature = "oled")]
            crate::subsystems::oled::set_usb_auto_dump(false);
            Some("[oled] auto dump OFF")
        }
        ":oledlog on" => {
            #[cfg(feature = "oled")]
            crate::subsystems::oled::set_oled_dump(true);
            Some("[oledlog] OLED RTT dump ON")
        }
        ":oledlog off" => {
            #[cfg(feature = "oled")]
            crate::subsystems::oled::set_oled_dump(false);
            Some("[oledlog] OLED RTT dump OFF")
        }
        _           => None,
    }
}

impl InputSource for UsbBattleInput<'_> {
    async fn run(&mut self, bus: &InputBus) {
        UsbBattleInput::run(self, bus).await
    }
}

