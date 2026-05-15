# Gen 1 Battle Engine Spec (`gen1_battle`)

Cartridge-accurate RBY mechanics. Goal: drop-in replacement for the `battler` crate in `mega_blastoise_core`/`_fw` consumption, with ~800 B of RAM per 6v6 battle and zero per-turn heap churn.

Authoritative sources cited inline:
- **pret/pokered** — RBY disassembly. Definitive. https://github.com/pret/pokered
- **Bulbapedia** — Gen I damage / status / move mechanics pages
- **Smogon Gen 1 OU / RBY mechanics writeups**

When this doc and pret/pokered disagree, pret wins.

---

## 1. Memory layout targets

### Per-Pokemon runtime state (`Mon`) — target ≤ 64 B

| Field | Size | Notes |
|---|---|---|
| `species: u8` | 1 | 151 species fit in 1 byte (0 = empty slot) |
| `level: u8` | 1 | 1–100 |
| `hp_cur: u16` | 2 | |
| `hp_max: u16` | 2 | |
| `status: u8` | 1 | `0bABCD_EEEE` where E = sleep counter (0–7), A=PSN, B=BRN, C=FRZ, D=PAR; BadPSN = PSN-bit set + bit in `volatile`. See §6. |
| `moves: [MoveSlot; 4]` | 12 | each `(id: u8, pp: u8, max_pp: u8)` = 3 B |
| `stats: [u16; 5]` | 10 | HP/Atk/Def/Spc/Speed (Gen 1 — no Sp.Atk/Sp.Def split) |
| `ivs: [u8; 4]` | 4 | Atk/Def/Spc/Speed IVs (4-bit each, packed); HP IV derived from LSBs |
| `stat_stages: [i8; 4]` | 4 | Atk/Def/Spc/Speed modifier stages (−6..+6). HP not modifiable. |
| `volatile: u32` | 4 | Bitflags: Confused, ConfusedTurns(3b), Flinched, LeechSeeded, Substituted, SubstituteHp(8b), MustRecharge, Biding, BideDmg(16b — overlap ok via separate field), Disabled, DisabledMoveSlot(2b), DisabledTurns(3b), WrappedBy(self-side / locked-into-Wrap), MultiTurnMoveId(8b), MultiTurnTurnsLeft(3b), ReflectActive, LightScreenActive, FocusEnergyActive, MistActive, BadPoisonCounter(4b) |
| `last_move_used: u8` | 1 | move id (0 = none); used by Mirror Move |
| `last_move_targeted_by_normal_or_fighting: u16` | 2 | Counter source damage. 0 if not eligible. |
| Padding/alignment | ~5 | |

→ ~52 B packed, ~64 B with alignment. Two teams × 6 = **768 B** for both rosters. Add `Battle` (active idx, turn counter, RNG, weather=none, choice buffers, narration ring) ≈ 200 B. **Total battle RAM ≤ 1 KB.**

### Static data tables (in flash)

| Table | Entries | Per-entry | Size |
|---|---|---|---|
| `SPECIES: &[SpeciesData; 151]` | 151 | 32 B (id, name_idx, base_stats×5, types×2, catch_rate, base_exp, growth_rate, learnset_ptr) | ~5 KB |
| `MOVES: &[MoveData; 165]` | 165 | 24 B (id, name_idx, type, category, power, accuracy, pp, target, effect_kind, effect_param×2) | ~4 KB |
| `TYPE_CHART: [[u8; 15]; 15]` | 225 | 1 B (0=immune, 5=½, 10=1×, 20=2×) | 225 B |
| `NAME_STRINGS: &[&'static str]` | ~316 | pointer | ~2.5 KB |
| `LEARNSETS: &'static […]` | learn entries | (level, move_id) packed | ~2 KB |

→ **~13 KB of flash**, zero heap.

---

## 2. Damage formula (Gen 1, cartridge-accurate)

Source: pret/pokered `engine/battle/core.asm` (`DamageCalc`), Bulbapedia "Damage" Gen I section.

```
1. If move power == 0:           damage = 0; skip
2. If self-targeted boost-only:  damage = 0; skip
3. atk, def = effective stats (see §3); use Special for Special category, else Phys.
   - if crit, atk = base[atk_stat] of attacker, def = base[def_stat] of defender
     (i.e. crits IGNORE stat stages, including the attacker's drops!)
4. If burned and Physical: atk = atk / 2  (clamped to ≥ 1)
5. Reflect doubles Def for Physical hits against you; Light Screen doubles Spc.
6. damage = ((((2 * level / 5 + 2) * power * atk / def) / 50) + 2)
7. damage *= STAB:                if user has move's type:  damage = damage * 15 / 10
8. damage *= type_effectiveness:  per multiplier from TYPE_CHART (each type slot applied separately)
   - if final effectiveness == 0:  damage = 0; report immune; skip rest
9. damage *= crit_multiplier:     2× if crit (see §4)
10. if damage > 0:
     random = rand_byte; loop until random in [217..=255]   ; pret: %byte then mask
     damage = damage * random / 255
11. clamp damage to [1, opponent_hp_remaining]              ; Gen 1 quirk: can't be 0 unless immune
```

### Notable Gen 1 quirks
- **STAB and type are applied BEFORE crit multiplier**, not after.
- **Crits ignore stat stages** on attacker AND defender. This means a Swords-Danced attacker still does base-attack damage on a crit, and a Defense-curled defender takes a base-defense crit. Source: pret `DamageCalc.scaleDamage`.
- **Random factor**: roll byte, reject below 217 (= ~85%–100% spread). Done via rejection sampling against `217..=255`.
- **Min damage = 1** if move hit and not immune. Even a 1-HP target takes the rolled damage (overkill possible).
- **No critical-hit cap** at level 1 like later gens enforce; Gen 1 has none.

---

## 3. Stat modifier formula

Source: pret `core.asm` (`ApplyBadgeStatBoosts` is irrelevant in link play; `CalculateModifyStats`).

```
Stages: -6..+6
Multiplier table (numerator/denominator):
  -6: 25/100,  -5: 28/100, -4: 33/100, -3: 40/100, -2: 50/100, -1: 66/100,
   0: 100/100,
  +1: 150/100, +2: 200/100, +3: 250/100, +4: 300/100, +5: 350/100, +6: 400/100

effective = (base_stat * num) / den
Clamp: 1..=999 (Gen 1 cap is 999, not 1023)
```

### Gen 1 quirks
- **PAR drop is permanent for that mon**: paralyzed mon's Speed = `base_speed * stage_mult / 4`. Even after PAR is healed mid-battle, the /4 stays applied until switch (Gen 1 bug; pret `core.asm`: `PartyHealStatusEffects` flow). Actually, the /4 is applied at *stat recalc time*, which happens after switch-in and after stat-modification moves. So a stat-boost move recalcs and the /4 is reapplied; on PAR-heal mid-battle without a recalc, /4 stays. **Document the bug: /4 is sticky unless a stat-changing move triggers a recalc.**
- **BRN halves Atk** similarly, sticky in the same way.
- **Stat-boost moves that try to go past +6 / below -6 still display the message and don't error.** No effect.

---

## 4. Crit chance and accuracy

### Crit
Source: pret `core.asm` `CriticalHitTest`.

```
crit_chance_byte = base_speed / 2         ; UNMODIFIED base speed
if move is High-Crit (Crabhammer, Karate Chop, Razor Leaf, Slash):
    crit_chance_byte *= 8
    clamp to 255
if Focus Energy active:
    crit_chance_byte = crit_chance_byte / 4   ; BUG: should be *4
    clamp to 255

random_byte = rand
crit if random_byte < crit_chance_byte
```

- Use base species speed, never modified.
- Focus Energy is reversed (REDUCES crit rate). **Implement as-is** for cartridge accuracy.

### Accuracy
Source: pret `core.asm` `MoveHitTest`.

```
1. If move always hits (Swift, Bide-store): hit, skip
2. accuracy = move.accuracy            ; e.g. 100 -> 255 (* 255 / 100, truncated)
3. accuracy = accuracy * acc_stage_mult / 100
4. accuracy = accuracy * (100 / evade_stage_mult) / 100
5. random_byte = rand
   if random_byte >= accuracy: MISS
```

- **1/256 miss bug**: even at "100% accuracy" the truncation `100 * 255 / 100 = 255` means random_byte must be `< 255`, so a roll of `255` misses. → ~1/256 miss probability for any non-Swift move.
- **OHKO moves**: extra rule before the accuracy roll: if user_speed < target_speed, the OHKO move **always misses** ("It doesn't affect [foe]"). Otherwise it uses 30% accuracy. Source: pret `OHKOMoveEffect`.

---

## 5. Turn structure

Source: pret `core.asm` `MainBattleLoop`.

```
Per turn:
1. Get player choice + AI/opponent choice
2. Choice validation (can't switch if Wrapped, must use last move if Bide/HyperBeam recharge/Wrap/Thrash/PetalDance/MultiTurnLock, etc.)
3. Determine order:
   priority A side > priority B side
   else: higher effective Speed (post-PAR, post-stage)
   else: random (50/50)
4. For each side in order:
   a. Faint check — if user fainted earlier this turn, skip
   b. PreMove status checks (in this order, from pret):
      - Frozen: skip (cannot thaw on own; only Fire-type moves on you can thaw)
      - Sleeping: decrement counter, if 0 wake, else skip. (Sleep counter rolls 1..=7.)
      - Paralyzed: 25% chance "fully paralyzed", skip
      - Flinched: skip; clear flinch
      - Confused: decrement counter; if 0 snap out;
        else 50% chance hit self with 40-power typeless Physical attack ignoring screens
      - Disabled: if chose disabled move, "no PP for that move" — actually in Gen 1, Disable forces no input restriction; it just makes the move fail. **Implement: Disable causes any attempt to use the disabled slot to fail silently with "<move> is disabled!".**
      - Bide active: continue Bide (turn 2 of 3 or unleash)
      - Multi-turn lock: continue the locked move
      - Recharge: skip; clear recharge
   c. Use move:
      - Deduct PP (Gen 1 quirk: PP deducted BEFORE move resolves; Mirror Move/Metronome do NOT rededuct)
      - Run move effect (see §7)
      - On hit: set defender's `last_move_targeted_by_normal_or_fighting` if applicable
   d. Faint check on defender
5. End-of-turn:
   - Burn / Poison damage: 1/16 max HP each
   - BadPoison: counter * 1/16 max HP; counter increments each turn
   - Leech Seed: 1/16 from seeded, restored to seeder
   - Wrap-like binding damage (handled at attacker's turn in Gen 1, not EOT — verify against pret)
   - Substitute fade: no, sub persists until broken
6. Faint resolution: forced switch from anyone at 0 HP
7. Win check: if all of either side fainted, battle ends
```

### Gen 1 quirks
- **Move order for tied speed is a 50/50** in pret. Some implementations make it deterministic; we should match the cartridge and use RNG.
- **Quick Attack**: priority +1. Counter: priority -1. (Pursuit doesn't exist in Gen 1.)
- **PP underflow into the next-slot's PP via Transform copy**: real Gen 1 bug; we can ignore (rare and the user probably doesn't want PP-glitches).

---

## 6. Status effects

| Status | Encoding | Effect | Cure |
|---|---|---|---|
| PSN | bit 3 | 1/16 max HP EOT | Antidote / Heal Bell (not in Gen 1, so: switch out doesn't cure; only items/move Rest) |
| BRN | bit 2 | 1/16 max HP EOT; halves Atk (sticky — see §3) | Rest / Full Heal |
| FRZ | bit 1 | Cannot act. **Cannot thaw on own.** Thawed only by being hit by Fire-type damaging move | Fire damage / Rest? (Rest in Gen 1 sleeps then heals; it WOULD cure FRZ on wake but FRZ doesn't wake) — actually Rest cures FRZ in Gen 1 because Rest sets sleep + heals + clears other status. Confirm against pret. |
| SLP | bits 0–2 (counter 1..7) | Cannot act; counter -- each turn at PreMove. Wake at 0. | Counter expiration / Rest sets a fresh counter |
| PAR | bit 5 | 25% fail to act; halves Speed (sticky) | Rest / Paralyz Heal |
| BAD_PSN | volatile flag + PSN bit | Counter * 1/16 dmg EOT, counter ++; counter resets to 1 on switch in (Gen 1 — confirm; Toxic counter does NOT survive switch in Gen 1). | Same as PSN |

**Mutually exclusive**: only one major status at a time (the bits above one per mon).
**Volatile** statuses (confusion, leech seed, sub, etc.) stack with major.

### Sleep mechanics
- Counter rolls 1..7 (Gen 1, not 1..3 like later gens).
- Each PreMove, decrement. At 0, wake; on the **same turn** the mon wakes, it can move (no "lost turn" — actually pret `core.asm` shows the wake turn IS lost in Gen 1; **confirm and document**).
- Rest sets counter to 2 → can act 2 turns later. (Rest in Gen 1 is unbreakable except Hyper Beam recharge).

### Freeze
- 100% lockout. The only way to thaw mid-battle is being hit by a damaging Fire-type move. Hyper Beam / Body Slam / Tri Attack do NOT thaw in Gen 1 (those secondary-effect mechanics come later gens).

---

## 7. `MoveEffectKind` enum (~30 variants)

This is the lookup table for what a move does on hit. Each move has a single `effect_kind`; weird moves get special-cased (§8).

```rust
#[repr(u8)]
pub enum MoveEffectKind {
    /// Plain damaging move with no rider.
    Damage,
    /// Damage + chance to inflict status. param0=status, param1=chance/256.
    DamageAndMaybeStatus,
    /// Damage + chance to flinch. param0=chance/256.
    DamageAndMaybeFlinch,
    /// Damage + stage change on target. param0=stat, param1=delta i8, param2=chance/256.
    DamageAndMaybeBoostTarget,
    /// Damage + stage change on self. (Swords Dance-on-hit kind of moves; n/a in Gen 1 ... but used by Hi Jump Kick etc.)
    DamageAndMaybeBoostSelf,
    /// Stage change on self only. param0=stat, param1=delta.
    BoostSelf,
    /// Stage change on target only. param0=stat, param1=delta.
    BoostTarget,
    /// Inflict status on target (always). param0=status.
    StatusOnly,
    /// Multi-hit damaging (2-5 hits, weighted 3/8 each for 2-3, 1/8 for 4-5).
    MultiHit2to5,
    /// Multi-hit fixed (Double Kick: 2; Twineedle: 2 + poison chance).
    MultiHitFixed,
    /// Drain HP (% of damage dealt heals user). param0=numerator, param1=denominator. (½ for Mega Drain, Giga Drain not in Gen 1.)
    DrainHp,
    /// Recoil 1/4 of damage dealt.
    Recoil1of4,
    /// Take Down recoil = 1/4; Double-Edge = 1/4 in Gen 1 (not 1/3).
    /// Crash damage on miss: Hi Jump Kick / Jump Kick. 1 HP self damage in Gen 1 (NOT 1/8 like later gens).
    CrashOnMiss,
    /// OHKO move (Fissure, Horn Drill, Guillotine).
    Ohko,
    /// Force target to switch (Whirlwind, Roar — Gen 1: ALWAYS FAILS in trainer battles; only works in wild).
    ForceSwitchTarget,
    /// Fixed damage equal to user's level (Night Shade, Seismic Toss).
    LevelDamage,
    /// Fixed damage of param0 HP (Sonic Boom = 20, Dragon Rage = 40).
    FlatDamage,
    /// Variable fixed damage 1..=1.5*level (Psywave). Actually random byte in 0..=1.5*level.
    Psywave,
    /// Set target HP to 50% (Super Fang).
    HalfHp,
    /// Heal self by 1/2 max HP (Recover, Soft-Boiled, Rest also uses).
    HealHalf,
    /// Rest: full heal + sleep counter set 2.
    Rest,
    /// Two-turn (charge then attack): Solar Beam, Sky Attack, Razor Wind, Skull Bash, Fly, Dig.
    /// param0=invulnerable_phase (0/1).
    TwoTurn,
    /// Bide: store damage 2-3 turns, return 2x.
    Bide,
    /// Hyper Beam (recharge required UNLESS KO).
    HyperBeam,
    /// Counter (mirror back 2x last Normal/Fighting damage targeting self).
    Counter,
    /// Mirror Move (use opponent's last move).
    MirrorMove,
    /// Mimic (replace random move slot with target's last move; PP=5).
    Mimic,
    /// Transform (copy target's species/stats/moves/types/stages; HP unchanged).
    Transform,
    /// Substitute (1/4 HP + 1 sub).
    Substitute,
    /// Disable (random target move, 1-6 turns).
    Disable,
    /// Wrap-like (lock target for 2-5 turns; trapped can't switch; small damage per turn).
    Wrap,
    /// Leech Seed (volatile, drains 1/16 EOT).
    LeechSeed,
    /// Light Screen (Spc damage doublers for 5+ turns).
    LightScreen,
    /// Reflect (Phys defense doublers for 5+ turns).
    Reflect,
    /// Mist (immune to stat reductions while active).
    Mist,
    /// Focus Energy (modifies crit rate, BUGGED in Gen 1 to lower).
    FocusEnergy,
    /// Conversion (copy target's type onto self).
    Conversion,
    /// Haze (clears all stat stages on both sides + status of opponent? — Gen 1 hazes status of opponent too).
    Haze,
    /// Metronome (random move 1..=164, excluding Metronome and Struggle).
    Metronome,
    /// Pay Day (deal damage; if it hits, sprinkle coins — implement as no-op narration).
    PayDay,
    /// Splash / Self-status no-op.
    NoOp,
    /// Explosion / Self-Destruct (high-power phys, halves target Def, KOs user; in Gen 1 halves *defense*, not damage).
    SelfDestruct,
}
```

This list is exhaustive for Gen 1; any move maps to exactly one variant, with the `param` slots in `MoveData` filling in chance/stat/delta etc.

---

## 8. Special-case moves

These get hand-written functions because their behavior is too unique for a single enum variant:

| Move | Behavior |
|---|---|
| **Counter** | Stores last hit damage taken from a Normal/Fighting damaging move *this turn*. Deals 2× back. Selects via `effect_kind = Counter`; logic in `apply_counter()`. |
| **Mirror Move** | Looks at target's `last_move_used`. If valid, recurses move execution with that move (no PP cost). If target has no last move, "Mirror Move failed!". |
| **Mimic** | Picks one of target's known moves at random; replaces user's Mimic slot with it (PP=5). Lasts until switch. |
| **Transform** | Copies species byte, types, stats (incl. stages), and moves (each with PP=5) from target. Volatile flag set; cleared on switch. |
| **Substitute** | Costs 1/4 max HP; sub has 1/4 max HP + 1. Fails if user has ≤ 1/4 HP. While active, all damage applies to sub HP; sub breaks when HP would go ≤ 0. **Bug: an attack dealing exact-HP damage breaks the sub** because sub_hp -= dmg goes 0, not -1. Confusion self-hit ignores sub. |
| **Disable** | Picks one of target's moves with PP > 0, disables it for 1..=8 turns. Fails if target has no moves with PP. Cleared on switch. |
| **Bide** | Turn 1: announce "started storing energy". Turn 2: continue. Turn 3 (or random 2/3): unleash 2× stored damage. Stores damage TAKEN, not dealt. Move always succeeds in unleash phase (no accuracy roll). |
| **Hyper Beam** | After dealing damage, if target NOT fainted, user gains `MustRecharge` and next turn is wasted on "must recharge". **If target fainted, no recharge.** |
| **Metronome** | Roll 1..=165, skip Metronome (and Struggle). Execute that move with no PP deduction. |
| **Conversion** | Copy target's primary + secondary type onto user (or just primary if single-typed). |
| **Haze** | Reset all stat stages on both sides to 0; clear opponent's non-volatile status; clear own confusion. (Gen 1 — see Bulbapedia.) |
| **Rest** | Restore full HP, set SLP counter to 2, cure other major statuses. Cannot be used at full HP. |
| **Wrap / Bind / Fire Spin / Clamp** | Lock target for 2..=5 turns; user can attack only with this move; target cannot move. Tiny damage (1/16 max) each turn. If user switches, target frees. |

---

## 9. Random battle teambuilder

Source: this is *not* cartridge — Smogon randbats use their own distributions. We'll do simplest plausible:

```
For each side, 6 species (sample without replacement from SPECIES, exclude legendaries optionally).
For each mon:
    level = 100
    IVs = max (15/15/15/15) for simplicity
    moves = 4 moves chosen from species learnset (level 1..=level)
             filter: prefer damaging, then status, then utility — or simpler: random 4
    HP recalc per Gen 1 formula
```

The `draw_two_randbat_teams(seed, n)` API in `mega_blastoise_core/src/randbat.rs` is the existing entry point. We'll port it to use `gen1_battle`'s tables.

---

## 10. API mimicry surface

The new crate exposes types/methods to satisfy these existing callsites:

### Types
```rust
pub struct Battle<'a> { /* opaque */ }
pub struct CoreBattleOptions { pub seed: Option<u64>, ... }
pub struct CoreBattleEngineOptions { ... }
pub struct PlayerData { pub id: String, pub name: String, ... }
pub struct TeamData { pub members: Vec<MonData>, ... }
pub struct MonData { pub species: ..., pub level: u8, pub moves: Vec<MoveSlot>, ... }
pub struct MonBattleData { pub active: bool, pub summary: MonSummary, pub moves: Vec<MoveSlot>, ... }
pub struct PlayerBattleData { pub mons: Vec<MonBattleData>, ... }
pub enum Request { Turn { ... }, Switch { ... }, TeamPreview { ... } }
pub enum Type { Normal, Fire, Water, ... }  // identical variants to battler::Type
pub enum Stat { Hp, Atk, Def, Spc, Speed }  // Gen 1 — no SpAtk/SpDef
```

### Methods
```rust
impl<'a> Battle<'a> {
    pub fn new(opts: CoreBattleOptions, data: &dyn DataStore, engine: CoreBattleEngineOptions) -> Result<Self>;
    pub fn update_team(&mut self, player_id: &str, team: TeamData) -> Result<()>;
    pub fn start(&mut self) -> Result<()>;
    pub fn active_requests(&self) -> impl Iterator<Item = (&str, &Request)>;
    pub fn set_player_choice(&mut self, player_id: &str, choice: &str) -> Result<()>;
    pub fn new_log_entries(&mut self) -> impl Iterator<Item = &str>;
    pub fn ended(&self) -> bool;
    pub fn player_data(&self, player_id: &str) -> Result<PlayerBattleData>;
    pub fn active_mon_move_pp(&self, player_id: &str) -> Option<Vec<(u8, u8)>>;
    pub fn drain_action_timings(&mut self) -> impl Iterator<Item = (&str, u32)>;
}
```

Choice string format (kept identical to battler so the AI / USB cmd parser doesn't change):
- `"move 1"`, `"move 2"`, ...
- `"switch 3"` (switch to team slot 3)
- `"team 1 2 3 4 5 6"` (team preview — Gen 1 doesn't actually use this in randbats but kept for compat)

---

## 11. RNG

Single `u64` LCG/xorshift state stored in `Battle`. All randomness routed through one `Battle::rand_byte() -> u8` / `Battle::rand_range(n) -> u8`. Seeded from `CoreBattleOptions::seed`.

This is the only place randomness happens; **no `rand` crate, no thread-local**. Same seed → same battle, reproducibly.

---

## 12. Narration / log

A small ring buffer of `Event`s (enum, fixed size each):

```rust
pub enum Event {
    MoveUsed { user: SideMonIdx, move_id: u8 },
    Damage   { target: SideMonIdx, dealt: u16 },
    Miss     { user: SideMonIdx },
    Crit     { user: SideMonIdx },
    SuperEffective,
    NotVeryEffective,
    Immune,
    StatusInflicted { target: SideMonIdx, status: Status },
    StatChanged { target: SideMonIdx, stat: Stat, delta: i8 },
    SwitchIn { side: u8, slot: u8 },
    Faint    { side: u8, slot: u8 },
    Win      { side: u8 },
    // ... ~20 total
}
```

`new_log_entries()` formats these into strings *on demand* at the API boundary (only when called). The strings allocate then, but they're consumed and dropped by the caller (`battle_runner::enrich_and_dispatch`) per turn — no accumulation. Ring buffer size: 128 events (4 KB at most).

---

## 13. Known gaps / TODOs (called out, not implemented in v1)

- Trapping moves clearing on switch: verify pret behavior for Wrap-user switching out.
- Status duration on cartridge: counters for Confusion (2–5 turns), Disable (1–8), Wrap (2–5) — exact ranges per pret.
- "Move underflow" PP glitch: Transform copy can leave PP fields uninitialized in some edge cases; cartridge bug, document as not implemented.
- AI move choice: not the engine's concern; `RandomAi` already exists outside the engine.

---

## 14. Testing strategy

Three layers:
1. **Unit tests for damage formula**: assert known cartridge results, e.g. "L100 Tauros body slamming L100 Snorlax does X HP per damage roll".
2. **Move-effect fixture tests**: one per `MoveEffectKind` variant covering basic case + edge cases (immune type, crit, low HP, etc.).
3. **End-to-end deterministic battles**: seed + scripted choices → expected final HP / win. Sourced from real Smogon RBY randbat replays if available.

Run on host (`cargo test -p gen1_battle`) — no firmware roundtrip needed for engine tests.
