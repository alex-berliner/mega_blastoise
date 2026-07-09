/// Host mirror of the firmware's button-event input pipeline.
///
/// Physical buttons → [`ButtonSource`] → [`ButtonController`] → battle choice.
/// Both GPIO (firmware) and this host implementation share the same pipeline;
/// the only difference is how "button pressed" is sensed — GPIO scan vs. stdin
/// or a pre-fed queue.
use std::collections::VecDeque;
use std::io::{self, IsTerminal, Write};

use gen1_battle::{PlayerBattleData, Request};
use mega_blastoise_core::{
    format_prompt, player_display_name, ButtonController, ButtonSource, InputBus, InputSource, PlayerAction,
};

// ── Simulated button source ───────────────────────────────────────────────────

/// Pre-fed button queue — mirrors the GPIO button matrix of `PicoBattleInput`.
/// Move and switch queues are kept separate so pre-feeding one kind does not
/// accidentally consume entries intended for the other.
pub struct SimButtonSource {
    moves: VecDeque<usize>,
    switches: VecDeque<usize>,
}

impl SimButtonSource {
    pub fn new() -> Self {
        Self { moves: VecDeque::new(), switches: VecDeque::new() }
    }

    pub fn queue_move(&mut self, slot: usize) {
        self.moves.push_back(slot);
    }

    pub fn queue_switch(&mut self, party_index: usize) {
        self.switches.push_back(party_index);
    }

    fn try_next_move(&mut self, n: usize) -> Option<usize> {
        while let Some(&slot) = self.moves.front() {
            self.moves.pop_front();
            if slot < n {
                return Some(slot);
            }
        }
        None
    }

    fn try_next_switch(&mut self) -> Option<usize> {
        self.switches.pop_front()
    }
}

// ── HostButtonSource ──────────────────────────────────────────────────────────

/// Implements [`ButtonSource`] for host use: checks the pre-fed queue first,
/// then blocks on stdin.
///
/// `on_prompt` prints the move list and available bench so the user knows what to
/// press — the host equivalent of what the OLED shows on hardware.
pub struct HostButtonSource {
    pub buttons: SimButtonSource,
    // Per-player snapshots — both players' prompts arrive (and fire on_prompt)
    // before either choice is collected, so a single shared slot would leave
    // p1's wait_switch reading p2's bench.
    player_data: [Option<PlayerBattleData>; 2],
    auto_cycle: usize,
}

impl HostButtonSource {
    fn new() -> Self {
        Self { buttons: SimButtonSource::new(), player_data: [None, None], auto_cycle: 0 }
    }

    fn player_data(&self, player_id: &str) -> Option<&PlayerBattleData> {
        self.player_data[(player_id == "p2") as usize].as_ref()
    }
}

impl ButtonSource for HostButtonSource {
    fn on_prompt(
        &mut self,
        player_id: &str,
        request: &Request,
        player_data: &Option<PlayerBattleData>,
    ) {
        self.player_data[(player_id == "p2") as usize] = player_data.clone();
        print!("\n{}", format_prompt(player_id, request, player_data.as_ref()));
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        if let Some(slot) = self.buttons.try_next_move(n_moves) {
            println!("[BTN] Move button {} pressed", slot + 1);
            return PlayerAction::Move(slot);
        }
        if let Some(idx) = self.buttons.try_next_switch() {
            println!("[BTN] Party button {} pressed", idx + 1);
            return PlayerAction::Switch(idx);
        }
        // Automated runs (no tty to prompt): cycle through every move slot then
        // every party slot. The controller's validating retry loop keeps calling
        // until a legal action comes up, so this always converges (e.g. when the
        // pre-fed slot is out of PP). Blocking on a closed stdin would hit
        // read_stdin_line's exit(0) and silently kill the test harness.
        if !io::stdin().is_terminal() {
            let party = self.player_data(player_id).map(|pd| pd.mons.len()).unwrap_or(6);
            let i = self.auto_cycle % (n_moves + party);
            self.auto_cycle += 1;
            let action = if i < n_moves {
                println!("[BTN] auto move slot {}", i + 1);
                PlayerAction::Move(i)
            } else {
                println!("[BTN] auto party slot {}", i - n_moves + 1);
                PlayerAction::Switch(i - n_moves)
            };
            return action;
        }
        let label = player_display_name(player_id);
        loop {
            print!("{label} > ");
            let _ = io::stdout().flush();
            let trimmed = read_stdin_line();
            if let Ok(n) = trimmed.parse::<usize>() {
                if (1..=n_moves).contains(&n) {
                    return PlayerAction::Move(n - 1);
                }
            }
            if let Some(rest) = trimmed.strip_prefix('s') {
                if let Ok(n) = rest.parse::<usize>() {
                    if n >= 1 {
                        return PlayerAction::Switch(n - 1);
                    }
                }
            }
            eprintln!("Enter 1-{n_moves} for a move, or s1-s3 to switch.");
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        if let Some(idx) = self.buttons.try_next_switch() {
            println!("[BTN] Party button {} pressed", idx + 1);
            return idx;
        }
        let label = player_display_name(player_id);
        let available: Vec<usize> = self.player_data(player_id)
            .map(|pd| pd.mons.iter().enumerate()
                .filter(|(_, m)| !m.active && m.hp > 0)
                .map(|(i, _)| i)
                .collect())
            .unwrap_or_default();
        // Automated runs (no tty to prompt): auto-pick the first available bench
        // slot, as documented on HostBattleController. Blocking on a closed stdin
        // would hit read_stdin_line's exit(0) and silently kill the test harness.
        if !io::stdin().is_terminal() {
            if let Some(&idx) = available.first() {
                println!("[BTN] auto-pick party slot {}", idx + 1);
                return idx;
            }
        }
        let max = self.player_data(player_id).map(|pd| pd.mons.len()).unwrap_or(6);
        loop {
            let n = stdin_number(&format!("{label}, slot [1-{max}]: "), 1, max).await - 1;
            if available.is_empty() || available.contains(&n) {
                return n;
            }
            if let Some(pd) = self.player_data(player_id) {
                let m = &pd.mons[n];
                if m.active {
                    eprintln!("  {} is already in battle.", m.summary.name);
                } else {
                    eprintln!("  {} has fainted.", m.summary.name);
                }
            }
        }
    }
}

// ── HostBattleController ──────────────────────────────────────────────────────

/// Host input controller.  Pre-feed presses via `controller.buttons.queue_move(slot)` /
/// `queue_switch(idx)` for automated tests.  When the queue is empty the controller
/// falls back to stdin (interactive use) or auto-picks the first available bench slot
/// (switch prompts).
pub struct HostBattleController {
    inner: ButtonController<HostButtonSource>,
}

impl HostBattleController {
    pub fn new() -> Self {
        Self {
            inner: ButtonController::with_log_sink(HostButtonSource::new(), |line| {
                println!("[EVT] {line}");
            }),
        }
    }

    /// Access the simulated button queue — pre-feed moves/switches for automated tests.
    pub fn buttons_mut(&mut self) -> &mut SimButtonSource {
        &mut self.inner.source.buttons
    }
}

impl InputSource for HostBattleController {
    async fn run(&mut self, bus: &InputBus) {
        self.inner.run(bus).await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_stdin_line() -> String {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => { eprintln!("\nstdin EOF — exiting."); std::process::exit(0); }
        Err(e) => { eprintln!("stdin error: {e} — exiting."); std::process::exit(1); }
        Ok(_) => {}
    }
    line.trim().to_string()
}

async fn stdin_number(prompt: &str, min: usize, max: usize) -> usize {
    loop {
        print!("{prompt}");
        let _ = io::stdout().flush();
        let s = read_stdin_line();
        if let Ok(n) = s.parse::<usize>() {
            if (min..=max).contains(&n) {
                return n;
            }
        }
        eprintln!("Enter a number from {min} to {max}.");
    }
}
