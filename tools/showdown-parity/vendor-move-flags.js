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
  // What a Magic Coat throws back, and what a Snatch takes.
  m.reflectable = !!move.flags.reflectable;
  m.snatchable = !!move.flags.snatch;
  // Pressure charges its extra PP when the other side is among the move's
  // apparent targets, which is what the sim calls pressureTargets: a move
  // aimed at yourself or your own side never costs the extra point, and
  // neither does one aimed at the foe's SIDE rather than the mon on it.
  const ownSide = ['self', 'adjacentAlly', 'adjacentAllyOrSelf', 'allySide', 'allyTeam', 'allies'];
  m.pressured = !!move.flags.mustpressure ||
    !(ownSide.includes(move.target) || move.target === 'foeSide');
}
fs.writeFileSync(path, JSON.stringify(moves) + '\n');
process.stderr.write(`${moves.length} moves tagged\n`);
