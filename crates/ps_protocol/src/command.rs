//! Input commands (PS `BattleStream` writes) and the formats they name.
//!
//! PS: `sim/battle-stream.ts` (`_writeLine`), `sim/side.ts` (`choose`,
//! `chooseMove`, `chooseSwitch`, `choosePass`), `sim/SIM-PROTOCOL.md`
//! "Sending decisions", `data/rulesets.ts`, `config/formats.ts`.

pub use crate::ident::SideId;
use crate::team::PackedTeam;
use crate::{BoundedStr, BoundedVec, Name};

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
/// `dynamax`, `terastallize`), `shift`, and PS's synonyms `skip` (for
/// `pass`) and `auto` (for `default`).
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
    /// `team DATA`: the team preview pick, `DATA` as written (`2134`,
    /// `2, 1`, `[1,2]`); the sim reads it (`chooseTeam`).
    Team(Name),
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
///
/// The custom rules public servers add to random battles (the replay
/// corpus's variant formats) follow: Best Of, Same Type Clause, Max Team
/// Size, Picked Team Size, Team Preview, Blitz, and the stat and move mods
/// (Scalemons, 350 Cup, Bad 'n Boosted, Guaranteed Secondary).
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
    /// `Best Of = N`: a series of N games. Validator only; nothing in one
    /// battle reads it.
    BestOf,
    /// `Same Type Clause`: validator only, announces itself.
    SameTypeClause,
    /// `Max Team Size = N`: validator only (the random team generator reads
    /// it; the battle takes the team it is given).
    MaxTeamSize,
    /// `Picked Team Size = N`: the battle asks for a team preview pick of N
    /// (`sim/battle.ts runPickTeam`, `sim/side.ts pickedTeamSize`).
    PickedTeamSize,
    /// `Team Preview`: the `|poke|` lines and the team preview request.
    TeamPreview,
    /// `Blitz`: a timer setting; announces itself.
    Blitz,
    /// `Scalemons Mod` (`data/rulesets.ts`): every base stat but HP scaled
    /// toward a BST of 600.
    ScalemonsMod,
    /// Gen 1's own `Scalemons Mod` (`data/mods/gen1/rulesets.ts`): toward
    /// 500, Special written to both special stats. Same name; a Gen 1
    /// format resolves the name to this one.
    ScalemonsModGen1,
    /// `350 Cup Mod`: a species with a BST of 350 or lower has its base
    /// stats doubled (gen 1's own version keeps the Special single).
    ThreeFiftyCupMod,
    /// `Guaranteed Secondary Mod`: every secondary's chance is 100.
    GuaranteedSecondaryMod,
    /// `Bad 'n Boosted Mod`: base stats of 70 or lower are doubled.
    BadNBoostedMod,
}

impl RuleId {
    pub const ALL: [RuleId; 24] = [
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
        RuleId::BestOf,
        RuleId::SameTypeClause,
        RuleId::MaxTeamSize,
        RuleId::PickedTeamSize,
        RuleId::TeamPreview,
        RuleId::Blitz,
        RuleId::ScalemonsMod,
        RuleId::ScalemonsModGen1,
        RuleId::ThreeFiftyCupMod,
        RuleId::GuaranteedSecondaryMod,
        RuleId::BadNBoostedMod,
    ];

    /// The rule's name (its `name` in `data/rulesets.ts`).
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
            RuleId::BestOf => "Best Of",
            RuleId::SameTypeClause => "Same Type Clause",
            RuleId::MaxTeamSize => "Max Team Size",
            RuleId::PickedTeamSize => "Picked Team Size",
            RuleId::TeamPreview => "Team Preview",
            RuleId::Blitz => "Blitz",
            RuleId::ScalemonsMod | RuleId::ScalemonsModGen1 => "Scalemons Mod",
            RuleId::ThreeFiftyCupMod => "350 Cup Mod",
            RuleId::GuaranteedSecondaryMod => "Guaranteed Secondary Mod",
            RuleId::BadNBoostedMod => "Bad 'n Boosted Mod",
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
            RuleId::SameTypeClause => Some("Same Type Clause: Pokemon in a team must share a type"),
            RuleId::Blitz => Some("Blitz: Super-fast timer"),
            RuleId::ScalemonsMod => Some(
                "Scalemons Mod: Every Pokemon's stats, barring HP, are scaled to come as close to a BST of 600 as possible",
            ),
            RuleId::ScalemonsModGen1 => Some(
                "Scalemons Mod: Every Pokemon's stats, barring HP, are scaled to come as close to a BST of 500 as possible",
            ),
            RuleId::ThreeFiftyCupMod => {
                Some("350 Cup Mod: If a Pokemon's BST is 350 or lower, all of its stats get doubled.")
            }
            RuleId::BadNBoostedMod => Some("Bad 'n Boosted Mod: All of a Pokemon's base stats below 70 are doubled."),
            // Team Preview announces only Tera Type Preview (gen 9).
            RuleId::BestOf
            | RuleId::MaxTeamSize
            | RuleId::PickedTeamSize
            | RuleId::TeamPreview
            | RuleId::GuaranteedSecondaryMod => None,
        }
    }

    /// The rule takes a value (`hasValue: 'positive-integer'`, written
    /// `Name = N`).
    pub const fn has_value(self) -> bool {
        matches!(self, RuleId::BestOf | RuleId::MaxTeamSize | RuleId::PickedTeamSize)
    }

    /// The rule a name resolves to: PS's `toID` of the name, as
    /// `validateRule` looks a rule up (`same type clause`, `sametypeclause`
    /// and `Same Type Clause` are one rule). A gen-specific rule resolves
    /// to its base version; [`FormatSpec::parse`] picks the gen's.
    pub fn parse(name: &str) -> Option<RuleId> {
        let id = crate::to_id::<RULE_TEXT_CAP>(name).ok()?;
        RuleId::ALL.into_iter().find(|r| {
            *r != RuleId::ScalemonsModGen1 && crate::to_id::<RULE_TEXT_CAP>(r.name()).is_ok_and(|n| n == id)
        })
    }

    /// The version of the rule format `base`'s gen resolves the name to
    /// (`Dex.mod(gen).formats`: a mod's own entry shadows the base one).
    pub const fn for_gen(self, gen: u8) -> RuleId {
        match self {
            RuleId::ScalemonsMod if gen == 1 => RuleId::ScalemonsModGen1,
            r => r,
        }
    }
}

/// Most custom rules after `@@@`.
pub const MAX_CUSTOM_RULES: usize = 4;

/// Byte capacity of one custom rule as written (`max team size = 3`).
pub const RULE_TEXT_CAP: usize = 32;

/// One custom rule after `@@@`: a rule added to the format's ruleset, or
/// (`!Name`) one repealed from it, with its value (`Best Of = 3`) and the
/// text it was written as. PS's custom rules infobox prints the rules as
/// written (`format.customRules`, the `formatid` split at `,`), so the text
/// is kept: `same type clause` and `Same Type Clause` are the same rule
/// but not the same line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CustomRule {
    pub rule: RuleId,
    /// `!Name`: PS's rule repeal (`sim/dex-formats.ts getRuleTable`).
    pub repeal: bool,
    /// The value of a rule that takes one, 0 for none.
    pub value: u16,
    /// The rule as written after `@@@`.
    pub text: BoundedStr<RULE_TEXT_CAP>,
}

impl CustomRule {
    /// `rule` added, written as its name.
    pub fn add(rule: RuleId) -> Self {
        Self { rule, repeal: false, value: 0, text: BoundedStr::from_str(rule.name()).unwrap_or_default() }
    }

    /// `rule` repealed, written `!` and the name.
    pub fn repeal(rule: RuleId) -> Self {
        let mut text = BoundedStr::new();
        let _ = text.push_str("!");
        let _ = text.push_str(rule.name());
        Self { rule, repeal: true, value: 0, text }
    }

    /// Calls `f` with the rule as written after `@@@` and in the custom
    /// rules infobox.
    pub fn write(&self, mut f: impl FnMut(&str)) {
        f(self.text.as_str());
    }

    /// One rule of a `formatid`, PS `validateRule`: the text must be
    /// trimmed; `Name = value` splits at `=`; the name is looked up by its
    /// id. A value rule needs a positive integer value (Best Of: odd, 3 to
    /// 9, its `onValidateRule`) and any other rule takes none. `!!` (repeal
    /// and replace a value) and `^` are not taken.
    pub fn parse(s: &str) -> Option<CustomRule> {
        if s != s.trim() || s.starts_with("!!") || s.starts_with('^') {
            return None;
        }
        let (repeal, body) = match s.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        // JS `rule.split('=')`: the name, then the text up to the next `=`.
        let mut parts = body.split('=');
        let name = parts.next()?;
        let value = parts.next().map(str::trim);
        let rule = RuleId::parse(name)?;
        let value = match (rule.has_value(), value) {
            (true, Some(v)) if !repeal => {
                let n: u16 = v.parse().ok().filter(|&n| n > 0)?;
                if rule == RuleId::BestOf && (n > 9 || n < 3 || n % 2 != 1) {
                    return None;
                }
                n
            }
            (_, None) if !rule.has_value() || repeal => 0,
            _ => return None,
        };
        let text = BoundedStr::from_str(s).ok()?;
        Some(CustomRule { rule, repeal, value, text })
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

    /// The gen 1 format the harness plays (`rules.js` installs the same
    /// custom rules). `Cartridge Trunc` is in every Gen 1 format's ruleset,
    /// so it is on.
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
                let mut rule = CustomRule::parse(name)?;
                rule.rule = rule.rule.for_gen(spec.base.gen());
                spec.custom_rules.push(rule).ok()?;
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
        assert_eq!(FormatSpec::parse("gen1customgame@@@Inverse Mod"), None);
        let math = g1.with_repeal(RuleId::CartridgeTrunc).unwrap();
        assert_eq!(id_of(&math), "gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod,!Cartridge Trunc");
        assert_eq!(FormatSpec::parse(&id_of(&math)), Some(math));
        let rb = FormatSpec::parse("gen1randombattle@@@!Cartridge Trunc").unwrap();
        assert_eq!(rb.custom_rules[0], CustomRule::repeal(RuleId::CartridgeTrunc));
        assert_eq!(FormatSpec::parse("gen1customgame@@@!!Cartridge Trunc"), None);
        // `validateRule` looks the name up by its id, so the space goes.
        assert_eq!(
            FormatSpec::parse("gen1customgame@@@! Cartridge Trunc").map(|f| (f.custom_rules[0].rule, f.custom_rules[0].repeal)),
            Some((RuleId::CartridgeTrunc, true))
        );
        assert_eq!(FormatSpec::parse("gen1randombattle").map(|f| f.base.name()), Some("[Gen 1] Random Battle"));
        assert_eq!(FormatSpec::parse("gen3randombattle").map(|f| f.base.gen()), Some(3));
        assert_eq!(g1.base.name(), "[Gen 1] Custom Game");
        assert_eq!(g1.base.gen(), 1);
    }

    #[test]
    fn variant_rules_parse_as_ps_validates_them() {
        // The variant formats of the public replay corpus.
        for (id, rules) in [
            ("gen1randombattle@@@Best of = 7", &[(RuleId::BestOf, 7)][..]),
            ("gen3randombattle@@@Blitz", &[(RuleId::Blitz, 0)]),
            ("gen1randombattle@@@Scalemons Mod", &[(RuleId::ScalemonsModGen1, 0)]),
            ("gen3randombattle@@@scalemonsmod", &[(RuleId::ScalemonsMod, 0)]),
            ("gen1randombattle@@@guaranteed secondary mod", &[(RuleId::GuaranteedSecondaryMod, 0)]),
            ("gen1randombattle@@@same type clause,max team size = 3", &[(RuleId::SameTypeClause, 0), (RuleId::MaxTeamSize, 3)]),
            ("gen1randombattle@@@maxteamsize=3,pickedteamsize=1", &[(RuleId::MaxTeamSize, 3), (RuleId::PickedTeamSize, 1)]),
            ("gen1randombattle@@@350 cupmod", &[(RuleId::ThreeFiftyCupMod, 0)]),
            ("gen3randombattle@@@Bad N Boosted Mod", &[(RuleId::BadNBoostedMod, 0)]),
            (
                "gen3randombattle@@@sametypeclause,pickedteamsize=4,team preview",
                &[(RuleId::SameTypeClause, 0), (RuleId::PickedTeamSize, 4), (RuleId::TeamPreview, 0)],
            ),
        ] {
            let spec = FormatSpec::parse(id).unwrap_or_else(|| panic!("{id}"));
            let got: alloc::vec::Vec<(RuleId, u16)> = spec.custom_rules.iter().map(|c| (c.rule, c.value)).collect();
            assert_eq!(got, rules, "{id}");
            // The text is kept as written, so the id prints back unchanged.
            assert_eq!(id_of(&spec), id);
        }
        // A value rule needs its value, a plain rule takes none, the text
        // must be trimmed, and Best Of is odd and 3 to 9.
        for bad in [
            "gen3randombattle@@@Best Of",
            "gen3randombattle@@@Best Of = 4",
            "gen3randombattle@@@Best Of = 11",
            "gen3randombattle@@@Max Team Size = 0",
            "gen3randombattle@@@Blitz = 2",
            "gen3randombattle@@@ Blitz",
        ] {
            assert_eq!(FormatSpec::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn rule_lines() {
        assert_eq!(RuleId::CancelMod.line(), None);
        for r in RuleId::ALL {
            assert_eq!(RuleId::parse(r.name()).map(|p| p.for_gen(if r == RuleId::ScalemonsModGen1 { 1 } else { 3 })), Some(r));
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
