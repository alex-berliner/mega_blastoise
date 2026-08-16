// Prints the RESOLVED gen 3 definition of one or more abilities, walking the
// mod chain the way the sim does. Reading data/abilities.ts directly gets the
// modern definition, which for most of these is not what gen 3 runs.
//
//   node tools/showdown-parity/show-ability.js static intimidate
const { Dex } = require('pokemon-showdown/dist/sim');

const dex = Dex.mod('gen3');
for (const name of process.argv.slice(2)) {
  const a = dex.abilities.get(name);
  if (!a.exists) {
    console.log(`-- ${name}: no such ability`);
    continue;
  }
  console.log(`-- ${a.id} (${a.name})`);
  for (const key of Object.keys(a).sort()) {
    if (!key.startsWith('on') && key !== 'suppressWeather' && key !== 'condition') continue;
    const v = a[key];
    if (key === 'condition') {
      for (const ck of Object.keys(v)) console.log(`  condition.${ck}: ${String(v[ck])}`);
    } else {
      console.log(`  ${key}: ${String(v)}`);
    }
  }
}
