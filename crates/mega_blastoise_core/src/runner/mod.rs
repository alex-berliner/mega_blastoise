//! The battle runners, and the switch between them.
//!
//! A runner's whole job is: distil each seat's options into the neutral
//! [`crate::choice_collect::SlotOptions`], put them on the
//! [`crate::battle_input::InputBus`], read the answers back as choice
//! strings, step its engine, and narrate what happened as
//! [`crate::board_event::BoardEvent`]s. Everything a player EXPERIENCES while
//! choosing — validation, unready, the both-ready grace, the robot's picks —
//! lives in `choice_collect`, once, for every generation.
//!
//! Which is worth saying out loud, because it was not always true: Gen 3
//! arrived with its own turn loop, its own prompt type and its own pad
//! plumbing, and every behaviour that parallel stack skipped came back as a
//! bug report.
//!
//! Two engines sit behind the same entry points
//! (`DeviceRunner::run_gen1` / `run_gen3`), picked per battle by
//! [`Engine`]: the legacy per-generation engines ([`gen1`], [`gen3`]), and
//! the Showdown split's server and client, one runner for both generations
//! ([`ps`], over a [`crate::transport::Transport`]).

pub mod gen1;
pub mod gen3;
pub mod ps;

/// Which battle engine plays a battle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Engine {
    /// `gen1_battle` / `gen3_battle` through their own runners.
    #[default]
    Legacy,
    /// `ps_sim` and `ps_client` through [`ps::run_battle`].
    Ps,
}

impl Engine {
    /// What the device plays when its platform does not say: `Legacy`,
    /// or `Ps` in a build with the core feature `ps_engine` (the
    /// firmware's `ps_engine`).
    pub const DEVICE_DEFAULT: Engine = if cfg!(feature = "ps_engine") { Engine::Ps } else { Engine::Legacy };

    /// The `engine=` URL / console parameter: `ps` or `legacy`.
    pub fn parse(s: &str) -> Option<Engine> {
        match s {
            "ps" => Some(Engine::Ps),
            "legacy" => Some(Engine::Legacy),
            _ => None,
        }
    }
}
