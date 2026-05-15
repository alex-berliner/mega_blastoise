//! Active prompt content delivered to the input source.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::data::MoveSlot;

#[derive(Clone, Debug)]
pub struct MonTurnRequest {
    /// Index into the player's team that this prompt is for.
    pub team_position: u8,
    /// Move slots available to choose from.
    pub moves: Vec<MoveSlot>,
    /// True if the mon cannot switch out this turn.
    pub trapped: bool,
    /// True if the mon must use the same move (Wrap, Bide, Hyper Beam recharge, etc.).
    pub locked_into_move: bool,
}

#[derive(Clone, Debug)]
pub struct TurnRequest {
    pub active: Vec<MonTurnRequest>,
}

#[derive(Clone, Debug)]
pub struct SwitchRequest {
    pub needs_switch: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct TeamPreviewRequest {
    pub max_team_size: u8,
}

#[derive(Clone, Debug)]
pub struct LearnMoveRequest {
    pub mon: String,
    pub moves: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Request {
    Turn(TurnRequest),
    Switch(SwitchRequest),
    TeamPreview(TeamPreviewRequest),
    LearnMove(LearnMoveRequest),
}
