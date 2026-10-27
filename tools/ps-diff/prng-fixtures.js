#!/usr/bin/env node
/* eslint-disable no-console */
// tools/ps-diff/prng-fixtures.js
//
// Generates `crates/ps_sim/tests/fixtures/prng.json`, the byte-exact
// reference of Pokemon Showdown's PRNG behavior used by the
// `ps_sim::prng` parity test (P1-2 acceptance).
//
// For each of 100 deterministic numeric seeds, it constructs a
// `new PRNG(seed)` (numeric seeds land on `Gen5RNG` per
// `tools/ps-diff/node_modules/pokemon-showdown/sim/prng.ts` and
// `setSeed`'s numeric-seeds branch) and drives 1,000 method calls in a
// fixed order that hits every public PS-PRNG method with the edge
// arguments the brief requires (`random(1)`, `random(2)`, `random(2^16)`,
// `random(1, 2)`). After every 100 method calls it captures the PRNG's
// `getSeed()` so the Rust side can pin both the per-call results and the
// internal state at well-spaced checkpoints.
//
// Output schema (JSON-stable, byte-identical across reruns):
//
//   {
//     "block_size": <int>,
//     "calls_per_seed": 1000,
//     "seeds": [
//       {
//         "seed": [a, b, c, d],
//         "calls": [
//           // one entry per method invocation; "shuffle" carries a
//           // 6-element array; "sample" carries a string; "random_chance"
//           // carries a boolean; "random" carries an f64; everything
//           // else carries a uint32.
//           ...
//         ],
//         "checks": [
//           {"after": 100, "seed":[a,b,c,d]},
//           ...
//         ]
//       },
//       ...
//     ]
//   }
//
// Usage:
//   node tools/ps-diff/prng-fixtures.js > crates/ps_sim/tests/fixtures/prng.json
//   node tools/ps-diff/prng-fixtures.js > /tmp/run1.json
//   node tools/ps-diff/prng-fixtures.js > /tmp/run2.json
//   diff /tmp/run1.json /tmp/run2.json   # must be empty
//
// The script deliberately does not use `Math.random()` (would vary
// between runs). Seeds are derived from a fixed schedule so the JSON is
// bit-identical on every run.

'use strict';

const path = require('path');
const { PRNG } = require(path.join(
  __dirname, 'node_modules', 'pokemon-showdown'));

const CALLS_PER_SEED = 1000;
const CHECK_EVERY = 100;

// One logical PRNG method call per entry. `shuffle` is one method
// invocation that internally consumes `end - 1` frames.
//
//   op               result type
//   --------------   ----------------------------------
//   random           uint32 = rng.next() that backs the f64
//                     (`random() = next / 2^32`; the integer round-trips
//                     through JSON bit-exactly; the Rust side recomputes
//                     `f64::from(next) / 4_294_967_296.0`, which IEEE 754
//                     rounding makes equal to PS's `next / 2**32`.)
//   random_n         uint32 in [0, n)
//   random_range     uint32 in [m, n)
//   random_chance    bool
//   sample           one of the three items (string)
//   shuffle          array of numbers (the post-shuffle order)
//
const BLOCK = [
  { kind: 'random' },
  { kind: 'random_n', n: 1 },
  { kind: 'random_n', n: 2 },
  { kind: 'random_n', n: 65536 },
  { kind: 'random_range', m: 1, n: 2 },
  { kind: 'random_range', m: 2, n: 65537 },
  { kind: 'random_chance', num: 1, den: 2 },
  { kind: 'random_chance', num: 1, den: 4 },
  { kind: 'sample', items: ['a', 'b', 'c'] },
  { kind: 'shuffle', items: [1, 2, 3, 4, 5, 6] },
];

function deriveSeeds(n) {
  // Deterministic 100 seeds, glibc-style LCG. Avoids 0-limb seeds and
  // spreads the four limbs across the 16-bit space; reproducible across
  // every Node major we run on this machine.
  const seeds = [];
  let s = 0xC0DE & 0xFFFF;
  for (let i = 0; i < n; i++) {
    s = ((s * 1103515245) + 12345) & 0xFFFF;
    const a = s;
    s = ((s * 1103515245) + 12345) & 0xFFFF;
    const b = s;
    s = ((s * 1103515245) + 12345) & 0xFFFF;
    const c = s;
    s = ((s * 1103515245) + 12345) & 0xFFFF;
    const d = s;
    seeds.push([a, b, c, d]);
  }
  return seeds;
}

function callOne(prng, blockIndex) {
  const b = BLOCK[blockIndex];
  switch (b.kind) {
    case 'random': {
      // Record the underlying integer so the fixture round-trips bit-
      // exactly through JSON. PS's own `random()` is
      // `result / 2**32` (IEEE 754 round-to-nearest) and the Rust side
      // computes the same `f64::from(next) / 4_294_967_296.0`; both give
      // the same f64 because the integer is identical.
      const r = prng.rng.next();
      return { op: 'random', result: r };
    }
    case 'random_n':
      return { op: 'random_n', n: b.n, result: prng.random(b.n) };
    case 'random_range':
      return { op: 'random_range', m: b.m, n: b.n, result: prng.random(b.m, b.n) };
    case 'random_chance':
      return {
        op: 'random_chance', num: b.num, den: b.den,
        result: prng.randomChance(b.num, b.den),
      };
    case 'sample': {
      const result = prng.sample(b.items);
      return { op: 'sample', items: b.items, result };
    }
    case 'shuffle': {
      const arr = b.items.slice();
      prng.shuffle(arr);
      return { op: 'shuffle', items: b.items, result: arr };
    }
    default:
      throw new Error(`unknown block kind: ${b.kind}`);
  }
}

function buildSeedCalls(seed) {
  const prng = new PRNG(seed);
  const calls = new Array(CALLS_PER_SEED);
  const checks = [];
  for (let n = 0; n < CALLS_PER_SEED; n++) {
    const entry = callOne(prng, n % BLOCK.length);
    calls[n] = entry;
    if ((n + 1) % CHECK_EVERY === 0) {
      checks.push({
        after: n + 1,
        seed: prng.getSeed().split(',').map(Number),
      });
    }
  }
  return { seed, calls, checks };
}

function buildFixture() {
  const seeds = deriveSeeds(100);
  return {
    block_size: BLOCK.length,
    calls_per_seed: CALLS_PER_SEED,
    seeds: seeds.map((seed) => buildSeedCalls(seed)),
  };
}

function main() {
  const out = process.stdout;
  // JSON.stringify with no spaces reproduces between runs; we still pin
  // key order manually below for absolute byte-identity.
  const fixture = buildFixture();
  out.write(JSON.stringify(fixture));
  out.write('\n');
}

if (require.main === module) {
  main();
}

module.exports = { buildFixture, deriveSeeds, BLOCK };
