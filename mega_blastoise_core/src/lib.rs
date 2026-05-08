#![no_std]

extern crate alloc;

pub mod battle_effects;
pub mod display;
pub mod board_event;
pub mod battle_input;
pub mod battle_runner;
pub mod data_store;
pub mod demo_teams;
pub mod randbat;

pub use battle_effects::{
    process_new_log_lines, BoardEffects, BoardEventQueue, NoopBoardEffects,
};
pub use board_event::{
    board_prompt_event, parse_log_line, player_display_name, side_display_name, BoardEvent,
    MoveSlot, ParsedBattleLogLine, PromptKind,
};
pub use display::{render_move_detail, render_player_screen};
pub use battle_input::{
    format_move_choice, format_switch_choice, join_choice_parts, switch_choice_from_team_indices,
    turn_choice_from_move_slots, ActivePrompt, InputBus, InputSource, NoInput,
};
pub use battle_runner::{demo_battle_options, demo_engine_opts, make_player, run_battle};
pub use data_store::FlashDataStore;
pub use demo_teams::{demo_team_blue, demo_team_red};
pub use randbat::draw_randbat_team;
