use battler::{PlayerBattleData, Request};
use js_sys::Date;
use mega_blastoise_core::{format_prompt, party_slot_from_mon, ButtonSource, PlayerAction};

#[derive(Clone)]
pub struct WebButtonSource;

impl ButtonSource for WebButtonSource {
    fn on_prompt(
        &mut self,
        player_id: &str,
        request: &Request,
        player_data: &Option<PlayerBattleData>,
    ) {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        let text = format_prompt(player_id, request, player_data.as_ref());
        for line in text.lines() {
            crate::print_log(line);
        }
        // Restore the battle screen — clears any "waiting / unready" overlay.
        crate::restore_screen(player);
        if let Some(pd) = player_data {
            let slots = pd.mons.iter().map(party_slot_from_mon).collect();
            crate::update_party(player, slots);
            crate::sync_party_leds(player);
            if matches!(request, Request::Switch(_)) {
                crate::show_switch_screen(player);
            }
        }
    }

    fn on_choice_pending(&mut self, player_id: &str) {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        if !crate::is_ai_player(player) {
            crate::show_waiting_screen(player);
        }
    }

    async fn wait_cancel_window(&mut self, player_id: &str) -> bool {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        if crate::is_ai_player(player) { return false; }
        let deadline = Date::now() + 1000.0;
        loop {
            if crate::pop_player_button(player).is_some() { return true; }
            if Date::now() >= deadline { return false; }
            crate::sleep_ms(50).await;
        }
    }

    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        if crate::is_ai_player(player) {
            while crate::is_ai_paused() { crate::sleep_ms(100).await; }
            return PlayerAction::Move(crate::ai_pick_move(n_moves));
        }
        loop {
            let ev = crate::PlayerButtonFuture(player).await;
            match ev {
                crate::ButtonEvent::Move { slot, .. } if (slot as usize) < n_moves => {
                    return PlayerAction::Move(slot as usize);
                }
                crate::ButtonEvent::Switch { idx, .. } => {
                    let idx = idx as usize;
                    if !crate::party_slot_alive(player, idx) {
                        crate::show_invalid_selection(player);
                        crate::sleep_ms(600).await;
                        crate::restore_screen(player);
                        continue;
                    }
                    return PlayerAction::Switch(idx);
                }
                _ => {}
            }
        }
    }

    async fn wait_switch(&mut self, player_id: &str) -> usize {
        let player = if player_id == "p1" { 1u8 } else { 2u8 };
        if crate::is_ai_player(player) {
            while crate::is_ai_paused() { crate::sleep_ms(100).await; }
            return crate::ai_pick_switch(player);
        }
        loop {
            let ev = crate::PlayerButtonFuture(player).await;
            if let crate::ButtonEvent::Switch { idx, .. } = ev {
                let idx = idx as usize;
                if !crate::party_slot_alive(player, idx) {
                    crate::show_invalid_selection(player);
                    crate::sleep_ms(600).await;
                    crate::restore_screen(player);
                    continue;
                }
                return idx;
            }
        }
    }
}
