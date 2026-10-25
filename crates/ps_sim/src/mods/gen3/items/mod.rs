//! Gen 3 items with handlers: one file per effect (or per shared
//! handler group; the file names match PS's ids so a `git grep` and a
//! `show-effect.js query` both fit).

pub mod flavor;
pub mod heal_berries;
pub mod leppa;
pub mod lum;
pub mod mentalherb;
pub mod persim;
pub mod stat_berries;
pub mod status_cure;
pub mod whiteherb;

#[cfg(test)]
mod tests;
