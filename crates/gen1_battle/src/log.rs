//! Narration events. Formatted to strings on push (in `dispatch.rs`).

use core::fmt;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Event {
    MoveUsed { side: u8, move_id: &'static str },
    MoveDisabled { side: u8, move_id: &'static str },
    Damage { side: u8, dealt: u16 },
    Heal { side: u8, amount: u16 },
    Miss { side: u8 },
    Crit { side: u8 },
    Immune,
    NoEffect,
    Failed,
    Faint { side: u8 },
    StatusInflicted { side: u8 },
    StatChanged { side: u8, stat: u8, delta: i8 },
    Wake { side: u8 },
    Frozen { side: u8 },
    FullyParalyzed { side: u8 },
    ConfusionSelfHit { side: u8, dealt: u16 },
    Recharge { side: u8 },
    SwitchIn { side: u8, slot: u8 },
    Win { side: u8 },
    Charging { side: u8, move_id: &'static str },
    BideStoring { side: u8 },
    BideUnleash { side: u8 },
    Mimicked { side: u8, move_id: &'static str },
    Transformed { side: u8 },
    Substitute { side: u8 },
    SubstituteBroken { side: u8 },
    Disabled { side: u8, move_slot: u8 },
    DisableEnded { side: u8 },
    Wrap { side: u8, turns: u8 },
    CantMoveTrapped { side: u8 },
    LeechSeed { side: u8 },
    ScreenUp { side: u8, kind: u8 }, // 0=Reflect 1=LightScreen
    MistOn { side: u8 },
    FocusEnergyOn { side: u8 },
    Converted { side: u8 },
    Haze,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side_name = |s: u8| if s == 0 { "White" } else { "Red" };
        match self {
            Event::MoveUsed { side, move_id } => write!(f, "{} used {}!", side_name(*side), move_id),
            Event::MoveDisabled { side, move_id } => write!(f, "{}'s {} is disabled!", side_name(*side), move_id),
            Event::Damage { side, dealt } => write!(f, "{} took {} damage", side_name(*side), dealt),
            Event::Heal { side, amount } => write!(f, "{} healed {} HP", side_name(*side), amount),
            Event::Miss { side } => write!(f, "{}'s attack missed!", side_name(*side)),
            Event::Crit { side } => write!(f, "Critical hit by {}!", side_name(*side)),
            Event::Immune => write!(f, "It had no effect."),
            Event::NoEffect => write!(f, "But it had no effect."),
            Event::Failed => write!(f, "But it failed!"),
            Event::Faint { side } => write!(f, "{} fainted!", side_name(*side)),
            Event::StatusInflicted { side } => write!(f, "{} was afflicted!", side_name(*side)),
            Event::StatChanged { side, stat, delta } => {
                let arrow = if *delta > 0 { "rose" } else { "fell" };
                write!(f, "{}'s stat {} {}", side_name(*side), stat, arrow)
            }
            Event::Wake { side } => write!(f, "{} woke up!", side_name(*side)),
            Event::Frozen { side } => write!(f, "{} is frozen solid!", side_name(*side)),
            Event::FullyParalyzed { side } => write!(f, "{} is fully paralyzed!", side_name(*side)),
            Event::ConfusionSelfHit { side, dealt } => write!(f, "{} hurt itself in confusion ({} dmg)", side_name(*side), dealt),
            Event::Recharge { side } => write!(f, "{} must recharge!", side_name(*side)),
            Event::SwitchIn { side, slot } => write!(f, "{} sent out slot {}!", side_name(*side), slot + 1),
            Event::Win { side } => write!(f, "{} wins!", side_name(*side)),
            Event::Charging { side, move_id } => write!(f, "{} is charging {}!", side_name(*side), move_id),
            Event::BideStoring { side } => write!(f, "{} is storing energy!", side_name(*side)),
            Event::BideUnleash { side } => write!(f, "{} unleashed Bide!", side_name(*side)),
            Event::Mimicked { side, move_id } => write!(f, "{} mimicked {}!", side_name(*side), move_id),
            Event::Transformed { side } => write!(f, "{} transformed!", side_name(*side)),
            Event::Substitute { side } => write!(f, "{} put up a substitute!", side_name(*side)),
            Event::SubstituteBroken { side } => write!(f, "{}'s substitute broke!", side_name(*side)),
            Event::Disabled { side, move_slot } => write!(f, "{}'s move slot {} was disabled!", side_name(*side), move_slot + 1),
            Event::DisableEnded { side } => write!(f, "{}'s disable wore off!", side_name(*side)),
            Event::Wrap { side, turns } => write!(f, "{} wraps for {} more turn(s)!", side_name(*side), turns),
            Event::CantMoveTrapped { side } => write!(f, "{} is trapped!", side_name(*side)),
            Event::LeechSeed { side } => write!(f, "{} was seeded!", side_name(*side)),
            Event::ScreenUp { side, kind } => write!(f, "{} put up a {}!", side_name(*side), if *kind == 0 { "Reflect" } else { "Light Screen" }),
            Event::MistOn { side } => write!(f, "{} shrouded itself in mist!", side_name(*side)),
            Event::FocusEnergyOn { side } => write!(f, "{} is getting pumped!", side_name(*side)),
            Event::Converted { side } => write!(f, "{} converted its type!", side_name(*side)),
            Event::Haze => write!(f, "All stat changes were eliminated!"),
        }
    }
}
