//! The ids the dex tables hand out.
//!
//! Every table is in PS's iteration order (`dex.moves.all()` and so on), and
//! an id is the index into its generation's table, so the same move has a
//! different id in gen 1 and gen 3. They are newtypes rather than bare
//! `u16` so a move id cannot be passed where a species id belongs. Each
//! generation's ids are also generated as constants named after PS's keys:
//! `ps_dex::gen1::moves::BIDE`, `ps_dex::gen3::abilities::INTIMIDATE`.

macro_rules! dex_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u16);

        impl $name {
            /// "No such entry": a move with no condition, a species with no
            /// second ability.
            pub const NONE: $name = $name(u16::MAX);

            /// The index into the generation's table.
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            pub const fn is_none(self) -> bool {
                self.0 == u16::MAX
            }
        }
    };
}

dex_id!(
    /// A move: the index into the generation's move table.
    MoveId
);
dex_id!(
    /// A species: the index into the generation's species table.
    SpeciesId
);
dex_id!(
    /// An item: the index into the generation's item table.
    ItemId
);
dex_id!(
    /// An ability: the index into the generation's ability table.
    AbilityId
);
dex_id!(
    /// A condition: the index into the generation's condition table, which
    /// holds PS's Conditions table and the `condition` of every move, ability
    /// and item (section "conditions" in `tools/ps-diff/dump-dex.js`).
    ConditionId
);
