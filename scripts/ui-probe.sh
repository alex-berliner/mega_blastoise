#!/usr/bin/env bash
# Headless end-to-end probe of the web client's seat flow, both generations.
#
# This exists because a class of bug lives ABOVE every unit test we have: the
# collector can be perfect and the engines can be perfect while the page still
# wedges, because the wiring between them — prompts, nav state, screen
# restores — is only exercised by a real client driving a real battle. The
# reports that motivated it: Gen 1 stuck on the "Both ready!" screen after
# both players chose, and Gen 3 seats that were never prompted at all.
#
# It boots device.html in headless Chromium, plays the opening of a battle by
# calling the SAME wasm exports the touch surfaces call (nav_a / press_move /
# nav_cancel), and asserts on `screen_state()` — the per-seat state trace the
# page maintains for exactly this purpose. Run it after any change to the
# collector, the runners, or the web glue:
#
#   scripts/ui-probe.sh            # gen 1 (the default ruleset)
#   scripts/ui-probe.sh --skip-build   # reuse the existing pkg/
#
# Chromium: the snap at /usr/bin/chromium-browser cannot write outside $HOME;
# the playwright build can run anywhere. Override with $CHROME.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB="$ROOT/mega_blastoise_web"
CHROME="${CHROME:-$(ls -d "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux64/chrome 2>/dev/null | head -1)}"
[ -x "$CHROME" ] || { echo "no chromium found; set \$CHROME" >&2; exit 2; }

if [[ "${1:-}" != "--skip-build" ]]; then
  echo "building wasm (dev)..."
  (cd "$WEB" && wasm-pack build --target web --dev >/dev/null 2>&1)
fi
# www/pkg is a symlink to ../pkg, so the fresh build is already in place.
[ -e "$WEB/www/pkg/mega_blastoise_web.js" ] || { echo "www/pkg missing" >&2; exit 2; }

PROBE="$WEB/www/_probe.html"
cleanup() { rm -f "$PROBE"; kill "$SERVER_PID" 2>/dev/null || true; }
trap cleanup EXIT

cat > "$PROBE" <<'HTML'
<!doctype html><meta charset="utf-8"><pre id="out">RUNNING</pre>
<pre id="log" style="display:none"></pre>
<script type="module">
import init, * as wasm from './pkg/mega_blastoise_web.js';
const out = [];
const say = (s) => { out.push(s); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const states = () => [wasm.screen_state(1), wasm.screen_state(2)];

// Wait until a predicate over the two seat states holds, or fail loudly with
// the states seen — the trace IS the diagnosis when this trips.
let trace = [];
async function until(label, pred, ms = 8000) {
  const t0 = Date.now();
  let last = '';
  while (Date.now() - t0 < ms) {
    const s = states();
    const key = s.join(',');
    if (key !== last) { trace.push(key); last = key; }
    if (pred(s)) { say(`PASS ${label}  [${s}]`); return s; }
    await sleep(100);
  }
  const s = states();
  say(`FAIL ${label}  [${s}]`);
  say(`trace: ${trace.join(' > ')}`);
  const log = document.getElementById('log').textContent.split('\n');
  say(`log tail: ${log.slice(-14).join(' | ')}`);
  throw new Error(label);
}

try {
  await init();
  // Through the menus: the gen picker first (row 0 Gen 1, row 1 Gen 3 —
  // the lane says which), then both seats ready up with default controls.
  const lane = new URLSearchParams(location.search).get('lane') ?? 'gen1';
  if (lane === 'gen3') { wasm.nav_dpad(1, 1); await sleep(120); }
  wasm.nav_a(1); await sleep(300);
  wasm.nav_a(1); wasm.nav_a(2); await sleep(300);
  wasm.nav_a(1); wasm.nav_a(2);
  await until('battle opens on the move menu', (s) =>
    s.every((x) => /BATTLE|MOVE|CHOOSE/i.test(x)), 15000);

  // ── The "Both ready!" hang. Both seats commit; the turn MUST resolve and
  //    re-prompt: neither seat may still be locked-in a grace-window later.
  wasm.press_move(1, 0); wasm.press_move(2, 0);
  await until('both committed', (s) => s.some((x) => /LOCKED_IN/.test(x)), 4000)
    .catch(() => say('note: commits resolved before the poll saw LOCKED_IN'));
  await until('the turn resolves and re-prompts', (s) =>
    !s.some((x) => /LOCKED_IN|WAIT_OPPONENT/.test(x)), 30000);

  // ── B cancels a committed choice: seat 1 commits, cancels, and must be
  //    back on a choosing screen while seat 2 stays uncommitted.
  //    Wait out the narration first: a press during EVENT_TEXT is a dialog
  //    skip, and a press between turns is deliberately dropped — both are
  //    behaviours this probe protects, not obstacles to it.
  await until('back on the menus', (s) => s.every((x) => /MOVES/.test(x)), 40000);
  wasm.press_move(1, 0);
  await until('seat 1 locked in', (s) => /LOCKED_IN/.test(s[0]), 4000);
  wasm.nav_cancel(1);
  await until('B put seat 1 back on its menu', (s) => !/LOCKED_IN/.test(s[0]), 4000);

  // Finish the turn so the page ends in a live state.
  await until('menus again after the cancel', (s) => /MOVES/.test(s[0]), 20000);
  wasm.press_move(1, 0); wasm.press_move(2, 0);
  await until('second turn resolves too', (s) =>
    !s.some((x) => /LOCKED_IN|WAIT_OPPONENT/.test(x)), 30000);

  // ── A faint must ASK. Play the battle out with both seats mashing their
  //    first move until somebody faints; the seat that lost a mon has to be
  //    put on the party picker, and nothing may be sent out until it picks.
  //    The report this covers: a replacement appeared on the board, health
  //    bar full, without the player ever being offered the choice.
  let sawPicker = false;
  for (let i = 0; i < 400 && !sawPicker; i++) {
    const s = states();
    if (s.some((x) => /FORCED_SWITCH|CONCEALED_SWITCH/.test(x))) { sawPicker = true; break; }
    if (s.some((x) => /WIN/.test(x))) break;
    for (const seat of [1, 2]) {
      if (/MOVES/.test(s[seat - 1])) wasm.press_move(seat, 0);
    }
    await sleep(150);
  }
  if (sawPicker) {
    say('PASS a faint puts the player on the party picker');
    const s = states();
    const seat = /FORCED_SWITCH|CONCEALED_SWITCH/.test(s[0]) ? 1 : 2;
    wasm.press_switch(seat, 1);
    await until('the picked replacement takes the field', (t) =>
      !/FORCED_SWITCH|CONCEALED_SWITCH/.test(t[seat - 1]), 20000);
  } else {
    say('FAIL nobody was asked for a replacement before the battle ended');
    say(`trace: ${trace.join(' > ')}`);
    throw new Error('no replacement prompt');
  }

  say('ALL PASS');
} catch (e) {
  say(`ABORT ${e.message ?? e}`);
}
document.getElementById('out').textContent = out.join('\n');
</script>
HTML

cd "$WEB/www"
python3 -m http.server 8123 >/dev/null 2>&1 &
SERVER_PID=$!
sleep 1

FAILED=0
for LANE in gen1 gen3; do
  echo "── lane: $LANE ──"
  DOM=$("$CHROME" --headless=new --no-sandbox --disable-gpu \
    --virtual-time-budget=150000 --timeout=180000 \
    --dump-dom "http://127.0.0.1:8123/_probe.html?seed=7777&lane=$LANE" 2>/dev/null || true)
  RESULT=$(echo "$DOM" | python3 -c "
import re, sys
m = re.search(r'<pre id=\"out\">(.*?)</pre>', sys.stdin.read(), re.S)
print(m.group(1) if m else 'NO OUTPUT — page did not render')
")
  echo "$RESULT"
  echo "$RESULT" | grep -q "ALL PASS" || FAILED=1
done
if [ "$FAILED" = 0 ]; then
  echo "ui-probe: OK"
else
  echo "ui-probe: FAILED" >&2
  exit 1
fi
