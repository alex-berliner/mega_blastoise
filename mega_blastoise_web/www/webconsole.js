// In-page console. The device UI is meant to be driven on a tablet or phone,
// where there is no devtools drawer to open, so anything console.log or a Rust
// panic writes has nowhere to land. This mirrors the console into a panel the
// debug bar can raise.
//
// Import this FIRST, before the wasm module: ES modules evaluate in import
// order, so patching here catches messages the wasm glue emits while loading.

const MAX_LINES = 500;

const lines = [];
let unseen = 0;
let open = false;

// ── Panel ─────────────────────────────────────────────────────────────────

const panel = document.createElement('div');
panel.id = 'console';
panel.hidden = true;
panel.innerHTML = `
  <div class="chead">
    <span class="lbl">CONSOLE</span>
    <span class="cfilter">
      <button data-level="all" class="on">all</button>
      <button data-level="warn">warn+</button>
      <button data-level="error">error</button>
    </span>
    <button class="ccopy">copy</button>
    <button class="cclear">clear</button>
    <button class="cclose">close</button>
  </div>
  <div class="clog"></div>
  <div class="cinput">
    <span class="cprompt">&gt;</span>
    <input type="text" autocomplete="off" autocapitalize="off" autocorrect="off"
           spellcheck="false" placeholder="command — ? for help">
  </div>`;
document.body.appendChild(panel);

const logEl = panel.querySelector('.clog');
let filter = 'all';

// The debug bar wraps to two rows on a narrow screen, so its height is not a
// constant to hardcode against — track it and sit exactly on top of it.
const debugBar = document.getElementById('debug');
if (debugBar) {
  const sit = () => {
    panel.style.bottom = `${debugBar.getBoundingClientRect().height}px`;
  };
  sit();
  new ResizeObserver(sit).observe(debugBar);
}

const RANK = { debug: 0, log: 1, info: 1, warn: 2, error: 3 };

function visible(level) {
  // Your own commands always show: filtering them out loses the context for
  // whatever the reply was.
  if (filter === 'all' || level === 'cmd') return true;
  return RANK[level] >= RANK[filter];
}

function render() {
  logEl.textContent = '';
  for (const line of lines) if (visible(line.level)) logEl.appendChild(node(line));
  logEl.scrollTop = logEl.scrollHeight;
}

function node(line) {
  const row = document.createElement('div');
  row.className = `crow ${line.level}`;
  row.textContent = `${line.t} ${line.text}`;
  return row;
}

function fmt(arg) {
  if (typeof arg === 'string') return arg;
  if (arg instanceof Error) return `${arg.message}\n${arg.stack || ''}`;
  try {
    return JSON.stringify(arg);
  } catch {
    return String(arg);
  }
}

const start = performance.now();

function push(level, args) {
  const t = ((performance.now() - start) / 1000).toFixed(2).padStart(7);
  const line = { level, t, text: args.map(fmt).join(' ') };
  lines.push(line);
  if (lines.length > MAX_LINES) lines.shift();

  if (open) {
    // Only keep pinning to the bottom if the reader was already there, so
    // scrolling back through history is not yanked away by new output.
    const atBottom = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 24;
    if (visible(level)) {
      logEl.appendChild(node(line));
      while (logEl.childElementCount > MAX_LINES) logEl.firstChild.remove();
      if (atBottom) logEl.scrollTop = logEl.scrollHeight;
    }
  } else {
    unseen += 1;
    updateBadge(level);
  }
}

// ── Capture ───────────────────────────────────────────────────────────────

for (const level of ['debug', 'log', 'info', 'warn', 'error']) {
  const original = console[level].bind(console);
  console[level] = (...args) => {
    push(level, args);
    original(...args);
  };
}

window.addEventListener('error', (e) => {
  push('error', [e.message, `${e.filename}:${e.lineno}:${e.colno}`]);
});
window.addEventListener('unhandledrejection', (e) => {
  push('error', ['unhandled rejection:', e.reason]);
});

// ── Debug-bar button ──────────────────────────────────────────────────────

const btn = document.getElementById('console-btn');
let worst = null;

function updateBadge(level) {
  if (!btn) return;
  if (level === 'error' || (level === 'warn' && worst !== 'error')) worst = level;
  btn.textContent = `console (${unseen})`;
  btn.classList.toggle('has-error', worst === 'error');
  btn.classList.toggle('has-warn', worst === 'warn');
}

export function isOpen() {
  return open;
}

export function setOpen(next) {
  open = next;
  panel.hidden = !open;
  if (btn) btn.classList.toggle('on', open);
  if (open) {
    unseen = 0;
    worst = null;
    if (btn) {
      btn.textContent = 'console';
      btn.classList.remove('has-error', 'has-warn');
    }
    render();
    // An open console owns the keyboard, so put the caret where typing goes.
    // Without this the first thing you type drives the game instead.
    const input = panel.querySelector('.cinput input');
    if (input) input.focus();
  }
}

if (btn) btn.addEventListener('click', () => setOpen(!open));
panel.querySelector('.cclose').addEventListener('click', () => setOpen(false));
panel.querySelector('.cclear').addEventListener('click', () => {
  lines.length = 0;
  render();
});
panel.querySelector('.ccopy').addEventListener('click', async () => {
  const text = lines.filter((l) => visible(l.level)).map((l) => `${l.t} ${l.text}`).join('\n');
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // Clipboard is blocked outside a secure context, which is exactly the
    // LAN-over-http case this panel exists for. Fall back to a selection.
    const sel = window.getSelection();
    sel.removeAllRanges();
    const range = document.createRange();
    range.selectNodeContents(logEl);
    sel.addRange(range);
  }
});

panel.querySelectorAll('.cfilter button').forEach((el) => {
  el.addEventListener('click', () => {
    filter = el.dataset.level;
    panel.querySelectorAll('.cfilter button').forEach((b) => b.classList.toggle('on', b === el));
    render();
  });
});

// ── Command line ──────────────────────────────────────────────────────────
//
// The web build already takes typed commands through wasm's submit_text — the
// same grammar the firmware's USB console uses, so `p1 2` or `:ready ai` mean
// the same thing on both. This is only the terminal for it; device.js hands
// over the handler once wasm is up, which keeps this module free of any
// import that would have to load before the console patch is installed.

const inputEl = panel.querySelector('.cinput input');
const history = [];
let histPos = 0;
let handler = null;

export function setCommandHandler(fn) {
  handler = fn;
}

// Clicking anywhere in the panel that is not a control puts the caret back in
// the command line, the same habit the two-OLED page has.
panel.addEventListener('pointerdown', (e) => {
  if (e.target.closest('button') || e.target === inputEl) return;
  if (window.getSelection()?.toString()) return;
  setTimeout(() => inputEl.focus(), 0);
});

inputEl.addEventListener('keydown', (e) => {
  // The game's key bindings live on window; anything typed here is text, not
  // a button press.
  e.stopPropagation();

  if (e.key === 'Enter') {
    const line = inputEl.value.trim();
    inputEl.value = '';
    if (!line) return;
    if (history[history.length - 1] !== line) history.push(line);
    histPos = history.length;
    push('cmd', [`> ${line}`]);
    if (handler) handler(line);
    else push('warn', ['no command handler yet — wasm still loading']);
    return;
  }

  if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    e.preventDefault();
    if (!history.length) return;
    histPos += e.key === 'ArrowUp' ? -1 : 1;
    histPos = Math.max(0, Math.min(history.length, histPos));
    inputEl.value = histPos === history.length ? '' : history[histPos];
    return;
  }

  if (e.key === 'Escape') {
    inputEl.blur();
    setOpen(false);
  }
});

// Backtick toggles it from a keyboard, matching the habit from game consoles.
// While typing it is just a character.
window.addEventListener('keydown', (e) => {
  if (e.code === 'Backquote' && document.activeElement !== inputEl) {
    e.preventDefault();
    setOpen(!open);
  }
});

push('info', [`console ready — ${location.pathname}${location.search} — ${screen.width}x${screen.height}`]);

// ?console raises it from boot, which is how a headless capture reaches it.
if (new URLSearchParams(location.search).has('console')) setOpen(true);

export function log(...args) {
  push('log', args);
}
