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
    parse_lobby_cmd, parse_switch_line, parse_team_spec, parse_turn_line, party_slot_from_mon,
    player_id_to_num, turn_action_choice, ActionReject, ActivePrompt, ButtonSource, InputBus,
    InputSource, PlayerAction, PlayerChoice, RandomAi, TurnChoice, LOBBY_HELP, TEAM_SEED_SALT,
};
use gen1_battle::MonData;
use mega_blastoise_fw::usb_cdc_line::{log_usb_rx_line_str_to_rtt, write_crlf};

use crate::pico_battle_input::{PicoBattleInput, PlayerTurn, TwoTurnProgress};
#[cfg(feature = "oled")]
use crate::subsystems::oled::{read_shadow_fb, send as oled_send, wait_fb_change, OledCmd};

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
            last_player_data: None,
            ai_players: [false, false],
            ai: RandomAi::new(TEAM_SEED_SALT),
            pending_lobby_team: None,
        }
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

            // Refresh each player's party snapshot so the OLED long-press stats
            // screen has data (run_inner doesn't go through ButtonSource::on_prompt).
            // A forced switch also flips that player's OLED to the pick-a-mon
            // screen, like web's on_prompt does.
            #[cfg(feature = "oled")]
            for p in &prompts {
                send_party_update(p);
                if matches!(p.request, Request::Switch(_)) {
                    oled_send(OledCmd::ShowSwitchScreen { player: player_id_to_num(p.player_id.as_str()) });
                }
            }

            // A player with no prompt this batch is waiting on the other (e.g.
            // a forced switch after a faint) — show their waiting screen.
            #[cfg(feature = "oled")]
            for pid in ["p1", "p2"] {
                if !prompts.iter().any(|p| p.player_id == pid) {
                    oled_send(OledCmd::ShowWaitingForOpponent { player: player_id_to_num(pid) });
                }
            }

            // ── Resolve each prompt: AI auto-chooses now; humans are deferred. ─
            let mut choices: Vec<Option<String>> = (0..prompts.len()).map(|_| None).collect();
            let mut humans: Vec<usize> = Vec::new();
            for (i, p) in prompts.iter().enumerate() {
                let idx = if p.player_id.as_str() == "p1" { 0 } else { 1 };
                if self.ai_players[idx] {
                    self.write_dbg(&alloc::format!("[AI] auto-choosing for {}", p.player_id.as_str())).await;
                    choices[i] = Some(self.ai.make_choice(&p.request, p.player_data.as_ref()));
                } else {
                    humans.push(i);
                }
            }

            // ── Collect human choices. Two human boards → fully parallel
            //    (each player operates independently, no waiting on the other). ─
            let parallel = buttons.is_some()
                && humans.len() == 2
                && !is_multi_switch(&prompts[humans[0]].request)
                && !is_multi_switch(&prompts[humans[1]].request);

            if parallel {
                let (i0, i1) = (humans[0], humans[1]);
                self.display_prompt(&prompts[i0]).await;
                self.display_prompt(&prompts[i1]).await;
                let players = [
                    PlayerTurn::from_request(
                        prompts[i0].player_id.as_str(), &prompts[i0].request, prompts[i0].player_data.as_ref(),
                    ),
                    PlayerTurn::from_request(
                        prompts[i1].player_id.as_str(), &prompts[i1].request, prompts[i1].player_data.as_ref(),
                    ),
                ];
                let ids = [prompts[i0].player_id.clone(), prompts[i1].player_id.clone()];
                let btns = buttons.as_mut().expect("parallel requires buttons");

                // Race typed USB lines ("p1 2", "p2 s3") against the button
                // matrix. Progress lives outside the button future, so losing
                // the select doesn't forget a choice a player already made.
                // wait_two_turns returns only after both players committed AND
                // the unready grace window passed — loop until then, so a
                // USB-typed final choice still gets the grace period.
                let mut progress = TwoTurnProgress::new(&players);
                loop {
                    match select(self.read_line(), btns.wait_two_turns(&players, &mut progress)).await {
                        Either::First(line) => {
                            self.handle_parallel_line(line.trim(), &players, &ids, &mut progress).await;
                        }
                        Either::Second(()) => break,
                    }
                }
                let [c0, c1] = progress.into_choices();
                choices[i0] = Some(c0);
                choices[i1] = Some(c1);
            } else {
                for &i in &humans {
                    let btns = buttons.as_mut().map(|b| &mut **b);
                    let pid = prompts[i].player_id.clone();
                    let req = prompts[i].request.clone();
                    let pd = prompts[i].player_data.clone();
                    let choice = self.handle(&pid, &req, pd, btns).await;
                    choices[i] = Some(choice);
                }
            }

            // ── Submit choices, tagged by player (the runner routes by id). ──
            for (p, c) in prompts.iter().zip(choices) {
                let c = c.unwrap_or_else(|| String::from("pass"));
                self.write_dbg(&alloc::format!("Submitting to engine ({}): \"{}\"", p.player_id.as_str(), c)).await;
                bus.choices.send(PlayerChoice { player_id: p.player_id.clone(), choice: c }).await;
            }

            while let Ok(line) = bus.log.try_receive() {
                self.write_event(&line).await;
            }
        }
    }

    /// Write a player's prompt text to USB without collecting input — used when
    /// the parallel button collector owns the actual input phase.
    async fn display_prompt(&mut self, p: &ActivePrompt) {
        if p.player_data.is_some() {
            self.last_player_data = p.player_data.clone();
        }
        self.write("\r\n").await;
        self.write_multiline(&format_prompt(p.player_id.as_str(), &p.request, p.player_data.as_ref())).await;
    }

    /// Parse a typed turn line, accepting both bare ("2", "s3") and
    /// player-prefixed ("p1 2") forms — the prefixed form is the universal
    /// syntax, the bare form is unambiguous while a single player is prompted.
    /// Writes the rejection message itself; None means "re-prompt".
    async fn parse_typed_turn(&mut self, line: &str, player_id: &str, n: usize) -> Option<PlayerAction> {
        let rest = match strip_player_prefix(line.trim(), player_id) {
            Ok(r) => r,
            Err(msg) => {
                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                return None;
            }
        };
        match parse_turn_line(rest, n) {
            Ok(TurnChoice::Move(s)) => Some(PlayerAction::Move(s)),
            Ok(TurnChoice::Switch(i)) => Some(PlayerAction::Switch(i)),
            Err(msg) => {
                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                None
            }
        }
    }

    /// Parse a typed forced-switch line: a party slot as "3", "s3", or the
    /// player-prefixed "p1 3" / "p1 s3". Writes the rejection itself.
    async fn parse_typed_switch(&mut self, line: &str, player_id: &str) -> Option<usize> {
        let rest = match strip_player_prefix(line.trim(), player_id) {
            Ok(r) => r,
            Err(msg) => {
                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                return None;
            }
        };
        let slot = rest.strip_prefix('s').unwrap_or(rest).trim();
        match parse_switch_line(slot) {
            Ok(idx) => Some(idx),
            Err(msg) => {
                self.write_err(&alloc::format!("Rejected — {}", msg)).await;
                None
            }
        }
    }

    /// Whether team slot `idx` in `last_player_data` is fainted (only a
    /// definite yes rejects — missing data defers to the engine).
    fn slot_fainted(&self, idx: usize) -> bool {
        self.last_player_data
            .as_ref()
            .and_then(|pd| pd.mons.get(idx))
            .is_some_and(|m| m.hp == 0)
    }

    /// Handle a typed line while both players are choosing in parallel.
    /// Requires a player prefix — "p1 <cmd>" / "p2 <cmd>" — where <cmd> is the
    /// usual turn syntax (move 1-4 or s2-s6). Runs through the same shared
    /// validator as button presses.
    async fn handle_parallel_line(
        &mut self,
        line: &str,
        players: &[PlayerTurn; 2],
        ids: &[String; 2],
        progress: &mut TwoTurnProgress,
    ) {
        if line.is_empty() {
            return;
        }
        let mut parsed = None;
        for i in 0..2 {
            if let Some(rest) = line.strip_prefix(ids[i].as_str()) {
                if rest.starts_with(' ') {
                    parsed = Some((i, rest.trim()));
                    break;
                }
            }
        }
        let Some((i, rest)) = parsed else {
            self.write_err("Both players are choosing — prefix with the player, e.g. \"p1 2\" or \"p2 s3\"").await;
            return;
        };
        // A committed player may retype: a valid line replaces their choice
        // (and restarts the grace window); auto choices are fixed.
        if progress.is_done(i) && progress.is_auto(i) {
            self.write_err(&alloc::format!("{}'s action is forced this turn", ids[i].as_str())).await;
            return;
        }

        // Forced replacement after a faint: expect a party slot ("3" or "s3").
        if players[i].forced_switch() {
            let slot = rest.strip_prefix('s').unwrap_or(rest).trim();
            match parse_switch_line(slot) {
                Ok(idx) if players[i].switch_target_ok(idx) => {
                    let choice = format_switch_choice(idx);
                    self.write_ok(&alloc::format!("Accepted — {} {}", ids[i].as_str(), choice.as_str())).await;
                    progress.set_choice(i, players, choice);
                }
                Ok(_) => {
                    self.write_err(&alloc::format!(
                        "Rejected ({}) — That pokemon can no longer fight!", ids[i].as_str()
                    )).await;
                }
                Err(msg) => {
                    self.write_err(&alloc::format!("Rejected ({}) — {}", ids[i].as_str(), msg)).await;
                }
            }
            return;
        }

        let n = players[i].n_moves();
        let action = match parse_turn_line(rest, n) {
            Ok(TurnChoice::Move(s)) => PlayerAction::Move(s),
            Ok(TurnChoice::Switch(x)) => PlayerAction::Switch(x),
            Err(msg) => {
                self.write_err(&alloc::format!("Rejected ({}) — {}", ids[i].as_str(), msg)).await;
                return;
            }
        };
        if let PlayerAction::Switch(idx) = &action {
            // Already-active is caught by the shared validator; a benched target
            // that still fails here can only be fainted.
            if players[i].active_slot_is_not(*idx) && !players[i].switch_target_ok(*idx) {
                self.write_err(&alloc::format!(
                    "Rejected ({}) — That pokemon can no longer fight!", ids[i].as_str()
                )).await;
                return;
            }
        }
        match players[i].typed_action_choice(&action) {
            Ok(choice) => {
                self.write_ok(&alloc::format!("Accepted — {} {}", ids[i].as_str(), choice.as_str())).await;
                progress.set_choice(i, players, choice);
            }
            Err(reason) => {
                self.write_err(&alloc::format!("Rejected ({}) — {}", ids[i].as_str(), reject_reason(reason))).await;
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

                    let mut usable = [false; 4];
                    for i in 0..n {
                        usable[i] = !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0;
                    }
                    let active_slot = Some(mon_req.team_position as usize);

                    'move_input: loop {
                        self.write_move_prompt(n).await;

                        // Obtain an action: a button press wins immediately (long-press
                        // detail handled internally), or a parsed USB line. Both then go
                        // through the SAME shared validator, so the rules can't drift.
                        let action: PlayerAction = match buttons.as_mut() {
                            Some(btns) => match select(self.read_line(), btns.wait_action(player_id, n)).await {
                                Either::First(line) => match self.parse_typed_turn(&line, player_id, n).await {
                                    Some(a) => a,
                                    None => continue 'move_input,
                                },
                                Either::Second(a) => {
                                    self.partial.clear();
                                    a
                                }
                            },
                            None => {
                                let line = self.read_line().await;
                                match self.parse_typed_turn(&line, player_id, n).await {
                                    Some(a) => a,
                                    None => continue 'move_input,
                                }
                            }
                        };

                        if let PlayerAction::Switch(idx) = &action {
                            if self.slot_fainted(*idx) {
                                self.write_err("Rejected — That pokemon can no longer fight!").await;
                                continue 'move_input;
                            }
                        }
                        match turn_action_choice(&action, n, &usable, mon_req.trapped, active_slot) {
                            Ok(choice) => {
                                match &action {
                                    PlayerAction::Move(s) => {
                                        self.write_ok(&alloc::format!("Accepted — {} (slot {})", mon_req.moves[*s].name, s)).await;
                                    }
                                    PlayerAction::Switch(i) => {
                                        self.write_ok(&alloc::format!("Switching in slot {}", i + 1)).await;
                                    }
                                }
                                parts.push(choice);
                                break 'move_input;
                            }
                            Err(reason) => {
                                self.write_err(&alloc::format!("Rejected — {}", reject_reason(reason))).await;
                                continue 'move_input;
                            }
                        }
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
                                        match self.parse_typed_switch(&line, player_id).await {
                                            Some(idx) => idx,
                                            None => continue 'switch_input,
                                        }
                                    }
                                    Either::Second(idx) => {
                                        self.partial.clear();
                                        idx
                                    }
                                }
                            }
                            None => {
                                let line = self.read_line().await;
                                match self.parse_typed_switch(&line, player_id).await {
                                    Some(idx) => idx,
                                    None => continue 'switch_input,
                                }
                            }
                        };

                        // Both button and typed selections go through the same
                        // aliveness check before the engine sees them.
                        let is_active = self.last_player_data.as_ref()
                            .and_then(|pd| pd.mons.get(team_idx))
                            .is_some_and(|m| m.active);
                        if self.slot_fainted(team_idx) || is_active {
                            if is_active {
                                self.write_err("Rejected — that Pokémon is already in battle").await;
                            } else {
                                self.write_err("Rejected — That pokemon can no longer fight!").await;
                            }
                            // Flash the invalid-selection screen for button players.
                            #[cfg(feature = "oled")]
                            {
                                let player = player_id_to_num(player_id);
                                oled_send(OledCmd::ShowInvalidSelection { player });
                                embassy_time::Timer::after_millis(600).await;
                                oled_send(OledCmd::ShowSwitchScreen { player });
                            }
                            continue 'switch_input;
                        }

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

    async fn write_move_prompt(&mut self, n: usize) {
        self.writef(&alloc::format!("Move [1-{}]: ", n)).await;
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
        self.writeln("  In battle: answer the prompt with a move number, or 'switch N'.").await;
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

/// Push a player's party snapshot to their OLED, so a long-press on a party
/// button (`ShowPokemonStats`) has bench data to render. Mirrors what
/// `ButtonSource::on_prompt` does on the button-only path.
#[cfg(feature = "oled")]
fn send_party_update(p: &ActivePrompt) {
    if let Some(pd) = &p.player_data {
        let player = player_id_to_num(p.player_id.as_str());
        let slots = pd.mons.iter().map(party_slot_from_mon).collect();
        oled_send(OledCmd::PartyUpdate { player, slots });
    }
}

/// Human-readable reason for a rejected turn action (shown over USB).
fn reject_reason(r: ActionReject) -> &'static str {
    match r {
        ActionReject::OutOfRange => "no such move",
        ActionReject::Unusable => "move is disabled or out of PP",
        ActionReject::Trapped => "Pokémon is trapped, cannot switch",
        ActionReject::AlreadyActive => "that Pokémon is already in battle",
    }
}

/// True when a request needs more than one replacement (e.g. multi-faint). The
/// parallel collector handles a single switch per player, so these fall back to
/// the serial path.
fn is_multi_switch(request: &Request) -> bool {
    matches!(request, Request::Switch(sw) if sw.needs_switch.len() > 1)
}

/// Strip an optional "p1 "/"p2 " prefix from a typed line. Bare lines pass
/// through (unambiguous while a single player is prompted); a prefix naming
/// the other player is an error.
fn strip_player_prefix<'a>(line: &'a str, player_id: &str) -> Result<&'a str, &'static str> {
    for pid in ["p1", "p2"] {
        if let Some(rest) = line.strip_prefix(pid) {
            if rest.starts_with(' ') {
                return if pid == player_id {
                    Ok(rest.trim())
                } else {
                    Err("that prefix names the other player")
                };
            }
        }
    }
    Ok(line)
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

