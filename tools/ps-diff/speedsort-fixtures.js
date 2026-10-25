#!/usr/bin/env node
/* eslint-disable no-console */
// tools/ps-diff/speedsort-fixtures.js
//
// Generates `crates/ps_sim/tests/fixtures/speed_sort.json`, the reference
// for `ps_sim::events::speed_sort` (P1-5 acceptance).
//
// It builds one PS `Battle` (gen3customgame) with a fixed numeric seed and
// calls its own `speedSort` (default comparator, `comparePriority`) on 500
// synthetic lists of 2 to 12 entries, all against the same battle PRNG, in
// order. Values come from small ranges so ties are common. Each field is
// sometimes missing; `order` is also sometimes 0, which `comparePriority`
// reads as `order || 4294967296`, the same as missing.
//
// For every list it records the entries as given (missing fields omitted),
// the permutation PS produced (indices into the input) and
// `battle.prng.getSeed()` after the sort.
//
// Output schema:
//
//   {
//     "seed": [a, b, c, d],
//     "lists": [
//       {
//         "items": [{"order": 1, "priority": 0, ...}, ...],
//         "perm": [2, 0, 1, ...],
//         "seed_after": [a, b, c, d]
//       },
//       ...
//     ]
//   }
//
// Usage:
//   node tools/ps-diff/speedsort-fixtures.js > crates/ps_sim/tests/fixtures/speed_sort.json
//
// The lists come from a fixed LCG (not Math.random), so reruns are
// byte-identical.

'use strict';

const path = require('path');
const { Battle } = require(path.join(__dirname, 'node_modules', 'pokemon-showdown'));

const SEED = [0x1234, 0x5678, 0x9abc, 0xdef0];
const LISTS = 500;

// glibc-style LCG over 31 bits, independent of the battle PRNG.
let lcg = 0x2545f491;
function rand(n) {
  lcg = (Math.imul(lcg, 1103515245) + 12345) & 0x7fffffff;
  return (lcg >>> 8) % n;
}

// A field is missing 1 time in `missingOdds`, else drawn from `values`.
function field(values, missingOdds) {
  if (rand(missingOdds) === 0) return undefined;
  return values[rand(values.length)];
}

function makeItem() {
  const item = {};
  const order = field([0, 0, 1, 2], 3);
  const priority = field([-1, 0, 0, 1], 3);
  const speed = field([0, 1, 2], 4);
  const subOrder = field([0, 0, 1], 3);
  const effectOrder = field([0, 0, 1], 3);
  if (order !== undefined) item.order = order;
  if (priority !== undefined) item.priority = priority;
  if (speed !== undefined) item.speed = speed;
  if (subOrder !== undefined) item.subOrder = subOrder;
  if (effectOrder !== undefined) item.effectOrder = effectOrder;
  return item;
}

function buildFixture() {
  const battle = new Battle({ formatid: 'gen3customgame', seed: SEED.slice() });
  const lists = [];
  for (let l = 0; l < LISTS; l++) {
    const len = 2 + rand(11);
    const items = [];
    for (let i = 0; i < len; i++) items.push(makeItem());
    const tagged = items.map((item, i) => Object.assign({ tag: i }, item));
    battle.speedSort(tagged);
    lists.push({
      items,
      perm: tagged.map(t => t.tag),
      seed_after: battle.prng.getSeed().split(',').map(Number),
    });
  }
  return { seed: SEED, lists };
}

function main() {
  process.stdout.write(JSON.stringify(buildFixture()));
  process.stdout.write('\n');
}

if (require.main === module) {
  main();
}

module.exports = { buildFixture };
