use std::io::{self, Write};

use mega_blastoise_core::choice_collect::SlotOptions;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, player_display_name,
    ActivePrompt, InputBus, InputSource, PlayerChoice,
};

pub struct StdinBattleInput;

impl StdinBattleInput {
    /// Run forever: wait for each prompt on `bus`, handle it synchronously, send the choice back.
    /// Blocking stdin reads are fine here — the test binary is single-threaded and nothing
    /// else needs to run while waiting for the user to type.
    pub async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { player_id, slot, .. } = bus.prompt.receive().await;
            let choice = self.handle(&player_id, &slot);
            bus.choices.send(PlayerChoice { player_id, choice }).await;
        }
    }

    fn handle(&self, player_id: &str, slot: &SlotOptions) -> String {
        let label = player_display_name(player_id);
        if let Some(auto) = &slot.auto {
            return auto.clone();
        }
        if slot.forced_switch {
            println!("\n=== {label} ({player_id}) — switch (bench 1-6) ===");
            let bench = Self::prompt_usize_inclusive(
                &format!("{label}, which party slot to send in [1-6]? "),
                1,
                6,
            );
            return format_switch_choice(bench - 1);
        }
        println!("\n=== {label} ({player_id}) — choose move ===");
        let n_moves = slot.n_moves;
        if n_moves == 0 {
            eprintln!("No moves available; passing.");
            return "pass".to_string();
        }
        for i in 0..n_moves {
            let status = if slot.usable[i] { "" } else { " (disabled)" };
            println!("  [{}]{}", i + 1, status);
        }
        loop {
            let btn = Self::prompt_usize_inclusive(
                &format!("{label}, pick move [1-{n_moves}]: "),
                1,
                n_moves,
            );
            let picked = btn - 1;
            if !slot.usable[picked] {
                eprintln!("That move cannot be used. Pick another.");
                continue;
            }
            return format_move_choice(picked);
        }
    }

    fn prompt_usize_inclusive(prompt: &str, min: usize, max: usize) -> usize {
        loop {
            print!("{prompt}");
            let _ = io::stdout().flush();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => {
                    eprintln!("\nstdin closed (EOF) — exiting.");
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
            eprintln!("Please enter a number from {min} to {max}.");
        }
    }
}

impl InputSource for StdinBattleInput {
    async fn run(&mut self, bus: &InputBus) {
        StdinBattleInput::run(self, bus).await
    }
}
