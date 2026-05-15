//! Narration events. Formatted to strings on push (in `dispatch.rs`).

use core::fmt;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    MoveUsed { side: u8, move_id: &'static str },
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
    FullyParalyzed { side: u8 },
    ConfusionSelfHit { side: u8, dealt: u16 },
    Recharge { side: u8 },
    SwitchIn { side: u8, slot: u8 },
    Win { side: u8 },
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side_name = |s: u8| if s == 0 { "Red" } else { "Blue" };
        match self {
            Event::MoveUsed { side, move_id } => write!(f, "{} used {}!", side_name(*side), move_id),
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
            Event::FullyParalyzed { side } => write!(f, "{} is fully paralyzed!", side_name(*side)),
            Event::ConfusionSelfHit { side, dealt } => write!(f, "{} hurt itself in confusion ({} dmg)", side_name(*side), dealt),
            Event::Recharge { side } => write!(f, "{} must recharge!", side_name(*side)),
            Event::SwitchIn { side, slot } => write!(f, "{} sent out slot {}!", side_name(*side), slot + 1),
            Event::Win { side } => write!(f, "{} wins!", side_name(*side)),
        }
    }
}
