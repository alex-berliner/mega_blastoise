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
    /// `>start {"formatid":"gen3customgame","seed":[a,b,c,d]}`, or with a
    /// seed string (`"a,b,c,d"`, `"gen5,<hex>"`, `"sodium,<hex>"`) as PS
    /// 0.11.10 and later write it. See [`PrngSeed`].
    Start { format: FormatSpec, seed: PrngSeed },
    /// `>player p1 {"name":"NAME","team":"PACKED"}`.
    Player { side: SideId, name: Name, team: PackedTeam },
    /// `>p1 CHOICE`.
    Choose { side: SideId, choice: Choice },
    /// `>forcewin p1` (debug only).
    ForceWin(SideId),
    /// `>forcetie` (debug only).
    ForceTie,
}

/// A battle PRNG seed, the forms PS's `PRNG.setSeed` reads.
///
/// | `>start` seed | PS | variant |
/// |---|---|---|
/// | `[a,b,c,d]` (older inputlogs; joined with `,` first) | `Gen5RNG` | [`PrngSeed::Gen5`] |
/// | `"a,b,c,d"` (a string starting with a digit) | `Gen5RNG` | [`PrngSeed::Gen5`] |
/// | `"gen5,<16 hex digits>"` | `Gen5RNG` | [`PrngSeed::Gen5`] |
/// | `"sodium,<hex>"` | `SodiumRNG` | [`PrngSeed::Sodium`] |
///
/// The encoder writes a Gen5 seed as the array (the harness's form) and a
/// sodium seed as `sodium,` plus 64 hex digits (`SodiumRNG.getSeed`'s
/// form; PS pads a shorter seed with `0` to the same key).
///
/// PS: sim/prng.ts `PRNG` constructor, `PRNG.setSeed`, `SodiumRNG.setSeed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrngSeed {
    /// Four 16-bit integers, most significant first.
    Gen5([u16; 4]),
    /// The 32-byte ChaCha20 key: the hex right-padded with `0` to 64
    /// digits, two digits a byte.
    Sodium([u8; 32]),
}

impl From<[u16; 4]> for PrngSeed {
    fn from(seed: [u16; 4]) -> PrngSeed {
        PrngSeed::Gen5(seed)
    }
}

impl PrngSeed {
    /// Parses a seed string as `PRNG.setSeed` does: `sodium,<hex>`,
    /// `gen5,<16 hex>` or digits `a,b,c,d`. Narrower than PS where PS would
    /// read garbage: a hex digit that is not one, more than 64 sodium
    /// digits, or a Gen5 part that is not a 16-bit integer is `None`.
    pub fn parse(s: &str) -> Option<PrngSeed> {
        if let Some(hex) = s.strip_prefix("sodium,") {
            // `seed.split(',')[1]`: anything after a second comma is
            // ignored.
            let hex = hex.split(',').next().unwrap_or("");
            if hex.len() > 64 {
                return None;
            }
            let mut key = [0u8; 32];
            for (i, b) in hex.bytes().enumerate() {
                let d = (b as char).to_digit(16)? as u8;
                key[i / 2] |= if i % 2 == 0 { d << 4 } else { d };
            }
            return Some(PrngSeed::Sodium(key));
        }
        if let Some(hex) = s.strip_prefix("gen5,") {
            // `seed.slice(5, 9)` ... `seed.slice(17, 21)`.
            if hex.len() != 16 {
                return None;
            }
            let mut out = [0u16; 4];
            for (i, o) in out.iter_mut().enumerate() {
                *o = u16::from_str_radix(hex.get(i * 4..i * 4 + 4)?, 16).ok()?;
            }
            return Some(PrngSeed::Gen5(out));
        }
        if !s.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        // `seed.split(',').map(Number)`.
        let mut out = [0u16; 4];
        let mut parts = s.split(',');
        for o in out.iter_mut() {
            *o = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        Some(PrngSeed::Gen5(out))
    }
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
    /// `[Gen 3] Random Battle`, the same for gen 3.
    Gen3RandomBattle,
}

impl FormatId {
    pub const ALL: [FormatId; 4] =
        [FormatId::Gen1CustomGame, FormatId::Gen3CustomGame, FormatId::Gen1RandomBattle, FormatId::Gen3RandomBattle];

    /// The format id (`gen1customgame`).
    pub const fn id(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "gen1customgame",
            FormatId::Gen3CustomGame => "gen3customgame",
            FormatId::Gen1RandomBattle => "gen1randombattle",
            FormatId::Gen3RandomBattle => "gen3randombattle",
        }
    }

    /// The format name, as `|tier|` writes it.
    pub const fn name(self) -> &'static str {
        match self {
            FormatId::Gen1CustomGame => "[Gen 1] Custom Game",
            FormatId::Gen3CustomGame => "[Gen 3] Custom Game",
            FormatId::Gen1RandomBattle => "[Gen 1] Random Battle",
            FormatId::Gen3RandomBattle => "[Gen 3] Random Battle",
        }
    }

    pub const fn gen(self) -> u8 {
        match self {
            FormatId::Gen1CustomGame | FormatId::Gen1RandomBattle => 1,
            FormatId::Gen3CustomGame | FormatId::Gen3RandomBattle => 3,
        }
    }

    /// The format's `debug` flag: the custom games have it, random battles
    /// do not.
    pub const fn debug(self) -> bool {
        !matches!(self, FormatId::Gen1RandomBattle | FormatId::Gen3RandomBattle)
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
/// Gen 3 random battle (gen 3's `Standard`) also announces Beat Up Nicknames
/// Mod, Switch Priority Clause Mod and Evasion Items Clause.
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
    BeatUpNicknamesMod,
    SwitchPriorityClauseMod,
    EvasionItemsClause,
    /// Our named deviation (design section 3): Gen 1's bit-width `trunc`
    /// (`Dex.trunc`) in place of `Math.trunc`. Every Gen 1 format has it in
    /// its ruleset; `!Cartridge Trunc` after `@@@` turns it off.
    CartridgeTrunc,
}

impl RuleId {
    pub const ALL: [RuleId; 13] = [
        RuleId::HpPercentageMod,
        RuleId::CancelMod,
        RuleId::DesyncClauseMod,
        RuleId::SleepClauseMod,
        RuleId::FreezeClauseMod,
        RuleId::EndlessBattleClause,
        RuleId::SpeciesClause,
        RuleId::OhkoClause,
        RuleId::EvasionMovesClause,
        RuleId::BeatUpNicknamesMod,
        RuleId::SwitchPriorityClauseMod,
        RuleId::EvasionItemsClause,
        RuleId::CartridgeTrunc,
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
            RuleId::BeatUpNicknamesMod => "Beat Up Nicknames Mod",
            RuleId::SwitchPriorityClauseMod => "Switch Priority Clause Mod",
            RuleId::EvasionItemsClause => "Evasion Items Clause",
            RuleId::CartridgeTrunc => "Cartridge Trunc",
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
            RuleId::SpeciesClause => Some("Species Clause: Limit one of each Pokemon"),
            RuleId::OhkoClause => Some("OHKO Clause: OHKO moves are banned"),
            RuleId::EvasionMovesClause => Some("Evasion Moves Clause: Evasion moves are banned"),
            // `onBegin` announces it only in gens 1 to 4, which are all ours.
            RuleId::BeatUpNicknamesMod => Some("Beat Up Nicknames Mod: Beat Up will not reveal any party members"),
            RuleId::SwitchPriorityClauseMod => Some("Switch Priority Clause Mod: Faster Pokemon switch first"),
            RuleId::EvasionItemsClause => Some("Evasion Items Clause: Evasion items are banned"),
            // It has no `onBegin` text (`tools/ps-diff/rules.js`).
            RuleId::CartridgeTrunc => None,
        }
    }

    pub fn parse(name: &str) -> Option<RuleId> {
        RuleId::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// Most custom rules after `@@@`.
pub const MAX_CUSTOM_RULES: usize = 4;

/// One custom rule after `@@@`: a rule added to the format's ruleset, or
/// (`!Name`) one repealed from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CustomRule {
    pub rule: RuleId,
    /// `!Name`: PS's rule repeal (`sim/dex-formats.ts getRuleTable`).
    pub repeal: bool,
}

impl CustomRule {
    pub const fn add(rule: RuleId) -> Self {
        Self { rule, repeal: false }
    }

    pub const fn repeal(rule: RuleId) -> Self {
        Self { rule, repeal: true }
    }

    /// Calls `f` with the rule as written after `@@@` and in the custom
    /// rules infobox: `!` for a repeal, then the name.
    pub fn write(&self, mut f: impl FnMut(&str)) {
        if self.repeal {
            f("!");
        }
        f(self.rule.name());
    }

    pub fn parse(s: &str) -> Option<CustomRule> {
        match s.strip_prefix('!') {
            // `!!` repeals and replaces a value rule; none of ours has a value.
            Some(name) if !name.starts_with('!') => Some(CustomRule::repeal(RuleId::parse(name)?)),
            Some(_) => None,
            None => Some(CustomRule::add(RuleId::parse(s)?)),
        }
    }
}

/// A `formatid` value: a base format and its custom rules, in the order
/// given (`gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormatSpec {
    pub base: FormatId,
    pub custom_rules: BoundedVec<CustomRule, MAX_CUSTOM_RULES>,
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
    /// `tools/showdown-parity/harness.js`). `Cartridge Trunc` is in every
    /// Gen 1 format's ruleset, so it is on.
    pub fn gen1_harness() -> Self {
        let mut f = Self::new(FormatId::Gen1CustomGame);
        // Two rules into a capacity of four cannot fail.
        let _ = f.custom_rules.push(CustomRule::add(RuleId::SleepClauseMod));
        let _ = f.custom_rules.push(CustomRule::add(RuleId::FreezeClauseMod));
        f
    }

    /// The gen 3 format the harness plays.
    pub fn gen3_harness() -> Self {
        Self::new(FormatId::Gen3CustomGame)
    }

    /// The same format with `rule` repealed (`@@@...,!Name`); `None` when
    /// the custom rules are full.
    pub fn with_repeal(mut self, rule: RuleId) -> Option<Self> {
        self.custom_rules.push(CustomRule::repeal(rule)).ok()?;
        Some(self)
    }

    /// Calls `f` with the pieces of the `formatid` string in order: the base
    /// id, then (if any custom rules) `@@@` and the rules joined by `,`.
    pub fn write_id(&self, mut f: impl FnMut(&str)) {
        f(self.base.id());
        for (i, r) in self.custom_rules.iter().enumerate() {
            f(if i == 0 { "@@@" } else { "," });
            r.write(&mut f);
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
                spec.custom_rules.push(CustomRule::parse(name)?).ok()?;
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
        let math = g1.with_repeal(RuleId::CartridgeTrunc).unwrap();
        assert_eq!(id_of(&math), "gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod,!Cartridge Trunc");
        assert_eq!(FormatSpec::parse(&id_of(&math)), Some(math));
        let rb = FormatSpec::parse("gen1randombattle@@@!Cartridge Trunc").unwrap();
        assert_eq!(rb.custom_rules[0], CustomRule::repeal(RuleId::CartridgeTrunc));
        assert_eq!(FormatSpec::parse("gen1customgame@@@!!Cartridge Trunc"), None);
        assert_eq!(FormatSpec::parse("gen1customgame@@@! Cartridge Trunc"), None);
        assert_eq!(FormatSpec::parse("gen1randombattle").map(|f| f.base.name()), Some("[Gen 1] Random Battle"));
        assert_eq!(FormatSpec::parse("gen3randombattle").map(|f| f.base.gen()), Some(3));
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
        let s = Command::Start { format: FormatSpec::gen3_harness(), seed: PrngSeed::Gen5([1, 2, 3, 4]) };
        match s {
            Command::Start { format, seed } => {
                assert_eq!(format.base, FormatId::Gen3CustomGame);
                assert_eq!(seed, PrngSeed::Gen5([1, 2, 3, 4]));
            }
            _ => unreachable!(),
        }
        let nm = Choice::MoveName(Name::from_str("thunderbolt").unwrap());
        let sn = Choice::SwitchName(Name::from_str("raichu").unwrap());
        assert_eq!(nm, Choice::MoveName(Name::from_str("thunderbolt").unwrap()));
        assert_ne!(nm, sn);
    }
}
