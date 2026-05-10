use battler::{PlayerBattleData, Request};
use mega_blastoise_core::{format_prompt, party_slot_from_mon, ButtonSource, PlayerAction};

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
        if let Some(pd) = player_data {
            let player = if player_id == "p1" { 1u8 } else { 2u8 };
            let slots = pd.mons.iter().map(party_slot_from_mon).collect();
            crate::update_party(player, slots);
        }
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        loop {
            let ev = crate::PlayerButtonFuture(player).await;
            match ev {
                crate::ButtonEvent::Move { slot, .. } if (slot as usize) < n_moves => {
                    return PlayerAction::Move(slot as usize);
                }
                crate::ButtonEvent::Switch { idx, .. } => {
                    return PlayerAction::Switch(idx as usize);
                }
                _ => {}
            }
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        loop {
            let ev = crate::PlayerButtonFuture(player).await;
            if let crate::ButtonEvent::Switch { idx, .. } = ev {
                return idx as usize;
            }
        }
    }
}
