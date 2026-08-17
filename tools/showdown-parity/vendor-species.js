// Regenerates gen3_battle/vendor/showdown_gen3_species.json from the pinned
// Pokemon Showdown checkout, so the dex the engine compiles against and the
// dex the parity harness runs against can never drift apart.
//
//   node tools/showdown-parity/vendor-species.js > gen3_battle/vendor/showdown_gen3_species.json
const { Dex } = require('pokemon-showdown/dist/sim');

const dex = Dex.mod('gen3');
const out = [];
for (const species of dex.species.all()) {
  if (species.num < 1 || species.num > 386) continue;
  if (species.isNonstandard) continue;
  const types = species.types.slice();
  const abilities = [species.abilities['0'] || '', species.abilities['1'] || '']
    .map((name) => dex.abilities.get(name).id || '');
  out.push({
    id: species.id,
    name: species.name,
    types,
    baseStats: species.baseStats,
    weightkg: species.weightkg,
    // 'M', 'F' or 'N' where the species is fixed; empty where it can be
    // either. Attract is the only gen 3 mechanic that reads it, and it reads
    // it as a plain comparison.
    gender: species.gender || '',
    abilities,
  });
}
out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
process.stdout.write(JSON.stringify(out) + '\n');
