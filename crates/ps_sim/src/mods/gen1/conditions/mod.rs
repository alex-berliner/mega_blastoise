//! Gen 1 conditions with handlers, one file per condition (statuses,
//! volatiles, and moves' own `condition`s, as `dex.conditions` finds them).

pub mod bide;
pub mod fakepartiallytrapped;
pub mod invulnerability;
pub mod lockedmove;
pub mod mustrecharge;
pub mod partiallytrapped;
pub mod partialtrappinglock;
pub mod rage;
pub mod trapped;
pub mod twoturnmove;
