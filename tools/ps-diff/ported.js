#!/usr/bin/env node
'use strict';
// tools/ps-diff/ported.js
//
// Bridge between `ps_sim_cli --ported <gen>` and the diff harness.
// One place that caches the output of `ps_sim-cli --ported <gen>`, parses
// it into per-kind pools (moves, species, items, abilities, conditions,
// rules), and serves them to teams.js and the wide smoke check.
//
// Output format from `ps_sim_cli --ported <gen>`:
//   one `<kind>:<id>` per line, where kind is one of
//     move, species, item, ability, condition, rule
//   and id is the lowercase PS effect id (`recover`, `pikachu`, ...).
// Empty output is a valid signal: the harness treats that as
// `ps_sim has no ported pipeline for gen N`, so:
//   * `--teams wide` against `ps_sim-cli` exits 2 immediately.
//   * `--teams wide --self` falls back to the whole dex.
//
// The deps cache invalidates on a `(psCli, gen)` pair; restarting the
// driver (e.g. running the protocol-diff test) refills it.

const fs = require('fs');
const { execFileSync, spawnSync } = require('child_process');
const { Dex } = require('pokemon-showdown');

const cache = new Map();

/// Read `ps_sim_cli --ported <gen>` once, parse, and return the per-kind
/// pools.
/// `opts`: { gen, self, psCli }
/// `self`: true -> `--self` mode (whole dex).
/// `psCli`: path to the rust binary; `/dev/null` if missing.
function getPool(opts) {
  const { gen, self, psCli } = opts;

  // --self: decked-out list (every species, every move the gen has).
  if (self) {
    return fullPool(gen);
  }

  if (!psCli) {
    // Caller passed neither --self nor --ps-cli; the harness is supposed
    // to catch that earlier. Treat as --self so a programming error does
    // not silently fail wider than reported.
    return fullPool(gen);
  }

  const cacheKey = `${psCli}|${gen}`;
  if (cache.has(cacheKey)) return cache.get(cacheKey);

  let raw;
  try {
    raw = execFileSync(psCli, ['--ported', String(gen)], { encoding: 'utf8' });
  } catch (e) {
    throw new Error(
      `ps_diff: cannot invoke ${psCli} --ported ${gen}: ${e.message}`,
    );
  }
  const lines = String(raw).split('\n').map((l) => l.trim()).filter((l) => l.length > 0);
  const out = parsePortedLines(gen, lines);
  cache.set(cacheKey, out);
  return out;
}

/// Parse the `<kind>:<id>` lines emitted by `ps_sim_cli --ported <gen>`.
/// Returns { species, moves, items, abilities, conditions, rules, empty }.
/// `empty` is true when no ids are reported (which means no ported
/// pipeline for that gen, per the brief).
function parsePortedLines(gen, lines) {
  const KINDS = ['species', 'move', 'item', 'ability', 'condition', 'rule'];
  if (lines.length === 0) {
    return makeEmpty(gen);
  }
  const pools = { species: [], moves: [], items: [], abilities: [], conditions: [], rules: [] };
  for (const line of lines) {
    const idx = line.indexOf(':');
    if (idx < 0) continue;
    const kind = line.slice(0, idx);
    const id = line.slice(idx + 1).toLowerCase();
    if (!KINDS.includes(kind)) continue;
    switch (kind) {
      case 'species': pools.species.push(id); break;
      case 'move': pools.moves.push(id); break;
      case 'item': pools.items.push(id); break;
      case 'ability': pools.abilities.push(id); break;
      case 'condition': pools.conditions.push(id); break;
      case 'rule': pools.rules.push(id); break;
    }
  }
  return { ...pools, empty: lines.length === 0 };
}

function makeEmpty(gen) {
  return {
    species: [], moves: [], items: [], abilities: [],
    conditions: [], rules: [], empty: true,
  };
}

/// The whole-dex pool used by `--self --teams wide`. No whitelist, no
/// filter: PS plays whatever falls out the same way twice; this is the
/// harness's own self-test path.
function fullPool(gen) {
  const dex = Dex.mod(`gen${gen}`);
  const species = dex.species.all()
    .filter((s) => s.exists && !s.isNonstandard && s.num > 0)
    .map((s) => s.id);
  const moves = dex.moves.all()
    .filter((m) => m.exists && !m.isNonstandard && m.id !== 'struggle')
    .map((m) => m.id);
  const items = gen === 3 ? dex.items.all()
    .filter((it) => it.exists && !it.isNonstandard)
    .map((it) => it.id) : [];
  const abilities = gen === 3 ? dex.abilities.all()
    .filter((a) => a.exists && !a.isNonstandard && a.id !== 'noability')
    .map((a) => a.id) : [];
  return {
    species, moves, items, abilities,
    conditions: [], rules: [],
    empty: false,
  };
}

/// Cache invalidation entry point (used by the cargo test that swaps
/// `ps_sim_cli` between runs).
function clear() { cache.clear(); }

module.exports = {
  spawnPorted: { getPool, clear },
  parsePortedLines,
  fullPool,
};
