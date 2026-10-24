//! Input commands (PS `BattleStream` writes) and the formats they name.
//!
//! PS: `sim/battle-stream.ts` (`_writeLine`), `sim/side.ts` (`choose`,
//! `chooseMove`, `chooseSwitch`, `choosePass`), `sim/SIM-PROTOCOL.md`
//! "Sending decisions", `data/rulesets.ts`, `config/formats.ts`.

pub use crate::ident::SideId;
use crate::team::PackedTeam;
use crate::{BoundedVec, Name};

/// One `BattleStream` input line.
///
/// PS's `setPlayer` reads four keys of a `>player` line: `name`, `team`
/// and, when a server sends them, `avatar` and `rating`. The lines PS
/// records (and the ones the device and the harness write) carry only
/// `name` and `team`, in that order, so [`Command::Player`] holds only
/// those two: the encoder writes exactly the keys PS's lines have, in
/// PS's order, and a line with extras decodes to the same [`Command`] as
/// one without.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `>start {"formatid":"gen3customgame","seed":[a,b,c,d]}`. A numeric
    /// seed of four 16-bit integers selects PS's `Gen5RNG`.
    Start { format: FormatSpec, seed: [u16; 4] },
    /// `>player p1 {"name":"NAME","team":"PACKED"}`.
    Player { side: SideId, name: Name, team: PackedTeam },
    /// `>p1 CHOICE`.
    Choose { side: SideId, choice: Choice },
    /// `>forcewin p1` (debug only).
    ForceWin(SideId),
    /// `>forcetie` (debug only).
    ForceTie,
}

/// One choice, PS's `side.choose` grammar.
///
/// The wire form is `move N`, `move NAME`, `switch N`, `switch NAME`,
/// `pass`, `default` or `undo`. The numbers are 1-based, as PS writes
/// them: `move 1` is the first move slot of the request, `switch 2` the
/// second Pokemon of the request's `side.pokemon`. PS validates them
/// itself and answers with its own error when they are out of range, so
/// the codec passes them on as written (`move 0` is PS's "you don't have
/// a move 0", not a codec error).
///
/// PS takes a name instead of a number for both moves (`move thunderbolt`)
/// and switches (`switch pikachu`), and resolves it at choice time:
/// `chooseMove` looks the name up in the request's `moves` (by the move's
/// id, so case and punctuation do not matter), `chooseSwitch` in the
/// team's Pokemon (by nickname, case-insensitively, or by species id).
/// The codec keeps the form the line carried, so a line round-trips byte
/// for byte.
///
/// Not covered: the `,` that joins several choices (doubles), the target
/// of a move (`move 1 +1`), the gimmick suffixes (`mega`, `zmove`,
/// `dynamax`, `terastallize`), `team` (Team Preview), `shift`, and PS's
/// synonyms `skip` (for `pass`) and `auto` (for `default`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Choice {
    /// `move N`: the Nth move slot of the request.
    Move(u8),
    /// `move NAME`: the move whose id is NAME.
    MoveName(Name),
    /// `switch N`: the Nth Pokemon of the request's `side.pokemon`.
    Switch(u8),
    /// `switch NAME`: the Pokemon named NAME (its nickname or its species).
    SwitchName(Name),
    /// `pass`.
    Pass,
    /// `default`: PS's `autoChoose`.
    Default,
    /// `undo`: PS's `undoChoice`.
    Undo,
}

/// A base format id, PS `config/formats.ts`. Only the ones the harnesses use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FormatId {
    Gen1CustomGame,
    Gen3CustomGame,
    /// `[Gen 1] Random Battle`: the protocol-diff harness's `--teams random`
    /// (it supplies the teams, so only the ruleset differs).
    Gen1RandomBattle,
}

impl FormatId {
    pub const ALL: [FormatId; 3] = [FormatId::Gen1CustomGame, FormatId::Gen3CustomGame, FormatId::Gen1RandomBattle];

    /// The format id (`gen1customgame`).
    pub const fn id(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "gen1customgame",
            FormatId::Gen3CustomGame => "gen3customgame",
            FormatId::Gen1RandomBattle => "gen1randombattle",
        }
    }

    /// The format name, as `|tier|` writes it.
    pub const fn name(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "[Gen 1] Custom Game",
            FormatId::Gen3CustomGame => "[Gen 3] Custom Game",
            FormatId::Gen1RandomBattle => "[Gen 1] Random Battle",
        }
    }

    pub const fn gen(self) -> u8 {
        match self {
            FormatId::Gen1CustomGame | FormatId::Gen1RandomBattle => 1,
            FormatId::Gen3CustomGame => 3,
        }
    }

    /// The format's `debug` flag: the custom games have it, random battles
    /// do not.
    pub const fn debug(self) -> bool {
        !matches!(self, FormatId::Gen1RandomBattle)
    }

    pub fn parse(id: &str) -> Option<FormatId> {
        FormatId::ALL.into_iter().find(|f| f.id() == id)
    }
}

/// A format rule that affects the battle (design section 3: the rule tables
/// PS resolves for our two formats). Gen 1 custom game: HP Percentage Mod,
/// Cancel Mod, Desync Clause Mod, plus the harness's Sleep Clause Mod and
/// Freeze Clause Mod; Gen 3 custom game: HP Percentage Mod, Cancel Mod.
/// Gen 1 random battle (`Standard`) adds Endless Battle Clause and the
/// validator clauses that announce themselves (Species, OHKO, Evasion Moves).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum RuleId {
    HpPercentageMod,
    CancelMod,
    DesyncClauseMod,
    SleepClauseMod,
    FreezeClauseMod,
    EndlessBattleClause,
    SpeciesClause,
    OhkoClause,
    EvasionMovesClause,
}

impl RuleId {
    pub const ALL: [RuleId; 9] = [
        RuleId::HpPercentageMod,
        RuleId::CancelMod,
        RuleId::DesyncClauseMod,
        RuleId::SleepClauseMod,
        RuleId::FreezeClauseMod,
        RuleId::EndlessBattleClause,
        RuleId::SpeciesClause,
        RuleId::OhkoClause,
        RuleId::EvasionMovesClause,
    ];

    /// The rule's name, as a custom rule after `@@@` and in the infobox.
    pub const fn name(self) -> &'static str {
        match self {
            RuleId::HpPercentageMod => "HP Percentage Mod",
            RuleId::CancelMod => "Cancel Mod",
            RuleId::DesyncClauseMod => "Desync Clause Mod",
            RuleId::SleepClauseMod => "Sleep Clause Mod",
            RuleId::FreezeClauseMod => "Freeze Clause Mod",
            RuleId::EndlessBattleClause => "Endless Battle Clause",
            RuleId::SpeciesClause => "Species Clause",
            RuleId::OhkoClause => "OHKO Clause",
            RuleId::EvasionMovesClause => "Evasion Moves Clause",
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
            RuleId::EndlessBattleClause => Some("Endless Battle Clause: Forcing endless battles is banned"),
            RuleId::SpeciesClause => Some("Species Clause: Limit one of each Pok\u{e9}mon"),
            RuleId::OhkoClause => Some("OHKO Clause: OHKO moves are banned"),
            RuleId::EvasionMovesClause => Some("Evasion Moves Clause: Evasion moves are banned"),
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
        assert_eq!(FormatSpec::parse("gen1customgame@@@Team Preview"), None);
        assert_eq!(FormatSpec::parse("gen1randombattle").map(|f| f.base.name()), Some("[Gen 1] Random Battle"));
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
        let nm = Choice::MoveName(Name::from_str("thunderbolt").unwrap());
        let sn = Choice::SwitchName(Name::from_str("raichu").unwrap());
        assert_eq!(nm, Choice::MoveName(Name::from_str("thunderbolt").unwrap()));
        assert_ne!(nm, sn);
    }
}
