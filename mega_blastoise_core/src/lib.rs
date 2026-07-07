#![no_std]

extern crate alloc;

pub mod battle_effects;
pub mod cli_parse;
pub mod display;
pub mod hp_bar;
pub mod board_event;
pub mod battle_input;
pub mod battle_runner;
pub mod data_store;
pub mod demo_teams;
pub mod prompt_fmt;
pub mod randbat;
pub mod rng;
pub mod random_ai;
pub mod sprites;

pub use battle_effects::{
    anim, process_new_log_lines, BoardEffects, BoardEventQueue, NoopBoardEffects,
};
pub use board_event::{
    board_prompt_event, mon_display_name, mon_player_id, parse_log_line, player_display_name,
    mon_player_num, player_id_to_num, side_display_name, status_abbrev, BoardEvent, MoveSlot, ParsedBattleLogLine, PromptKind,
};
pub use display::{party_slot_from_mon, render_event_text, render_invalid_selection, render_lobby_screen, render_move_detail, render_player_screen, render_pokemon_stats, render_pokemon_stats_page2, render_switch_screen, render_waiting_for_opponent, render_waiting_screen, render_win_screen, OledFrameBuffer, PartySlotData};
pub use battle_input::{
    format_move_choice, format_switch_choice, join_choice_parts, switch_choice_from_team_indices,
    turn_action_choice, turn_choice_from_move_slots, ActionReject, ActivePrompt, ButtonController,
    ButtonSource, InputBus, InputSource, NoInput, PlayerAction,
};
pub use battle_runner::{battle_options_with_seed, demo_battle_options, demo_engine_opts, make_player, run_battle, LOBBY_DEMO_DELAY_MS};
pub use hp_bar::{hp_bar_color, hp_bar_count, HpBarState};
pub use prompt_fmt::{format_active_state, format_lobby_status, format_player_state, format_prompt};
pub use data_store::FlashDataStore;
pub use demo_teams::{demo_team_blue, demo_team_red};
pub use randbat::{draw_randbat_team, draw_two_randbat_teams, TEAM_SEED_SALT};
pub use rng::SimpleRng;
pub use random_ai::RandomAi;
pub use cli_parse::{
    parse_lobby_cmd, parse_switch_line, parse_team_spec, parse_turn_line, parse_web_game_cmd,
    LobbyCmd, TurnChoice, WebGameInput, LOBBY_HELP,
};
