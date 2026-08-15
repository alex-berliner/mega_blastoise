// Single-screen device client. The browser is raw IO only: it paints the
// framebuffer core hands it and forwards button presses. Every decision about
// what appears on screen, and what a press means, lives in the Rust core.

// The console is loaded before the wasm glue so its patch catches anything
// the module emits while loading — but dynamically, with a stub fallback: a
// static import would take the whole page down if this one debug file ever
// 404s, which is exactly what a broken Pages deploy did once.
let setCommandHandler = () => {};
let consoleOpen = () => false;
try {
  const dc = await import('./devconsole.js');
  setCommandHandler = dc.setCommandHandler;
  consoleOpen = dc.isOpen;
} catch (e) {
  console.warn('devconsole unavailable, running without it:', e);
}
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
// Laid-out size of the panel element, which the physical-size presets scale
// against. Mirrors #panel in device.css.
const PANEL_CSS_H = 640;

// Hold thresholds are core's numbers, read out after the wasm loads, so the
// page classifies presses exactly like the firmware's matrix scan.
let HOLD_MS = 500;
let AI_HOLD_MS = 2000;

// ── Candidate panels ──────────────────────────────────────────────────────
//
// Real parts you might put in the physical build, so the page can show the
// case at the size it would actually be. `res` is the panel's own pixel grid:
// core renders 240x320 and nothing else, so a panel with a different grid is
// shown at its true physical size with the 240x320 image stretched onto it —
// the case size is honest, the pixel density is not.
const PANEL_NATIVE = { w: 240, h: 320 };
// The panel the firmware is actually being built for, per
// architecture/09-single-screen.md: 2.8" IPS ST7789, 240x320, SPI. Its own
// entry (rather than reusing the generic 2.8" row) so that layout judgements
// are made against THE target — if the hardware spec ever changes, change
// this one line and the picker follows.
const FW_TARGET = { id: 'fw', diag: 2.8, res: [240, 320], part: 'ST7789 IPS', fw: true };

const PANELS = [
  FW_TARGET,
  { id: '1.69', diag: 1.69, res: [240, 280], part: 'ST7789' },
  { id: '2.0', diag: 2.0, res: [240, 320], part: 'ST7789' },
  { id: '2.4', diag: 2.4, res: [240, 320], part: 'ILI9341' },
  { id: '2.8', diag: 2.8, res: [240, 320], part: 'ST7789 / ILI9341' },
  { id: '3.2', diag: 3.2, res: [240, 320], part: 'ILI9341' },
  { id: '3.5', diag: 3.5, res: [320, 480], part: 'ILI9488' },
  { id: '4.0', diag: 4.0, res: [320, 480], part: 'ST7796' },
  { id: '5.0', diag: 5.0, res: [800, 480], part: 'RGB parallel' },
];

const MM_PER_IN = 25.4;
// CSS defines an inch as 96px. Screens lie about their real DPI, so this is
// nominal — good enough to compare candidates side by side, and close to life
// size on a typical desktop monitor.
const PX_PER_MM = 96 / MM_PER_IN;

/// Active-area size in mm from the diagonal and the panel's pixel aspect.
function panelSizeMm(p) {
  const [pw, ph] = p.res;
  const diagMm = p.diag * MM_PER_IN;
  const h = diagMm / Math.sqrt(1 + (pw / ph) ** 2);
  return { w: h * (pw / ph), h };
}

function panelLabel(p) {
  const mm = panelSizeMm(p);
  const ppi = Math.round(Math.hypot(p.res[0], p.res[1]) / p.diag);
  const native = p.res[0] === PANEL_NATIVE.w && p.res[1] === PANEL_NATIVE.h;
  const base = `${p.diag.toFixed(2).replace(/0$/, '')}" ${p.res[0]}x${p.res[1]} · `
    + `${Math.round(mm.w)}x${Math.round(mm.h)}mm · ${ppi}ppi${native ? '' : ' · stretched'}`;
  return p.fw ? `firmware — ${base}` : base;
}

// 'auto' keeps the original behaviour: scale the whole case to fill the
// window. Anything else pins the panel to that part's real size.
let panelChoice = localStorage.getItem('mb-screen') || 'auto';

// Purely cosmetic rotation of the console on the page, in 90-degree steps.
// The device itself has one arrangement and never turns: the two players sit
// across from each other. This only moves where *you* are standing.
let viewTurns = 0;

// ── Painting ──────────────────────────────────────────────────────────────

function fitCanvas() {
  // The device is laid out at a fixed natural size and then scaled as one
  // piece to fill the viewport. Scaling the whole body keeps the panel and
  // the controls in exact proportion, and fills the screen instead of
  // stepping down to the nearest whole pixel multiple and leaving bars.
  const device = document.getElementById('device');
  const debug = document.getElementById('debug');
  const debugH = debug ? debug.getBoundingClientRect().height : 30;
  // The console panel is fixed above the debug bar and can take up to 45vh.
  // Reserve its height so the case shrinks to what is left instead of hiding
  // behind it — a console you have to close to see the screen you are
  // debugging is no use.
  const consoleEl = document.getElementById('console');
  const consoleH = consoleEl && !consoleEl.hidden
    ? consoleEl.getBoundingClientRect().height
    : 0;
  // The body flex-centres the device, so the same reserve has to come off the
  // bottom padding too; taking it off only the scale would centre the smaller
  // case in the full viewport and put it back under the panel. Written only
  // when it changes: the console watches the debug bar and we watch the
  // console, so an unconditional write feeds that loop every pass.
  const pad = `${debugH + consoleH + 8}px`;
  if (document.body.style.paddingBottom !== pad) document.body.style.paddingBottom = pad;

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

  const quarter = viewTurns % 4;
  const turned = quarter % 2 === 1;
  const availW = window.innerWidth - 16;
  const availH = window.innerHeight - debugH - consoleH - 16;

  // Turned, the case occupies its own height across and its width down.
  const footW = turned ? natH : natW;
  const footH = turned ? natW : natH;
  const fit = Math.min(availW / footW, availH / footH);
  // A pinned panel scales the case so the SCREEN comes out life size; auto
  // fills the window as before. The panel element is laid out at a fixed CSS
  // size, so the case scale that makes it `target` tall is target/that size.
  // Pinned still yields to the viewport: a case running off the edge tells
  // you nothing about the part.
  const pinned = PANELS.find((p) => p.id === panelChoice);
  const k = pinned
    ? Math.min((panelSizeMm(pinned).h * PX_PER_MM) / PANEL_CSS_H, fit)
    : fit;

  device.style.transformOrigin = 'center center';
  device.style.transform = `rotate(${-90 * quarter}deg) scale(${k})`;
  // A transformed element still reserves its untransformed box, so pull the
  // layout in by the difference to keep it centred without overflow.
  const margin = `${(footH * k - natH) / 2}px ${(footW * k - natW) / 2}px`;
  if (device.style.margin !== margin) device.style.margin = margin;
}

function paint(px) {
  const img = rawCtx.createImageData(PANEL_W, PANEL_H);
  img.data.set(px);
  rawCtx.putImageData(img, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(raw, 0, 0, canvas.width, canvas.height);
}

function frame() {
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
// The page only scales the pointer to the 240x320 grid. Which half was hit,
// the far half's rotation, and what the pixel means are core's decisions
// (DeviceSession::panel_tap) — the hit boxes are derived from the same layout
// constants the renderer draws with, so they cannot go stale here.

function panelTap(ev) {
  const r = canvas.getBoundingClientRect();
  const x = Math.floor(((ev.clientX - r.left) / r.width) * PANEL_W);
  const y = Math.floor(((ev.clientY - r.top) / r.height) * PANEL_H);
  if (x < 0 || y < 0 || x >= PANEL_W || y >= PANEL_H) return;
  wasm.panel_tap?.(x, y);
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
  // An open console owns the keyboard: every key is text for the command
  // line, never a seat's button. Close it to play with the keys again.
  if (consoleOpen() || e.target instanceof HTMLInputElement) return;
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

// The screen picker: auto, then every candidate part.
{
  const sel = document.getElementById('size-sel');
  const opt = (value, label) => {
    const o = document.createElement('option');
    o.value = value;
    o.textContent = label;
    sel.appendChild(o);
  };
  opt('auto', 'auto (fit window)');
  PANELS.forEach((p) => opt(p.id, panelLabel(p)));
  const params = new URLSearchParams(location.search);
  if (params.has('screen')) panelChoice = params.get('screen');
  sel.value = panelChoice;
  // An unknown id (stale storage, typo in the URL) falls back to auto rather
  // than leaving the select showing nothing.
  if (!sel.value) {
    panelChoice = 'auto';
    sel.value = 'auto';
  }
  sel.addEventListener('change', () => {
    panelChoice = sel.value;
    localStorage.setItem('mb-screen', panelChoice);
    const p = PANELS.find((x) => x.id === panelChoice);
    if (p) {
      const mm = panelSizeMm(p);
      const note = p.fw
        ? 'the firmware target (architecture/09-single-screen.md) — what you see is the panel, true size'
        : p.res[0] === PANEL_NATIVE.w && p.res[1] === PANEL_NATIVE.h
          ? 'native grid'
          : `NOT the render grid — 240x320 stretched onto ${p.res[0]}x${p.res[1]}`;
      console.log(
        `[screen] ${p.diag}" ${p.part}: ${mm.w.toFixed(1)}x${mm.h.toFixed(1)}mm, ${note}`,
      );
    }
    fitCanvas();
  });
}

document.getElementById('turn-btn').addEventListener('click', () => {
  viewTurns = (viewTurns + 1) % 4;
  fitCanvas();
});

document.getElementById('ai-btn').addEventListener('click', () => wasm.wasm_enter_vs_ai_mode());
document.getElementById('demo-btn').addEventListener('click', () => wasm.wasm_enter_demo_mode());
document.getElementById('reset-btn').addEventListener('click', () => wasm.wasm_reset());

window.addEventListener('resize', fitCanvas);

// Raising, closing or growing the console changes the height the case has to
// fit in. Watching the panel itself covers every way it opens — the debug-bar
// button, the backtick shortcut, `?console` — without the console module
// having to call back.
{
  const consoleEl = document.getElementById('console');
  let queued = false;
  // Deferred out of the callback rather than run inside it: refitting writes
  // layout, and doing that during delivery is what makes the browser report
  // "ResizeObserver loop completed with undelivered notifications" — an error
  // line in the very panel being opened. A timeout, not rAF: a page whose
  // frames are throttled (background tab, headless capture) would leave the
  // refit pending forever.
  const refit = () => {
    if (queued) return;
    queued = true;
    setTimeout(() => { queued = false; fitCanvas(); }, 0);
  };
  if (consoleEl) {
    // The panel opens and closes by toggling `hidden`, and a ResizeObserver
    // does NOT fire for display:none flips — verified in Chrome, it reports
    // 0 once at registration and then stays silent. So the attribute is what
    // is watched for open/close; the ResizeObserver only catches the panel
    // growing or the debug bar rewrapping underneath it while it is up.
    new MutationObserver(refit).observe(consoleEl, {
      attributes: true,
      attributeFilter: ['hidden', 'style'],
    });
    new ResizeObserver(refit).observe(consoleEl);
  }
}

// ── Boot ──────────────────────────────────────────────────────────────────

async function run() {
  await init();
  // Optional-chained: a stale cached wasm bundle missing newer exports must
  // degrade to defaults, not throw and black-screen the boot.
  HOLD_MS = wasm.hold_threshold_ms?.() ?? HOLD_MS;
  AI_HOLD_MS = wasm.ai_hold_ms?.() ?? AI_HOLD_MS;
  // The console's command line drives the same entry point the two-OLED page
  // types into, which is the firmware's USB grammar.
  setCommandHandler((line) => wasm.submit_text(line));
  const params = new URLSearchParams(location.search);
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
  // Debug aid: ?cmd=:ready ai runs console commands at boot, so a headless
  // capture can reach a state that otherwise needs typing. Repeatable.
  params.getAll('cmd').forEach((line, i) => {
    setTimeout(() => wasm.submit_text(line), 500 + i * 200);
  });
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
  console.error('boot failed:', err);
  document.body.insertAdjacentHTML(
    'beforeend',
    `<pre style="color:#e05257;padding:12px">Failed to load: ${err}</pre>`,
  );
});
