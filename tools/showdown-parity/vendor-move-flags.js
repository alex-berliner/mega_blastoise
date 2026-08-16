// Adds the move flags the engine reads straight off the pinned Showdown dex
// to gen3_battle/vendor/showdown_gen3_moves.json, leaving every hand-curated
// field in that file alone. Run it in place:
//
//   node tools/showdown-parity/vendor-move-flags.js gen3_battle/vendor/showdown_gen3_moves.json
const fs = require('fs');
const { Dex } = require('pokemon-showdown/dist/sim');

const dex = Dex.mod('gen3');
const path = process.argv[2];
const moves = JSON.parse(fs.readFileSync(path, 'utf8'));
for (const m of moves) {
  const move = dex.moves.get(m.id);
  if (!move.exists) throw new Error(`no move ${m.id}`);
  m.sound = !!move.flags.sound;
  m.contact = !!move.flags.contact;
}
fs.writeFileSync(path, JSON.stringify(moves) + '\n');
process.stderr.write(`${moves.length} moves tagged\n`);
