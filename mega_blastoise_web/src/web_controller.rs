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
            crate::print_log(line);
        }
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        crate::set_active_player(player);

        loop {
            let ev = crate::ButtonFuture.await;
            let action = match ev {
                crate::ButtonEvent::Move { player: p, slot } if p == player => {
                    if (slot as usize) < n_moves {
                        Some(PlayerAction::Move(slot as usize))
                    } else {
                        None
                    }
                }
                crate::ButtonEvent::Switch { player: p, idx } if p == player => {
                    Some(PlayerAction::Switch(idx as usize))
                }
                _ => None,
            };
            if let Some(a) = action {
                crate::set_active_player(0);
                return a;
            }
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        crate::set_active_player(player);

        loop {
            let ev = crate::ButtonFuture.await;
            if let crate::ButtonEvent::Switch { player: p, idx } = ev {
                if p == player {
                    crate::set_active_player(0);
                    return idx as usize;
                }
            }
        }
    }
}
