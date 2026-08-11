// Single-screen device client. The browser is raw IO only: it paints the
// framebuffer core hands it and forwards button presses. Every decision about
// what appears on screen, and what a press means, lives in the Rust core.

import init, * as wasm from './pkg/mega_blastoise_web.js';

const canvas = document.getElementById('panel');
const ctx = canvas.getContext('2d');
// Core hands us a 240x320 image. Painting it straight into a canvas that CSS
// then rotates and fractionally scales makes the browser resample it, which
// is what turns crisp 1px strokes into grey mush. Instead we blow the frame
// up by a whole-number factor with smoothing off, so CSS only ever has to
// shrink it slightly — downscaling a sharp image looks far better than
// upscaling a small one.
const UPSCALE = 3;
const raw = document.createElement('canvas');
raw.width = 240;
raw.height = 320;
const rawCtx = raw.getContext('2d');
const PANEL_W = 240;
const PANEL_H = 320;

// Hold thresholds mirror core: HOLD_THRESHOLD_MS and AI_HOLD_MS.
const HOLD_MS = 500;
const AI_HOLD_MS = 2000;

let orientation = 0;
// Auto mode follows the spec: landscape for the lobby and menus, split
// head-to-head once a battle starts. The debug buttons pin it manually.
let autoOrient = true;
// Purely cosmetic rotation of the console on the page, in 90-degree steps.
// Independent of the game's orientation: it turns the whole device the way
// you would turn the real one on a table, without changing what it draws.
let viewTurns = 0;

// ── Painting ──────────────────────────────────────────────────────────────

function fitCanvas() {
  // The device is laid out at a fixed natural size and then scaled as one
  // piece to fill the viewport. Scaling the whole body keeps the panel and
  // the controls in exact proportion, and fills the screen instead of
  // stepping down to the nearest whole pixel multiple and leaving bars.
  //
  // In landscape the whole console turns, exactly like picking the real
  // device up and rotating it: the controls stay bolted to the case, and
  // core keeps drawing the same pixels the hardware panel would show.
  const device = document.getElementById('device');
  const debug = document.getElementById('debug');
  const debugH = debug ? debug.getBoundingClientRect().height : 30;

  // Backing store is a whole multiple of the panel grid.
  if (canvas.width !== PANEL_W * UPSCALE) {
    canvas.width = PANEL_W * UPSCALE;
    canvas.height = PANEL_H * UPSCALE;
  }
  device.style.transform = 'none';
  const rect = device.getBoundingClientRect();
  const natW = rect.width;
  const natH = rect.height;
  if (!natW || !natH) return;

  // The game's landscape mode counts as one turn, plus any the viewer added.
  const quarter = ((orientation === 2 ? 1 : 0) + viewTurns) % 4;
  const turned = quarter % 2 === 1;
  const availW = window.innerWidth - 16;
  const availH = window.innerHeight - debugH - 16;

  // Turned, the case occupies its own height across and its width down.
  const footW = turned ? natH : natW;
  const footH = turned ? natW : natH;
  const k = Math.min(availW / footW, availH / footH);

  device.style.transformOrigin = 'center center';
  device.style.transform = `rotate(${-90 * quarter}deg) scale(${k})`;
  // A transformed element still reserves its untransformed box, so pull the
  // layout in by the difference to keep it centred without overflow.
  device.style.margin = `${(footH * k - natH) / 2}px ${(footW * k - natW) / 2}px`;
}

function applyOrientation(mode) {
  orientation = mode;
  wasm.set_orientation(mode);
  fitCanvas();
  // Nothing to do to the controls: they are anchored to the device body and
  // never move, in any orientation. Only what the panel draws changes.
}

function paint(px) {
  const img = rawCtx.createImageData(PANEL_W, PANEL_H);
  img.data.set(px);
  rawCtx.putImageData(img, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(raw, 0, 0, canvas.width, canvas.height);
}

function frame() {
  if (autoOrient) {
    // Head-to-head is the default and covers the whole battle: turn playback
    // is one shared field across both halves, and choosing is the same halves
    // showing private menus. Landscape is only for the one-person menus.
    const menus = wasm.menu_active() && wasm.is_lobby_mode();
    const want = menus ? 2 : 0;
    if (want !== orientation) applyOrientation(want);
  }
  paint(wasm.get_device_pixels());
  requestAnimationFrame(frame);
}

// ── Buttons ───────────────────────────────────────────────────────────────

// A press classifies into tap / hold / AI-hold, the same three outcomes the
// firmware's matrix scan produces.
function wireHoldable(el, onTap, onHold, onAiHold) {
  let timer = null;
  let aiTimer = null;
  let fired = false;

  const clear = () => {
    clearTimeout(timer);
    clearTimeout(aiTimer);
    timer = aiTimer = null;
  };

  el.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    clear();
    fired = false;
    timer = setTimeout(() => {
      if (wasm.is_lobby_mode()) {
        aiTimer = setTimeout(() => {
          fired = true;
          if (onAiHold) onAiHold();
        }, AI_HOLD_MS - HOLD_MS);
      } else if (onHold) {
        fired = true;
        onHold();
      }
    }, HOLD_MS);
  });

  const finish = (e) => {
    if (e) e.preventDefault();
    clear();
    if (!fired) onTap();
    fired = false;
  };
  el.addEventListener('pointerup', finish);
  el.addEventListener('pointercancel', () => { clear(); fired = false; });
  el.addEventListener('pointerleave', () => { clear(); fired = false; });
}

function wireSeat(player) {
  document.querySelectorAll(`.dpad[data-player="${player}"] .d`).forEach((el) => {
    const dir = Number(el.dataset.dir);
    // Held direction auto-repeats, like the firmware's D-pad will.
    let repeat = null;
    el.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      wasm.nav_dpad(player, dir);
      repeat = setInterval(() => wasm.nav_dpad(player, dir), 180);
    });
    const stop = () => { clearInterval(repeat); repeat = null; };
    el.addEventListener('pointerup', stop);
    el.addEventListener('pointercancel', stop);
    el.addEventListener('pointerleave', stop);
  });

  document.querySelectorAll(`.rb[data-player="${player}"]`).forEach((el) => {
    const kind = el.dataset.btn;
    if (kind === 'a') {
      wireHoldable(el, () => wasm.nav_a(player), null, () => wasm.nav_a_hold(player));
    } else if (kind === 'b') {
      wireHoldable(el, () => wasm.nav_b(player), null, null);
    } else {
      wireHoldable(el, () => wasm.nav_info(player), null, null);
    }
  });
}

// ── Direct tap on the panel ───────────────────────────────────────────────
//
// Tablet players will reach for the thing they want, so a tap on a move cell
// or party row points the cursor at it. Confirming is still an explicit A,
// which keeps a stray touch from committing a turn.

function panelTap(ev) {
  // Menus are one-person landscape screens: a tap picks a row and confirms.
  if (wasm.menu_active() && wasm.is_lobby_mode()) {
    wasm.nav_a(1);
    return;
  }
  // Lobby: tapping your own half readies you up.
  if (wasm.is_lobby_mode()) {
    const r0 = canvas.getBoundingClientRect();
    const half = (ev.clientY - r0.top) / r0.height < 0.5 ? 2 : 1;
    wasm.nav_a(half);
    return;
  }
  // Shared landscape view: a tap advances the narration.
  if (orientation === 2) {
    if (wasm.seat_is_waiting(1)) wasm.nav_cancel(1);
    else wasm.nav_a(1);
    return;
  }
  const r = canvas.getBoundingClientRect();
  let x = ((ev.clientX - r.left) / r.width) * PANEL_W;
  let y = ((ev.clientY - r.top) / r.height) * PANEL_H;
  if (x < 0 || y < 0 || x >= PANEL_W || y >= PANEL_H) return;

  // Which half, and where inside it. The far half is rotated 180 when the
  // device is in head-to-head mode.
  let player;
  let hx;
  let hy;
  if (y < PANEL_H / 2) {
    player = 2;
    hx = orientation === 0 ? PANEL_W - 1 - x : x;
    hy = orientation === 0 ? PANEL_H / 2 - 1 - y : y;
  } else {
    player = 1;
    hx = x;
    hy = y - PANEL_H / 2;
  }

  // After committing, a tap anywhere on your own half takes it back.
  if (wasm.seat_is_waiting(player)) {
    wasm.nav_cancel(player);
    return;
  }

  const mode = wasm.nav_mode(player);
  if (mode === 0) {
    // 2x2 move grid: x 76..236, y 46..126. A tap is a whole decision, so it
    // commits rather than only moving the cursor.
    if (hx < 76 || hy < 46 || hy > 126) return;
    const col = hx < 158 ? 0 : 1;
    const row = hy < 88 ? 0 : 1;
    wasm.nav_tap_commit(player, row * 2 + col);
  } else if (mode === 1) {
    // Party rows start at y 26, 21px pitch.
    if (hy < 26) return;
    const idx = Math.floor((hy - 26) / 21);
    if (idx >= 0 && idx < 6) wasm.nav_tap_commit(player, idx);
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); panelTap(e); });

// ── Keyboard ──────────────────────────────────────────────────────────────

const KEYS = {
  KeyW: [1, 'd', 0], KeyS: [1, 'd', 1], KeyA: [1, 'd', 2], KeyD: [1, 'd', 3],
  KeyZ: [1, 'a'], KeyX: [1, 'b'], KeyC: [1, 'q'],
  ArrowUp: [2, 'd', 0], ArrowDown: [2, 'd', 1], ArrowLeft: [2, 'd', 2], ArrowRight: [2, 'd', 3],
  Comma: [2, 'a'], Period: [2, 'b'], Slash: [2, 'q'],
};

window.addEventListener('keydown', (e) => {
  const k = KEYS[e.code];
  if (!k) return;
  e.preventDefault();
  const [player, kind, dir] = k;
  if (kind === 'd') wasm.nav_dpad(player, dir);
  else if (kind === 'a') wasm.nav_a(player);
  else if (kind === 'b') wasm.nav_b(player);
  else wasm.nav_info(player);
});

// ── Debug bar ─────────────────────────────────────────────────────────────

document.querySelectorAll('#debug button[data-orient]').forEach((el) => {
  el.addEventListener('click', () => {
    autoOrient = false;
    applyOrientation(Number(el.dataset.orient));
    document.querySelectorAll('#debug button[data-orient]')
      .forEach((b) => b.classList.toggle('on', b === el));
    document.getElementById('auto-btn').classList.remove('on');
  });
});

document.getElementById('auto-btn').addEventListener('click', (e) => {
  autoOrient = true;
  e.currentTarget.classList.add('on');
  document.querySelectorAll('#debug button[data-orient]')
    .forEach((b) => b.classList.remove('on'));
});

document.getElementById('turn-btn').addEventListener('click', () => {
  viewTurns = (viewTurns + 1) % 4;
  fitCanvas();
});

document.getElementById('ai-btn').addEventListener('click', () => wasm.wasm_enter_vs_ai_mode());
document.getElementById('demo-btn').addEventListener('click', () => wasm.wasm_enter_demo_mode());
document.getElementById('reset-btn').addEventListener('click', () => wasm.wasm_reset());

window.addEventListener('resize', fitCanvas);

// ── Boot ──────────────────────────────────────────────────────────────────

async function run() {
  await init();
  const params = new URLSearchParams(location.search);
  if (params.has('orient')) {
    autoOrient = false;
    const mode = Number(params.get('orient'));
    applyOrientation(mode);
    document.getElementById('auto-btn').classList.remove('on');
    document.querySelectorAll('#debug button[data-orient]').forEach((b) => {
      b.classList.toggle('on', Number(b.dataset.orient) === mode);
    });
  } else {
    applyOrientation(0);
  }
  if (params.has('demo')) setTimeout(() => wasm.wasm_enter_demo_mode(), 300);
  // Debug aid: open P1's battle log after a delay, so a headless capture can
  // reach a screen that normally needs a button press.
  if (params.has('ready')) {
    // Debug aid: A on the gen picker, then A on both seats to ready up.
    setTimeout(() => wasm.nav_a(1), 400);
    setTimeout(() => { wasm.nav_a(1); wasm.nav_a(2); }, 900);
  }
  if (params.has('log')) {
    setTimeout(() => wasm.nav_info(1), Number(params.get('log')) || 12000);
  }
  if (params.has('ai')) setTimeout(() => wasm.wasm_enter_vs_ai_mode(), 300);
  wireSeat(1);
  wireSeat(2);
  fitCanvas();
  // Sizing the controls changes the pad height, which changes the budget the
  // panel was fitted against — settle it with a second pass.
  requestAnimationFrame(() => fitCanvas());
  requestAnimationFrame(frame);
  setInterval(() => wasm.wasm_tick_bob(), 75);
}

run().catch((err) => {
  document.body.insertAdjacentHTML(
    'beforeend',
    `<pre style="color:#e05257;padding:12px">Failed to load: ${err}</pre>`,
  );
});
