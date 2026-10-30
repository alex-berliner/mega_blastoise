// The phone bridge without the phone: site/bridge/relay.js (the page's own
// relay) between a real Showdown server and the board's real game logic
// (crates/mega_blastoise_test/src/bin/bridge_board.rs, the DeviceRunner with
// stdin/stdout as the USB link). Plays one battle per generation asked for,
// the board's seat on the runner's random AI (`play N auto`).
//
//   node tools/bridge/relay_test.mjs --ws ws://localhost:8000/showdown/websocket --opponent
//   node tools/bridge/relay_test.mjs --ws wss://sim3.psim.us/showdown/websocket --gens 1,3
//
// Needs Node 22 or newer (the global WebSocket). `--opponent` adds a second
// guest that answers every request with `/choose default`; the live server
// never matches two guests from one IP (ladders.ts matchmakingOK), so against
// it the opponent is whoever else is searching.
// Exit status 0 only if every battle finished with no rejected choice, no
// undecoded line and no disconnect.

import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { Relay } from '../../site/bridge/relay.js';

const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(`--${name}`); return i < 0 ? dflt : args[i + 1]; };
const wsUrl = opt('ws', 'ws://localhost:8000/showdown/websocket');
const boardBin = opt('board', 'target/debug/bridge_board');
const gens = opt('gens', '1,3').split(',').map(Number);
const useOpponent = args.includes('--opponent');
const logFile = opt('log', '');
const battleTimeoutS = Number(opt('timeout', '300'));
const searchTimeoutS = Number(opt('search-timeout', '240'));

if (typeof WebSocket === 'undefined') { console.error('needs Node 22+ (global WebSocket)'); process.exit(2); }

// The board: the host build of the firmware's game loop, lines on stdio.
const child = spawn(boardBin, [], { stdio: ['pipe', 'pipe', 'inherit'] });
let lineCb = () => {};
let buf = '';
child.stdout.on('data', (d) => {
  buf += d.toString();
  let i;
  while ((i = buf.indexOf('\n')) >= 0) { const l = buf.slice(0, i); buf = buf.slice(i + 1); lineCb(l); }
});
const board = {
  onLine: (cb) => { lineCb = cb; },
  write: (line) => new Promise((res) => child.stdin.write(line + '\n', res)),
};

// One guest that plays /choose default until its battle ends.
function opponent(fmt) {
  return new Promise((resolve) => {
    const ws = new WebSocket(wsUrl);
    const answered = new Set();
    ws.onopen = () => setTimeout(() => ws.send(`|/search ${fmt}`), 1500);
    ws.onmessage = (ev) => {
      let room = '';
      for (const l of String(ev.data).split('\n')) {
        if (l.startsWith('>')) { room = l.slice(1); continue; }
        if (l.startsWith('|request|') && room.startsWith('battle-')) {
          const j = l.slice(9);
          if (!j || j === 'null') continue;
          let r; try { r = JSON.parse(j); } catch { continue; }
          if (r.wait || answered.has(`${room}${r.rqid}`)) continue;
          answered.add(`${room}${r.rqid}`);
          ws.send(`${room}|/choose default`);
        }
        if (l.startsWith('|win|') || l === '|tie') { setTimeout(() => { ws.close(); resolve(); }, 500); }
      }
    };
  });
}

const relay = new Relay(board, { onLog: (e) => { if (e.dir === '<-board' && /^(info|result|hello)/.test(e.text)) console.log(`  board: ${e.text}`); } });
const timeout = (s, what) => new Promise((_, rej) => setTimeout(() => rej(new Error(`${what} timed out after ${s}s`)), s * 1000));

let failed = false;
try {
  await relay.connect(wsUrl);
  await relay.hello('relay_test.mjs node');
  for (const gen of gens) {
    console.log(`Gen ${gen}: searching`);
    const t0 = Date.now();
    const played = relay.play(gen, true);
    if (useOpponent) opponent(`gen${gen}randombattle`);
    // The board says `info battle` when matched; searching has its own limit.
    const started = new Promise((res) => {
      const iv = setInterval(() => { if (relay.inBattle) { clearInterval(iv); res(); } }, 200);
    });
    await Promise.race([started, timeout(searchTimeoutS, `Gen ${gen} search`)]);
    const r = await Promise.race([played, timeout(battleTimeoutS, `Gen ${gen} battle`)]);
    console.log(`Gen ${gen}: done in ${((Date.now() - t0) / 1000).toFixed(1)} s: ${JSON.stringify(r)}`);
  }
} catch (e) {
  console.error(`FAIL: ${e.message}`);
  failed = true;
}

const done = relay.results.filter((r) => ['win', 'loss', 'tie'].includes(r.outcome)).length;
const rejected = relay.results.reduce((n, r) => n + Number(r.rejected || 0), 0);
const undecoded = relay.results.reduce((n, r) => n + Number(r.undecoded || 0), 0);
console.log(`battles finished: ${done} of ${gens.length}; rejected choices: ${rejected}; undecoded lines: ${undecoded}; disconnects: ${relay.disconnects}`);
if (logFile) writeFileSync(logFile, relay.logText({ test: 'relay_test.mjs', server: wsUrl }));
relay.close();
child.kill();
process.exit(failed || done !== gens.length || rejected || undecoded || relay.disconnects ? 1 : 0);
