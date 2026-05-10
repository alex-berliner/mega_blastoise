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
            let ev = crate::PlayerButtonFuture(player).await;
            match ev {
                crate::ButtonEvent::Move { slot, .. } if (slot as usize) < n_moves => {
                    crate::set_active_player(0);
                    return PlayerAction::Move(slot as usize);
                }
                crate::ButtonEvent::Switch { idx, .. } => {
                    crate::set_active_player(0);
                    return PlayerAction::Switch(idx as usize);
                }
                crate::ButtonEvent::LongPressMove { slot, .. } if (slot as usize) < n_moves => {
                    crate::show_move_detail(player, slot as usize);
                    // Wait for release; ignore other events until then.
                    loop {
                        let ev2 = crate::PlayerButtonFuture(player).await;
                        if matches!(ev2, crate::ButtonEvent::LongPressRelease { .. }) { break; }
                    }
                    crate::restore_screen(player);
                }
                _ => {}
            }
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        crate::set_active_player(player);
        loop {
            let ev = crate::PlayerButtonFuture(player).await;
            if let crate::ButtonEvent::Switch { idx, .. } = ev {
                crate::set_active_player(0);
                return idx as usize;
            }
        }
    }
}
