#!/usr/bin/env node
'use strict';
// tools/ps-diff/teams.js
//
// Build the two sides' teams for one battle.
//
//   const teams = require('./teams');
//   const built = teams.buildPair({ gen: 3, mode: 'wide', seed, focus, self: true });
//
// Modes:
//   'random'  - PS's own genNrandombattle generator.
//   'wide'    - 1-6 mons per side, 1-4 distinct moves each, level 40-100,
//               IVs 0-31, EVs <= 510 with rotating-start stat order. Gen 3
//               adds items and abilities; gen 1 has neither. The pool is:
//                 * --self: the whole showdown_fuzz.rs space (no
//                   whitelist) -- it's the harness's own self-test, so the
//                   only thing it has to be consistent about is that PS
//                   runs the same draws both times.
//                 * --ps-cli: only ids `ps_sim_cli --ported <gen>` prints.
//                   An empty list is `wide` against the rust side exiting
//                   2 with "no ported pipeline for gen N".
//   'file:...' - one PS packed team per line from <path>.
//
// `readCarried(dir)` loads the carried corpus (diff.js `--carried`): the
// old engines' pinned battles as PS sets JSON, one team pair per case.
//
// Each side is returned as the packed-string form PS takes in `>player`,
// which the driver (diff.js) feeds into both engines.
//
// Source of truth for the random-battle generator and the packed format
// is PS 0.11.11 (tools/ps-diff/node_modules/pokemon-showdown). The wide
// generator is a clean-room port of crates/mega_blastoise_test/tests/
// showdown_fuzz.rs's `Fuzz::new` + species/nature/IV/EV/item/ability
// draws, so a diff against the Rust side lays pitfalls at the same
// joints the original fuzz did.

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const { Teams, Dex } = require('pokemon-showdown');

const { spawnPorted } = require('./ported');

/// Build a team pair.
/// `opts`: { gen, mode, seed (a 4 u16 array), focus?, self?, psCli? }
/// Returns { p1, p2, packed: [packed1, packed2] }.
function buildPair(opts) {
  const { gen, mode, seed, focus } = opts;
  const prng = new TeamsPrng(seed);
  if (mode === 'random') {
    return buildRandom({ gen, prng });
  }
  if (mode === 'wide') {
    const ported = spawnPorted.getPool({ gen, self: !!opts.self, psCli: opts.psCli });
    if (ported.empty) {
      // Diff.js saw this case before calling us, but a misuse (e.g. caller
      // bypassing diff.js) lands here. Echo the same message so the failure
      // is identifiable.
      const e = new Error(`no ported pipeline for gen ${gen}`);
      e.code = 2;
      throw e;
    }
    return buildWide({ gen, prng, focus, ported, self: !!opts.self });
  }
  if (mode.startsWith('file:')) {
    return buildFromFile({ path: mode.slice('file:'.length) });
  }
  throw new Error(`unknown team mode ${mode}`);
}

function buildRandom({ gen, prng }) {
  const format = `gen${gen}randombattle`;
  // p1's team from the team seed, p2's from a seed drawn off it (not a
  // mirror match).
  const p2Seed = [0, 1, 2, 3].map(() => Math.floor(prng.random() * 0x10000));
  const teams = [prng.battleSeed(), p2Seed].map((seed) => Teams.generate(format, { seed }));
  if (teams.some((t) => !t || !Array.isArray(t) || t.length === 0)) {
    throw new Error('random team generation returned an empty team');
  }
  return { p1: teams[0], p2: teams[1], packed: [Teams.pack(teams[0]), Teams.pack(teams[1])] };
}

function buildWide({ gen, prng, focus, ported, self }) {
  const p1 = makeWideTeam({ gen, prng, focus, ported, self });
  const p2 = makeWideTeam({ gen, prng, focus, ported, self });
  return { p1, p2, packed: [Teams.pack(p1), Teams.pack(p2)] };
}

function buildFromFile({ path: filePath }) {
  const text = fs.readFileSync(filePath, 'utf8');
  const lines = text.split(/\r?\n/).filter((s) => s.trim().length > 0);
  if (lines.length < 2) throw new Error(`file ${filePath} holds fewer than 2 teams`);
  const teams = lines.slice(0, 2).map((line) => Teams.unpack(line));
  if (teams.some((t) => !t)) throw new Error(`failed to unpack a team in ${filePath}`);
  return { p1: teams[0], p2: teams[1], packed: [Teams.pack(teams[0]), Teams.pack(teams[1])] };
}

/// The carried corpus: every `*.json` under `dir` (written by
/// `scripts/carried-dump.sh` through showdown_fuzz.rs's `FUZZ_DUMP`), each
/// holding the team pairs of one old-engine pinned entry as PS sets. Files
/// in name order, cases in file order; a pair that an earlier file already
/// holds (a `case=` entry inside a whole-seed one) is played once, under its
/// first name. Returns [{ name, p1, p2, packed }], name `<file>#<case>`.
function readCarried(dir) {
  if (!fs.existsSync(dir)) throw new Error(`no carried corpus at ${dir}`);
  const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json')).sort();
  const pairs = [];
  const seen = new Set();
  for (const f of files) {
    const doc = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
    for (const c of doc.cases || []) {
      const packed = [Teams.pack(c.p1), Teams.pack(c.p2)];
      const key = packed.join('\n');
      if (seen.has(key)) continue;
      seen.add(key);
      pairs.push({ name: `${f.replace(/\.json$/, '')}#${c.case}`, p1: c.p1, p2: c.p2, packed });
    }
  }
  return pairs;
}

/// The targeted battles (diff.js `--targeted`): a JSON array of
/// `{name, covers, seed, trunc?, p1, p2, choices, expect?}`. A side is a list
/// of PS sets; a set names its `species` and `moves` and may give `level`,
/// `evs` and `ivs` (default level 100, every EV 255, every IV 30, ability
/// `noability`, as the carried sets). Returns the entries with `packed`.
function readTargeted(file) {
  if (!fs.existsSync(file)) throw new Error(`no targeted battles at ${file}`);
  const all = (n) => ({ hp: n, atk: n, def: n, spa: n, spd: n, spe: n });
  const set = (m) => ({
    ability: 'noability', level: 100, evs: all(255), ivs: all(30), ...m,
  });
  const list = JSON.parse(fs.readFileSync(file, 'utf8'));
  const names = new Set();
  for (const t of list) {
    if (names.has(t.name)) throw new Error(`targeted battle ${t.name} is listed twice`);
    names.add(t.name);
    t.packed = [Teams.pack(t.p1.map(set)), Teams.pack(t.p2.map(set))];
  }
  return list;
}

/// One team: 1-6 mons. Each mon: 1-4 moves (distinct), level 40-100,
/// a random species, IVs/EVs (gen 3 also picks item/ability + status).
function makeWideTeam({ gen, prng, focus, ported, self }) {
  const speciesPool = ported.species;
  const movePool = ported.moves;
  const itemPool = ported.items;
  const abilityPool = ported.abilities;

  const teamSize = 1 + Math.floor(prng.random() * 6);
  const pickBag = new Set(speciesPool);
  const set = [];
  for (let i = 0; i < teamSize; i++) {
    if (pickBag.size === 0) break;
    const speciesId = pickOnce(prng, pickBag);
    const nMoves = 1 + Math.floor(prng.random() * 4);
    const moveBag = new Set(movePool);
    const setMoves = [];
    for (let j = 0; j < nMoves; j++) {
      if (moveBag.size === 0) break;
      const mv = pickOnce(prng, moveBag);
      setMoves.push(mv);
    }
    if (setMoves.length === 0) {
      // Every move was filtered out for this species: skip; the team shrinks
      // by one in that case, which is a known shortcoming of the wide
      // generator and matches what showdown_fuzz.rs does when a species
      // does not learn any ported move.
      continue;
    }
    const level = 40 + Math.floor(prng.random() * 61);
    const ivs = {};
    for (const s of ['hp', 'atk', 'def', 'spa', 'spd', 'spe']) {
      ivs[s] = Math.floor(prng.random() * 32);
    }
    // EVs: rotating stat start, total <= 510. Same shape as showdown_fuzz.rs.
    const evs = { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 };
    let budget = 510;
    const start = Math.floor(prng.random() * 6);
    for (let k = 0; k < 6; k++) {
      const statKeys = ['hp', 'atk', 'def', 'spa', 'spd', 'spe'];
      const stat = statKeys[(start + k) % 6];
      const max = Math.min(255, budget);
      const ev = Math.floor(prng.random() * (max + 1));
      evs[stat] = ev;
      budget -= ev;
    }
    const nature = gen === 3 ? NATURES[Math.floor(prng.random() * 25)] : '';
    const gender = pickGender(prng, speciesId, gen);
    let item = '';
    let ability = '';
    if (gen === 3) {
      // Empty most of the time (60%), else a pick from itemPool. The draw
      // ALWAYS happens so the generator stream is independent of the
      // species draw -- matching showdown_fuzz.rs.
      const empty = prng.random() * 100 < 60;
      if (!empty && itemPool.length) {
        item = pickFromArr(prng, itemPool);
      }
      const slot = Math.floor(prng.random() * 2);
      const none = prng.random() * 100 < 30;
      if (!none && abilityPool.length) {
        ability = pickAbility(prng, speciesId, gen, slot, abilityPool);
      }
      // Against ps_sim_cli the team may only carry ported abilities. A
      // packed empty ability is not "no ability": PS's `Teams.unpack`
      // turns it into the species' first ability (from the base dex),
      // which is usually one with handlers. So when neither the species'
      // abilities nor the 30% "none" roll gave a ported one, draw one from
      // the pool (custom games do not validate the set). `--self` keeps
      // the old stream.
      if (!self && !ability && abilityPool.length) {
        ability = pickFromArr(prng, abilityPool);
      }
    }
    const statuses = WIDE_STATUSES[gen];
    const stPick = statuses[Math.floor(prng.random() * statuses.length)];
    set.push({
      species: speciesId,
      level,
      moves: setMoves,
      ivs,
      evs,
      nature,
      gender,
      item: gen === 1 ? '' : item,
      ability: gen === 1 ? 'noability' : ability,
      status: stPick,
    });
  }
  const { moves: focusMoves, abilities: focusAbilities, items: focusItems } = splitFocus(gen, focus);
  if (focusMoves.length) {
    // A team that carries none of the focus moves gets one: a random move
    // of a random mon. A moves-only focus draws exactly as it always has.
    const hit = set.some((m) => m.moves.some((mv) => focusMoves.includes(mv)));
    if (!hit && set.length && set[0].moves.length) {
      const want = focusMoves[Math.floor(prng.random() * focusMoves.length)];
      const idx = Math.floor(prng.random() * set.length);
      set[idx].moves[Math.floor(prng.random() * set[idx].moves.length)] = want;
    }
  }
  if (focusAbilities.length && gen !== 1) {
    // Gen 3: an ability id becomes a random mon's ability (custom games do
    // not validate the set).
    const hit = set.some((m) => focusAbilities.includes(m.ability));
    if (!hit && set.length) {
      const want = focusAbilities[Math.floor(prng.random() * focusAbilities.length)];
      set[Math.floor(prng.random() * set.length)].ability = want;
    }
  }
  if (focusItems.length && gen !== 1) {
    const hit = set.some((m) => focusItems.includes(m.item));
    if (!hit && set.length) {
      const want = focusItems[Math.floor(prng.random() * focusItems.length)];
      set[Math.floor(prng.random() * set.length)].item = want;
    }
  }
  return set;
}

/// Split the `--focus` ids into moves, abilities and items. `move:<id>`,
/// `ability:<id>` and `item:<id>` name the kind; a bare id is a move when
/// the gen's dex has that move, else (gen 3) an ability when it has that
/// ability, else an item when it has that item, else a move (the old
/// moves-only reading). A moves-only focus draws exactly what it drew
/// before abilities and items could be named.
function splitFocus(gen, focus) {
  const out = { moves: [], abilities: [], items: [] };
  if (!focus) return out;
  const dex = Dex.mod(`gen${gen}`);
  for (const f of focus) {
    if (f.startsWith('move:')) { out.moves.push(f.slice(5)); continue; }
    if (f.startsWith('ability:')) { out.abilities.push(f.slice(8)); continue; }
    if (f.startsWith('item:')) { out.items.push(f.slice(5)); continue; }
    if (dex.moves.get(f).exists) out.moves.push(f);
    else if (gen !== 1 && dex.abilities.get(f).exists) out.abilities.push(f);
    else if (dex.items.get(f).exists) out.items.push(f);
    else out.moves.push(f);
  }
  return out;
}

const NATURES = [
  'hardy', 'lonely', 'brave', 'adamant', 'naughty', 'bold', 'docile',
  'relaxed', 'impish', 'lax', 'timid', 'hasty', 'serious', 'jolly',
  'naive', 'modest', 'mild', 'quiet', 'bashful', 'rash', 'calm',
  'gentle', 'sassy', 'careful', 'quirky',
];

/// Gen 1's status pool (sleep and freeze stall on clocks the engines pin
/// differently during the wide run; the original showdown_fuzz.rs leaves
/// them off too).
const WIDE_STATUSES = {
  1: [null, null, null, 'brn', 'psn', 'par'],
  3: [null, null, null, 'brn', 'psn', 'par', 'tox', 'slp', 'frz'],
};

function pickGender(prng, species, gen) {
  const dex = Dex.mod(`gen${gen}`);
  const sp = dex.species.get(species);
  if (!sp || !sp.exists) return '';
  const ratio = sp.genderRatio;
  if (ratio === undefined || ratio === null) return '';
  if (ratio === 'M') return 'M';
  if (ratio === 'F') return 'F';
  if (ratio === 'N') return '';
  if (typeof ratio === 'object') {
    return prng.random() * 100 < ratio.M ? 'M' : 'F';
  }
  return prng.random() * 100 < ratio ? 'M' : 'F';
}

function pickAbility(prng, speciesId, gen, slot, abilityPool) {
  const sp = Dex.mod(`gen${gen}`).species.get(speciesId);
  if (!sp || !sp.exists) return '';
  const candidates = [sp.abilities['0'], sp.abilities['1'], sp.abilities['H'] || ''];
  const pair = [candidates[slot], candidates[1 - slot]];
  for (const a of pair) {
    if (a && abilityPool.includes(a.toLowerCase())) return a.toLowerCase();
  }
  return '';
}

function pickFromArr(prng, arr) {
  if (!arr.length) return '';
  return arr[Math.floor(prng.random() * arr.length)];
}

function pickOnce(prng, bag) {
  const snapshot = [...bag];
  if (!snapshot.length) return '';
  const out = snapshot[Math.floor(prng.random() * snapshot.length)];
  bag.delete(out);
  return out;
}

/// PS's PRNG with our 4 u16 battle seed shape exposed. Driver uses it for
/// the choice seed too; teams.js uses it for the team seed.
/// `battleSeed()` returns the outer seed so PS's random-battle generator
/// can build its own PRNG from the exact same root.
class TeamsPrng {
  constructor(seed) {
    // Lazy-require to dodge a require cycle; ps-sim's PRNG is small.
    const { PRNG } = require('pokemon-showdown');
    // PS's PRNG takes a comma-string OR an array (which it joins).
    // The driver's team seed is `[u16, u16]`.
    this._outerSeed = seed;
    this._prng = new PRNG(seed);
  }
  battleSeed() { return this._outerSeed; }
  random() { return this._prng.random(); }
  random(n) { return this._prng.random(n); }
}

function makePairFromPRNG({ gen, prng, teams }) {
  const [a, b] = teams;
  if (!a) throw new Error('random team generation returned null');
  if (!b) throw new Error('random team generation returned null on second call');
  return { p1: a, p2: b, packed: [Teams.pack(a), Teams.pack(b)] };
}

module.exports = {
  readTargeted,
  buildPair,
  readCarried,
  _internals: { makeWideTeam, buildRandom, buildWide, splitFocus },
  // Public surface used by diff.js and the cargo tests in
  // crates/ps_harness/tests/protocol_diff.rs.
  NATURES, WIDE_STATUSES,
};
