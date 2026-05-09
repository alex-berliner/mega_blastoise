use battler::{PlayerBattleData, Request};
use mega_blastoise_core::{format_prompt, ButtonSource, PlayerAction};

pub struct WebButtonSource;

impl ButtonSource for WebButtonSource {
    fn on_prompt(
        &mut self,
        player_id: &str,
        request: &Request,
        player_data: &Option<PlayerBattleData>,
    ) {
        let text = format_prompt(player_id, request, player_data.as_ref());
        for line in text.lines() {
            crate::print(line);
        }
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        let label = if player_id == "p1" { "Red" } else { "Blue" };
        loop {
            let line = crate::read_input_line().await;
            let trimmed = line.trim();

            if let Ok(n) = trimmed.parse::<usize>() {
                if (1..=n_moves).contains(&n) {
                    crate::print(&format!("{label} > {trimmed}"));
                    return PlayerAction::Move(n - 1);
                }
            }
            if let Some(rest) = trimmed.strip_prefix('s') {
                if let Ok(n) = rest.parse::<usize>() {
                    if n >= 1 {
                        crate::print(&format!("{label} > {trimmed}"));
                        return PlayerAction::Switch(n - 1);
                    }
                }
            }
            if !trimmed.is_empty() {
                crate::print(&format!(
                    "  (enter 1-{n_moves} for a move, or s1-s3 to switch)"
                ));
            }
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let label = if player_id == "p1" { "Red" } else { "Blue" };
        loop {
            let line = crate::read_input_line().await;
            let trimmed = line.trim();
            if let Ok(n) = trimmed.parse::<usize>() {
                if n >= 1 {
                    crate::print(&format!("{label} > s{trimmed}"));
                    return n - 1;
                }
            }
            if !trimmed.is_empty() {
                crate::print("  (enter party slot number, e.g. 2)");
            }
        }
    }
}
