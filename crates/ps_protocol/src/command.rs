//! Input commands (PS `BattleStream` writes) and the formats they name.
//!
//! PS: `sim/battle-stream.ts` (`_writeLine`), `sim/side.ts` (`choose`,
//! `chooseMove`, `chooseSwitch`, `choosePass`), `sim/SIM-PROTOCOL.md`
//! "Sending decisions", `data/rulesets.ts`, `config/formats.ts`.

pub use crate::ident::SideId;
use crate::team::PackedTeam;
use crate::{BoundedVec, Name};

/// One `BattleStream` input line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `>start {"formatid":"gen3customgame","seed":[a,b,c,d]}`. A numeric
    /// seed of four 16-bit integers selects PS's `Gen5RNG`.
    Start { format: FormatSpec, seed: [u16; 4] },
    /// `>player p1 {"name":"NAME","team":"PACKED"}` (`avatar` and `rating`
    /// keys follow `name` when non-empty, as PS's `setPlayer` options).
    Player { side: SideId, name: Name, avatar: Name, rating: Name, team: PackedTeam },
    /// `>p1 CHOICE`
    Choose { side: SideId, choice: Choice },
    /// `>forcewin p1` (debug only).
    ForceWin(SideId),
    /// `>forcetie` (debug only).
    ForceTie,
}

/// A singles choice, PS grammar. Indices are 1-based as written on the wire
/// (`move 1` is the first move slot, `switch 2` the second Pokemon in the
/// request's `side.pokemon` order); the server validates them as PS does, so
/// an out-of-range index is representable and gets PS's error.
///
/// Not covered: choices by name (`move thunderbolt`, `switch Pikachu`),
/// targets and gimmick suffixes (doubles, mega, Z, max, tera), `team`, `shift`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Choice {
    /// `move N`
    Move(u8),
    /// `switch N`
    Switch(u8),
    /// `pass`
    Pass,
    /// `default`
    Default,
    /// `undo`
    Undo,
}

/// A base format id, PS `config/formats.ts`. Only the two the harness uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FormatId {
    Gen1CustomGame,
    Gen3CustomGame,
}

impl FormatId {
    pub const ALL: [FormatId; 2] = [FormatId::Gen1CustomGame, FormatId::Gen3CustomGame];

    /// The format id (`gen1customgame`).
    pub const fn id(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "gen1customgame",
            FormatId::Gen3CustomGame => "gen3customgame",
        }
    }

    /// The format name, as `|tier|` writes it.
    pub const fn name(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "[Gen 1] Custom Game",
            FormatId::Gen3CustomGame => "[Gen 3] Custom Game",
        }
    }

    pub const fn gen(self) -> u8 {
        match self {
            FormatId::Gen1CustomGame => 1,
            FormatId::Gen3CustomGame => 3,
        }
    }

    pub fn parse(id: &str) -> Option<FormatId> {
        FormatId::ALL.into_iter().find(|f| f.id() == id)
    }
}

/// A format rule that affects the battle (design section 3: the rule tables
/// PS resolves for our two formats). Gen 1 custom game: HP Percentage Mod,
/// Cancel Mod, Desync Clause Mod, plus the harness's Sleep Clause Mod and
/// Freeze Clause Mod; Gen 3 custom game: HP Percentage Mod, Cancel Mod.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum RuleId {
    HpPercentageMod,
    CancelMod,
    DesyncClauseMod,
    SleepClauseMod,
    FreezeClauseMod,
}

impl RuleId {
    pub const ALL: [RuleId; 5] = [
        RuleId::HpPercentageMod,
        RuleId::CancelMod,
        RuleId::DesyncClauseMod,
        RuleId::SleepClauseMod,
        RuleId::FreezeClauseMod,
    ];

    /// The rule's name, as a custom rule after `@@@` and in the infobox.
    pub const fn name(self) -> &'static str {
        match self {
            RuleId::HpPercentageMod => "HP Percentage Mod",
            RuleId::CancelMod => "Cancel Mod",
            RuleId::DesyncClauseMod => "Desync Clause Mod",
            RuleId::SleepClauseMod => "Sleep Clause Mod",
            RuleId::FreezeClauseMod => "Freeze Clause Mod",
        }
    }

    /// The text of the rule's `|rule|` line (its `onBegin` in
    /// `data/rulesets.ts`), or `None` for a rule that announces nothing.
    pub const fn line(self) -> Option<&'static str> {
        match self {
            RuleId::HpPercentageMod => Some("HP Percentage Mod: HP is shown in percentages"),
            RuleId::CancelMod => None,
            RuleId::DesyncClauseMod => Some(
                "Desync Clause Mod: Desyncs resolve to link battle behavior from the acting player's perspective.",
            ),
            RuleId::SleepClauseMod => Some("Sleep Clause Mod: Limit one foe put to sleep"),
            RuleId::FreezeClauseMod => Some("Freeze Clause Mod: Limit one foe frozen"),
        }
    }

    pub fn parse(name: &str) -> Option<RuleId> {
        RuleId::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// Most custom rules after `@@@`.
pub const MAX_CUSTOM_RULES: usize = 4;

/// A `formatid` value: a base format and its custom rules, in the order
/// given (`gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormatSpec {
    pub base: FormatId,
    pub custom_rules: BoundedVec<RuleId, MAX_CUSTOM_RULES>,
}

impl Default for RuleId {
    fn default() -> Self {
        RuleId::HpPercentageMod
    }
}

impl FormatSpec {
    pub fn new(base: FormatId) -> Self {
        Self { base, custom_rules: BoundedVec::new() }
    }

    /// The gen 1 format the harness plays (`formatFor(1)` in
    /// `tools/showdown-parity/harness.js`).
    pub fn gen1_harness() -> Self {
        let mut f = Self::new(FormatId::Gen1CustomGame);
        // Two rules into a capacity of four cannot fail.
        let _ = f.custom_rules.push(RuleId::SleepClauseMod);
        let _ = f.custom_rules.push(RuleId::FreezeClauseMod);
        f
    }

    /// The gen 3 format the harness plays.
    pub fn gen3_harness() -> Self {
        Self::new(FormatId::Gen3CustomGame)
    }

    /// Calls `f` with the pieces of the `formatid` string in order: the base
    /// id, then (if any custom rules) `@@@` and the names joined by `,`.
    pub fn write_id(&self, mut f: impl FnMut(&str)) {
        f(self.base.id());
        for (i, r) in self.custom_rules.iter().enumerate() {
            f(if i == 0 { "@@@" } else { "," });
            f(r.name());
        }
    }

    /// Parses a `formatid` string.
    pub fn parse(s: &str) -> Option<FormatSpec> {
        let (base, rules) = match s.split_once("@@@") {
            Some((b, r)) => (b, Some(r)),
            None => (s, None),
        };
        let mut spec = FormatSpec::new(FormatId::parse(base)?);
        if let Some(rules) = rules {
            for name in rules.split(',') {
                spec.custom_rules.push(RuleId::parse(name)?).ok()?;
            }
        }
        Some(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn id_of(spec: &FormatSpec) -> String {
        let mut s = String::new();
        spec.write_id(|p| s.push_str(p));
        s
    }

    #[test]
    fn harness_formats_print_and_parse() {
        let g1 = FormatSpec::gen1_harness();
        assert_eq!(id_of(&g1), "gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod");
        assert_eq!(FormatSpec::parse(&id_of(&g1)), Some(g1));
        let g3 = FormatSpec::gen3_harness();
        assert_eq!(id_of(&g3), "gen3customgame");
        assert_eq!(FormatSpec::parse("gen3customgame"), Some(g3));
        assert_eq!(FormatSpec::parse("gen9ou"), None);
        assert_eq!(FormatSpec::parse("gen1customgame@@@Species Clause"), None);
        assert_eq!(g1.base.name(), "[Gen 1] Custom Game");
        assert_eq!(g1.base.gen(), 1);
    }

    #[test]
    fn rule_lines() {
        assert_eq!(RuleId::CancelMod.line(), None);
        for r in RuleId::ALL {
            assert_eq!(RuleId::parse(r.name()), Some(r));
            if let Some(line) = r.line() {
                assert!(line.starts_with(r.name()));
            }
        }
    }

    #[test]
    fn command_constructors() {
        let c = Command::Choose { side: SideId::P2, choice: Choice::Move(1) };
        assert_eq!(c, Command::Choose { side: SideId::P2, choice: Choice::Move(1) });
        let s = Command::Start { format: FormatSpec::gen3_harness(), seed: [1, 2, 3, 4] };
        match s {
            Command::Start { format, seed } => {
                assert_eq!(format.base, FormatId::Gen3CustomGame);
                assert_eq!(seed, [1, 2, 3, 4]);
            }
            _ => unreachable!(),
        }
    }
}
