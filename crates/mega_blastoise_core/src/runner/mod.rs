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
//! [`Engine`]: the Showdown split's server and client, one runner for both
//! generations ([`ps`], over a [`crate::transport::Transport`]), and the
//! legacy per-generation engines (`gen1`, only with the `legacy_gen1`
//! feature, and [`gen3`], until `ps_sim` ports Gen 3).

#[cfg(feature = "legacy_gen1")]
pub mod gen1;
pub mod gen3;
pub mod ps;
#[cfg(test)]
mod forced;

/// Idle delay (ms) at lobby start before the demo battle begins automatically.
pub const LOBBY_DEMO_DELAY_MS: u64 = 15_000;

/// Which battle engine plays a battle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Engine {
    /// `ps_sim` and `ps_client` through [`ps::run_battle`], for every
    /// generation [`ps_ports_every_move`] holds for; Gen 3 plays on its
    /// legacy engine until then.
    #[default]
    Ps,
    /// `gen1_battle` / `gen3_battle` through their own runners. Gen 1
    /// needs the `legacy_gen1` feature; without it Gen 1 plays on `Ps`.
    Legacy,
}

impl Engine {
    /// What the device plays when its platform does not say.
    pub const DEVICE_DEFAULT: Engine = Engine::Ps;

    /// The `engine=` URL / console parameter: `ps` or `legacy`.
    pub fn parse(s: &str) -> Option<Engine> {
        match s {
            "ps" => Some(Engine::Ps),
            "legacy" => Some(Engine::Legacy),
            _ => None,
        }
    }
}

/// True once `ps_sim` ports every move of `gen`'s dex a team can carry
/// (`ps_sim::mods::ported`; like its own derivation, Struggle and the
/// Hidden Power placeholders are not team moves). Until then a PS battle of that generation
/// could only use the moves already ported (`randbat::draw_ps_team` drops
/// the rest), so the generation keeps playing on its legacy engine.
pub fn ps_ports_every_move(gen: u8) -> bool {
    let dex: &ps_dex::Dex = match gen {
        1 => &ps_dex::GEN1,
        3 => &ps_dex::GEN3,
        _ => return false,
    };
    let ported = ps_sim::mods::ported(gen);
    dex.moves.iter().filter(|m| m.placeholder_for.is_none()).all(|m| {
        let id = ps_dex::text::IdBuf::from_name(m.name.as_str());
        id.as_str() == "struggle"
            || ported.iter().any(|e| e.strip_prefix("move:") == Some(id.as_str()))
    })
}

#[cfg(test)]
mod tests {
    /// Gen 1 plays on the PS engine: every one of its moves is ported.
    #[test]
    fn gen1_is_fully_ported() {
        assert!(super::ps_ports_every_move(1));
    }
}
