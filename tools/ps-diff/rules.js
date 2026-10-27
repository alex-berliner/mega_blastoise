'use strict';
// tools/ps-diff/rules.js
//
// `Cartridge Trunc` as a PS format rule (design section 3), so the harness
// can name the Gen 1 rule set in the `>start` formatid that both sides read:
//
//   gen1customgame                       cartridge truncation (the default)
//   gen1customgame@@@!Cartridge Trunc    plain math
//
// Stock PS decides `battle.trunc` per format: the Battle constructor sets
// `this.trunc = this.dex.trunc` (the cartridge's bit-width wrap) and then
// applies `format.battle`, which in every Custom Game is
// `{trunc: Math.trunc}` (`config/formats.ts`). `install()` gives both Gen 1
// formats the harness plays (`gen1customgame`, `gen1randombattle`) that
// `Math.trunc` and appends `Cartridge Trunc` to their rulesets; the rule's
// `onBegin` puts `this.dex.trunc` back. With the rule in effect
// `gen1randombattle` is exactly stock PS's (`Dex.trunc`); `!Cartridge Trunc`
// repeals it the way PS repeals any rule (`sim/dex-formats.ts getRuleTable`,
// which throws when the rule is not in effect). `onBegin` runs in `start()`,
// before any damage or PP math; the stats computed at `setPlayer` call
// `trunc` without `bits` on small non-negative numbers, where both agree.
// The rule announces nothing (no `|rule|` line) and has no other handler, so
// it is not a pseudo-weather either (`Battle` constructor, `hasEventHandler`).
//
// `ps_sim` mirrors this: `Rule::CartridgeTrunc` ends every Gen 1 ruleset
// (`crates/ps_sim/src/battle.rs rule_table`) and decides `Battle::trunc`.

const { Dex } = require('pokemon-showdown');
const { Format } = require('pokemon-showdown/dist/sim/dex-formats');

const RULE_NAME = 'Cartridge Trunc';
const RULE = {
  effectType: 'Rule',
  name: RULE_NAME,
  desc: "Gen 1's bit-width truncation (stat and PP rollover) in place of Math.trunc",
  onBegin() {
    this.trunc = this.dex.trunc;
  },
};
const GEN1_FORMATS = ['gen1customgame', 'gen1randombattle'];

let installed = false;

function install() {
  if (installed) return;
  installed = true;
  const data = Dex.data; // loads the base data and the format list
  const patched = {};
  for (const id of GEN1_FORMATS) {
    const entry = data.Rulesets[id];
    if (!entry) throw new Error(`ps-diff rules: no format ${id}`);
    patched[id] = {
      ...entry,
      ruleset: [...entry.ruleset, RULE_NAME],
      battle: { ...(entry.battle || {}), trunc: Math.trunc },
    };
  }
  // Every dex whose data is already loaded holds its own copy of the table;
  // mods loaded later inherit the base's entries.
  for (const dex of Object.values(Dex.dexes)) {
    if (!dex.dataCache) continue;
    dex.dataCache.Rulesets.cartridgetrunc = RULE;
    for (const id of GEN1_FORMATS) dex.dataCache.Rulesets[id] = patched[id];
  }
  // `Dex.formats.get(id)` without `@@@` reads the format list's cache.
  const list = Dex.formats.formatsListCache;
  for (const id of GEN1_FORMATS) {
    const format = new Format(patched[id]);
    const i = list.findIndex((f) => f.id === id);
    if (i >= 0) list[i] = format;
    Dex.formats.rulesetCache.set(id, format);
  }
  if (!Dex.mod('gen1').data.Rulesets.cartridgetrunc) {
    throw new Error('ps-diff rules: Cartridge Trunc did not reach the gen1 mod');
  }
}

/// The formatid for `base` under the Gen 1 rule set `trunc`
/// (`cartridge` or `math`); other gens take no `trunc`.
function withTrunc(base, gen, trunc) {
  if (gen !== 1 || trunc === 'cartridge') return base;
  if (trunc !== 'math') throw new Error(`ps-diff rules: unknown trunc ${trunc}`);
  return `${base}${base.includes('@@@') ? ',' : '@@@'}!${RULE_NAME}`;
}

module.exports = { install, withTrunc, RULE_NAME };
