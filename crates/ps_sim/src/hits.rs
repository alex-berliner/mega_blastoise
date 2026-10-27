//! Per-handler hit counts (feature `hit-count`, off by default and in the
//! firmware): every place the dispatcher calls a registered handler
//! (`runEvent`'s handler loop, `singleEvent`, the direct `*Callback`
//! calls) counts one call of that `(effect, prefix, event)` handler in
//! `Battle::hits`. A driver reads the battle's counts once it is over and
//! sums them over a sweep; `ps_sim_cli --hits` does, for
//! `scripts/ps-coverage.sh` (the V-1 coverage gate).
//!
//! The key is the handler as registered (its own prefix and event), not
//! the event that reached it: a gen 1-4 `onStart` run for `SwitchIn`
//! counts as `onStart`.

use alloc::collections::BTreeMap;

use crate::effect::{EffectId, EventId, Prefix};

/// One battle's handler calls.
#[derive(Clone, Debug, Default)]
pub struct Hits {
    pub calls: BTreeMap<(EffectId, Prefix, EventId), u32>,
}

impl Hits {
    pub const fn new() -> Hits {
        Hits { calls: BTreeMap::new() }
    }

    pub fn count(&mut self, effect: EffectId, prefix: Prefix, event: EventId) {
        *self.calls.entry((effect, prefix, event)).or_insert(0) += 1;
    }
}
