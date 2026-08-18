//! One runner per generation, each driving its own engine over the SAME seat
//! protocol.
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

pub mod gen1;
pub mod gen3;
