#!/usr/bin/env node
'use strict';

// Records REAL Pokemon Showdown battle transcripts for the ps_protocol text
// codec round-trip test (task P1-4).
//
// The codec has to turn PS's line protocol into typed messages and back, byte
// for byte. The only honest way to prove that is against PS's own output, so
// this script plays battles with PS's own `BattleStream` and writes down
// everything that went in and everything that came out.
//
// What it plays:
//   * gen 1 and gen 3, the two formats the device plays
//     (`gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod` and
//     `gen3customgame`),
//   * random battle teams from PS's own generators (`gen1randombattle`,
//     `gen3randombattle`),
//   * random but always legal choices, drawn from PS's own request JSON,
//   * fixed seeds, so the same seeds give the same logs every time.
//
// Transcript format (one battle per file, lines):
//
//   >start {"formatid":"...","seed":[a,b,c,d]}      a command we sent
//   >player p1 {"name":"...","team":"<packed>"}     a command we sent
//   >p1 move 1                                     a command we sent
//   >update                                        the start of an update block
//   |player|p1|Player 1||                          ... the block's lines
//   >sideupdate p1                                 the start of a side update
//   |request|{"active":[...]}                      ... the block's lines
//
// A line that starts with `>` is a record header; every other line belongs to
// the block the last header opened. PS log lines always start with `|`, so the
// split is unambiguous, and the empty line PS writes after a one-sided split
// survives as an empty content line.
//
// Usage:
//   node tools/ps-diff/record-logs.js            write the fixtures
//   node tools/ps-diff/record-logs.js --check    regenerate and compare
//   node tools/ps-diff/record-logs.js --out DIR  write somewhere else

const fs = require('fs');
const path = require('path');
const {BattleStream, Teams, Dex} = require('pokemon-showdown/dist/sim');

const REPO = path.resolve(__dirname, '..', '..');
const OUT_DIR = path.join(REPO, 'crates', 'ps_protocol', 'tests', 'fixtures');

// How many battles per generation the recorder plays, and how many of them
// are committed as fixtures. The task asks for 60 played and 20 committed.
const PLAYED_PER_GEN = 60;
const COMMITTED_PER_GEN = 20;

// A battle that has not ended after this many turns is cut off; the codec
// does not care and the diff harness uses the same cap.
const MAX_TURNS = 300;

// The master seed. Everything else is derived from it, so the whole corpus is
// a function of this one number.
const MASTER_SEED = 0x5eed1e;

// The formats we play, with the custom rules the device plays.
const GENS = [
  {gen: 1, formatid: 'gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod'},
  {gen: 3, formatid: 'gen3customgame'},
];

// ---------------------------------------------------------------------------
// Deterministic helpers
// ---------------------------------------------------------------------------

// mulberry32: a tiny PRNG with its own state, so the choice draws do not
// touch (or depend on) PS's PRNG.
function mulberry32(a) {
  let s = a >>> 0;
  return function () {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function u16(rng) {
  return Math.floor(rng() * 65536) & 0xffff;
}

// PS's Gen 5 PRNG takes a four-number seed. Every one of them has to fit in
// 16 bits, so the corpus never depends on how PS rounds a bigger number.
function seedOf(rng) {
  return [u16(rng), u16(rng), u16(rng), u16(rng)];
}

// ---------------------------------------------------------------------------
// Playing one battle
// ---------------------------------------------------------------------------

// Pick a legal choice from a PS request. Returns null when the side has
// nothing to decide (a `wait` request).
function chooseFor(request, rng) {
  if (!request || request.wait) return null;
  const pick = (n) => Math.floor(rng() * n) % n;

  if (Array.isArray(request.forceSwitch) && request.forceSwitch.some(Boolean)) {
    const candidates = [];
    (request.side?.pokemon || []).forEach((pokemon, i) => {
      if (pokemon.active) return;
      if (pokemon.condition === '0 fnt') return;
      candidates.push(i + 1);
    });
    if (!candidates.length) return 'default';
    return `switch ${candidates[pick(candidates.length)]}`;
  }

  const active = request.active && request.active[0];
  if (!active) return 'default';
  const candidates = [];
  (active.moves || []).forEach((move, i) => {
    if (move.disabled) return;
    candidates.push(i + 1);
  });
  if (!candidates.length) return 'default';
  return `move ${candidates[pick(candidates.length)]}`;
}

// Play one battle and return its transcript as an array of lines.
async function playBattle(gen, formatid, index) {
  const seedRng = mulberry32((MASTER_SEED + gen * 7919 + index * 104729) >>> 0);
  const battleSeed = seedOf(seedRng);
  const teamSeed1 = seedOf(seedRng);
  const teamSeed2 = seedOf(seedRng);
  const choiceRng = mulberry32((MASTER_SEED ^ (gen * 2654435761 + index * 40503)) >>> 0);

  const team1 = Teams.getGenerator(`gen${gen}randombattle`, teamSeed1).getTeam();
  const team2 = Teams.getGenerator(`gen${gen}randombattle`, teamSeed2).getTeam();
  const packed1 = Teams.pack(team1);
  const packed2 = Teams.pack(team2);

  const lines = [];
  const start = `>start ${JSON.stringify({formatid, seed: battleSeed})}`;
  const player1 = `>player p1 ${JSON.stringify({name: 'Player 1', team: packed1})}`;
  const player2 = `>player p2 ${JSON.stringify({name: 'Player 2', team: packed2})}`;
  lines.push(start, player1, player2);

  const stream = new BattleStream();
  const iter = stream[Symbol.asyncIterator]();
  stream.write(`${start}\n${player1}\n${player2}\n`);

  let turns = 0;
  for (;;) {
    const {value, done} = await iter.next();
    if (done) break;
    const nl = value.indexOf('\n');
    const type = value.slice(0, nl);
    const data = value.slice(nl + 1);
    if (type === 'update') {
      lines.push('>update');
      const block = data.split('\n');
      for (const line of block) {
        lines.push(line);
        if (line.startsWith('|turn|')) turns++;
      }
    } else if (type === 'sideupdate') {
      const nl2 = data.indexOf('\n');
      const side = data.slice(0, nl2);
      const body = data.slice(nl2 + 1);
      lines.push(`>sideupdate ${side}`);
      const block = body.split('\n');
      for (const line of block) lines.push(line);
      const requestLine = block.find((line) => line.startsWith('|request|'));
      if (requestLine) {
        let request = null;
        try {
          request = JSON.parse(requestLine.slice('|request|'.length));
        } catch (e) {
          request = null;
        }
        const choice = chooseFor(request, choiceRng);
        if (choice) {
          lines.push(`>${side} ${choice}`);
          stream.write(`>${side} ${choice}\n`);
        }
      }
    }
    if (turns > MAX_TURNS) {
      stream.destroy();
      break;
    }
  }
  return lines;
}

// ---------------------------------------------------------------------------
// The name table
// ---------------------------------------------------------------------------

// The codec turns names into dex numbers and back, so the round-trip test
// needs a name table. This dumps the dex entries of both generations we
// play: id and display name, one per line, one kind per section.
//
// `kind<TAB>id<TAB>name`, kinds: move, species, item, ability, type, nature.
function dumpNames() {
  const rows = [];
  const seen = new Set();
  const add = (kind, id, name) => {
    const key = `${kind}\t${id}`;
    if (seen.has(key)) return;
    seen.add(key);
    rows.push(`${kind}\t${id}\t${name}`);
  };
  for (const gen of [1, 3]) {
    const dex = Dex.mod(`gen${gen}`);
    for (const move of dex.moves.all()) {
      if (!move.exists || move.isNonstandard) continue;
      add('move', move.id, move.name);
    }
    for (const species of dex.species.all()) {
      if (!species.exists || species.isNonstandard || species.num <= 0) continue;
      add('species', species.id, species.name);
    }
    for (const item of dex.items.all()) {
      if (!item.exists || item.isNonstandard) continue;
      add('item', item.id, item.name);
    }
    for (const ability of dex.abilities.all()) {
      if (!ability.exists || ability.isNonstandard) continue;
      add('ability', ability.id, ability.name);
    }
  }
  const base = Dex.mod('gen3');
  for (const type of base.types.all()) {
    if (!type.exists) continue;
    add('type', type.id, type.name);
  }
  for (const nature of base.natures.all()) {
    if (!nature.exists) continue;
    add('nature', nature.id, nature.name);
  }
  return rows.join('\n') + '\n';
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function record() {
  const files = {};
  for (const {gen, formatid} of GENS) {
    const lines = [];
    for (let index = 0; index < PLAYED_PER_GEN; index++) {
      lines.push(`#battle gen=${gen} index=${index}`);
      const battle = await playBattle(gen, formatid, index);
      lines.push(...battle);
    }
    files[`gen${gen}.log`] = lines.join('\n') + '\n';
  }
  files['names.tsv'] = dumpNames();
  return files;
}

async function main() {
  const argv = process.argv.slice(2);
  const check = argv.includes('--check');
  const outIndex = argv.indexOf('--out');
  const outDir = outIndex >= 0 ? path.resolve(argv[outIndex + 1]) : OUT_DIR;

  // PS stamps the battle log with the wall clock. Freeze it so the corpus is
  // a function of the seeds alone; the codec still sees a real timestamp.
  const frozenNow = 1700000000000;
  const realNow = Date.now;
  Date.now = () => frozenNow;

  let files;
  try {
    files = await record();
  } finally {
    Date.now = realNow;
  }

  fs.mkdirSync(outDir, {recursive: true});
  let bad = 0;
  for (const [name, content] of Object.entries(files)) {
    const target = path.join(outDir, name);
    if (check) {
      const old = fs.existsSync(target) ? fs.readFileSync(target, 'utf8') : null;
      if (old === content) {
        console.log(`ok       ${name}`);
      } else {
        bad++;
        console.log(`DIFFERS  ${name}`);
        if (old === null) {
          console.log(`         missing ${target}`);
        } else {
          const a = old.split('\n');
          const b = content.split('\n');
          let shown = 0;
          for (let i = 0; i < Math.max(a.length, b.length) && shown < 10; i++) {
            if (a[i] !== b[i]) {
              console.log(`         line ${i + 1}:`);
              console.log(`           old: ${JSON.stringify(a[i])}`);
              console.log(`           new: ${JSON.stringify(b[i])}`);
              shown++;
            }
          }
        }
      }
      continue;
    }
    fs.writeFileSync(target, content);
    const kb = (Buffer.byteLength(content) / 1024).toFixed(1);
    console.log(`wrote    ${path.relative(REPO, target)} (${kb} KB)`);
  }
  if (check) {
    if (bad) {
      console.error(`${bad} file(s) differ`);
      process.exit(1);
    }
    console.log('all fixtures reproduce byte for byte');
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
