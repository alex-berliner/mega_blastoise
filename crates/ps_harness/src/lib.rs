//! std, test-only harness around `ps_sim`.
//!
//! Per `docs/architecture/showdown-split-plan.md` section 4:
//! `ps_sim_cli` runs the server over PS BattleStream text (the `text`
//! feature gates the codec). The driver itself is `tools/ps-diff/diff.js`,
//! which feeds identical command streams to PS's `BattleStream` and to
//! `ps_sim_cli` and diffs the output line by line.
//!
//! This crate starts empty in P1-0 (scaffold only); ps_sim_cli lands in
//! the same phase as the protocol-diff harness (P1-7).
