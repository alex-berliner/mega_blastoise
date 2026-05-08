/// Host mirror of the firmware's button-event input pipeline.
///
/// Physical buttons → [`ButtonSource`] → [`ButtonController`] → battle choice.
/// Both GPIO (firmware) and this host implementation share the same pipeline;
/// the only difference is how "button pressed" is sensed — GPIO scan vs. stdin
/// or a pre-fed queue.
use std::collections::VecDeque;
use std::io::{self, Write};

use battler::{PlayerBattleData, Request};
use mega_blastoise_core::{
    ButtonController, ButtonSource, InputBus, InputSource,
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
/// falls back to auto-pick (switches), then blocks on stdin.
///
/// `on_prompt` prints the move list or bench state so the user knows what to
/// press — this is the host equivalent of what the OLED shows on hardware.
pub struct HostButtonSource {
    pub buttons: SimButtonSource,
    last_player_data: Option<PlayerBattleData>,
}

impl HostButtonSource {
    fn new() -> Self {
        Self { buttons: SimButtonSource::new(), last_player_data: None }
    }
}

impl ButtonSource for HostButtonSource {
    fn on_prompt(
        &mut self,
        player_id: &str,
        request: &Request,
        player_data: &Option<PlayerBattleData>,
    ) {
        self.last_player_data = player_data.clone();
        let label = player_label(player_id);
        match request {
            Request::Turn(turn) => {
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    println!("\n══ {label} ({player_id}) — choose move ══");
                    if n == 0 {
                        println!("  No moves available.");
                        continue;
                    }
                    for i in 0..n {
                        let m = &mon_req.moves[i];
                        let tag = if m.disabled {
                            " [DISABLED]"
                        } else if m.pp == 0 {
                            " [NO PP]"
                        } else {
                            ""
                        };
                        println!("  [{}] {:<20}  PP {}/{}{}", i + 1, m.name, m.pp, m.max_pp, tag);
                    }
                }
            }
            Request::Switch(sw) => {
                println!("\n══ {label} ({player_id}) — switch required ({} slot(s)) ══",
                    sw.needs_switch.len());
                if let Some(pd) = player_data {
                    for (i, m) in pd.mons.iter().enumerate() {
                        let slot = i + 1;
                        if m.active {
                            println!("  [{}] {} — active", slot, m.summary.name);
                        } else if m.hp == 0 {
                            println!("  [{}] {} — fainted", slot, m.summary.name);
                        } else {
                            let pct = m.hp * 100 / m.max_hp.max(1);
                            println!("  [{}] {} — HP {}/{} ({}%)  <-- available",
                                slot, m.summary.name, m.hp, m.max_hp, pct);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn wait_move(&mut self, player_id: &str, n: usize) -> usize {
        if let Some(slot) = self.buttons.try_next_move(n) {
            println!("[BTN] Button {} → move slot {}", slot + 1, slot + 1);
            return slot;
        }
        let label = player_label(player_id);
        stdin_number(&format!("{label}, button [1-{n}]: "), 1, n).await - 1
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        if let Some(idx) = self.buttons.try_next_switch() {
            println!("[BTN] Button {} → party slot {}", idx + 1, idx + 1);
            return idx;
        }
        if let Some(auto) = first_available_bench(&self.last_player_data) {
            println!("[AUTO] Switching in slot {} (first available bench)", auto + 1);
            return auto;
        }
        let label = player_label(player_id);
        stdin_number(&format!("{label}, switch slot [1-6]: "), 1, 6).await - 1
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

fn first_available_bench(player_data: &Option<PlayerBattleData>) -> Option<usize> {
    let pd = player_data.as_ref()?;
    pd.mons.iter().enumerate().find_map(|(i, m)| {
        if !m.active && m.hp > 0 { Some(i) } else { None }
    })
}

fn player_label(id: &str) -> &'static str {
    match id {
        "p1" => "Red",
        "p2" => "Blue",
        _ => "?",
    }
}

async fn stdin_number(prompt: &str, min: usize, max: usize) -> usize {
    loop {
        print!("{prompt}");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                eprintln!("\nstdin EOF — exiting.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("stdin error: {e} — exiting.");
                std::process::exit(1);
            }
            Ok(_) => {}
        }
        if let Ok(n) = line.trim().parse::<usize>() {
            if (min..=max).contains(&n) {
                return n;
            }
        }
        eprintln!("Enter a number from {min} to {max}.");
    }
}
