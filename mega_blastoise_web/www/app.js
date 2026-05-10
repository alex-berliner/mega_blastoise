import init, * as wasm from './pkg/mega_blastoise_web.js';

// ── Canvas contexts ───────────────────────────────────────────────────────────

const oled1 = document.getElementById('oled-p1');
const oled2 = document.getElementById('oled-p2');
const ctx1  = oled1.getContext('2d');
const ctx2  = oled2.getContext('2d');

function renderOled(ctx, pixels) {
    const img = ctx.createImageData(128, 64);
    img.data.set(pixels);
    ctx.putImageData(img, 0, 0);
}

// ── LED strip rendering ───────────────────────────────────────────────────────

const ledEls = Array.from({ length: 24 }, (_, i) => document.getElementById(`led-${i}`));

function renderLeds(leds) {
    for (let i = 0; i < 24; i++) {
        const rgb = leds[i];
        const r = (rgb >> 16) & 0xff;
        const g = (rgb >> 8)  & 0xff;
        const b =  rgb        & 0xff;
        const el = ledEls[i];
        if (!el) continue;
        if (r === 0 && g === 0 && b === 0) {
            el.style.background = '#111';
            el.style.boxShadow  = 'none';
        } else {
            const col = `rgb(${r},${g},${b})`;
            el.style.background = col;
            el.style.boxShadow  = `0 0 7px ${col}`;
        }
    }
}

// ── Flash effects (super-effective / crit) ────────────────────────────────────

function applyFlash(flashState) {
    for (let p = 1; p <= 2; p++) {
        const type = flashState[p - 1];
        if (type === 0) continue;
        const el = document.getElementById(`oled-p${p}`);
        el.classList.remove('flash-super', 'flash-crit');
        void el.offsetWidth; // restart animation
        el.classList.add(type === 1 ? 'flash-super' : 'flash-crit');
    }
}

// ── Move button labels ────────────────────────────────────────────────────────

// Key hints shown on buttons (move slots 0-3 map to keys 1-4)
const MOVE_KEYS = ['1', '2', '3', '4'];
const SWITCH_KEYS = ['A', 'S', 'D'];

let p1MovesVer = -1, p2MovesVer = -1;
let p1PartyVer = -1, p2PartyVer = -1;

// Gen 1 type colors (dark theme friendly)
const TYPE_COLORS = {
    'Normal':   '#888',   'Fire':     '#c84',   'Water':    '#48c',
    'Electric': '#ca0',   'Grass':    '#4a8',   'Ice':      '#8cc',
    'Fighting': '#c44',   'Poison':   '#a4a',   'Ground':   '#a84',
    'Flying':   '#88c',   'Psychic':  '#c48',   'Bug':      '#8a4',
    'Rock':     '#888',   'Ghost':    '#448',   'Dragon':   '#44c',
};

function updateMoveLabels(player) {
    const raw = wasm.get_move_labels(player);
    const slots = raw.split(',');
    for (let i = 0; i < 4; i++) {
        const el = document.getElementById(`p${player}-m${i}`);
        if (!el) continue;
        const slot = slots[i] || '';
        if (!slot) {
            el.innerHTML = `<span class="mv-key">${MOVE_KEYS[i]}</span>`;
            el.classList.remove('mv-no-pp');
            el.style.borderColor = '';
            continue;
        }
        const [name, ppStr, typeName] = slot.split('|');
        const [pp, maxPp] = ppStr ? ppStr.split('/') : ['', ''];
        const noPp = pp === '0';
        const typeColor = TYPE_COLORS[typeName] || '#333';
        el.innerHTML = `<span class="mv-key">${MOVE_KEYS[i]}</span><span class="mv-name">${name}</span><span class="mv-pp">${pp}/${maxPp}</span>`;
        el.classList.toggle('mv-no-pp', noPp);
        el.style.borderColor = noPp ? '#1a2a1a' : typeColor;
    }
}

function hpColor(pct) {
    if (pct > 50) return '#4a8';
    if (pct > 25) return '#a84';
    return '#c44';
}

function updatePartyLabels(player) {
    const raw = wasm.get_party_labels(player);
    const slots = raw.split(',');
    const keys = player === 1 ? ['A', 'S', 'D'] : ['U', 'I', 'O'];
    for (let i = 0; i < 3; i++) {
        const el = document.getElementById(`p${player}-s${i}`);
        if (!el) continue;
        const slot = slots[i] || '';
        if (!slot) {
            el.innerHTML = `<span class="sw-key">${keys[i]}</span><span class="sw-label">S${i + 1}</span>`;
            el.classList.remove('sw-active', 'sw-fainted');
            el.style.borderColor = '';
            continue;
        }
        const [name, hpStr, activeStr] = slot.split('|');
        const [hp, maxHp] = hpStr ? hpStr.split('/').map(Number) : [0, 0];
        const active = activeStr === '1';
        const fainted = hp === 0 && !active;
        const pct = maxHp > 0 ? Math.round(hp * 100 / maxHp) : 0;
        // Abbreviate name to 5 chars max
        const abbr = name.length > 5 ? name.slice(0, 5) : name;
        const hpLabel = active ? '●' : (fainted ? '✕' : `${pct}%`);
        const hpCol = fainted ? '#333' : hpColor(pct);
        el.innerHTML = `<span class="sw-key">${keys[i]}</span><span class="sw-label">${abbr}</span><span class="sw-hp" style="color:${hpCol}">${hpLabel}</span>`;
        el.classList.toggle('sw-active', active);
        el.classList.toggle('sw-fainted', fainted);
        el.style.borderColor = active ? '#224488' : (fainted ? '#222' : hpCol + '66');
    }
}

function tickMoveLabels() {
    const v1 = wasm.get_moves_version(1);
    const v2 = wasm.get_moves_version(2);
    if (v1 !== p1MovesVer) { p1MovesVer = v1; updateMoveLabels(1); }
    if (v2 !== p2MovesVer) { p2MovesVer = v2; updateMoveLabels(2); }
    const pv1 = wasm.get_party_version(1);
    const pv2 = wasm.get_party_version(2);
    if (pv1 !== p1PartyVer) { p1PartyVer = pv1; updatePartyLabels(1); }
    if (pv2 !== p2PartyVer) { p2PartyVer = pv2; updatePartyLabels(2); }
}

// ── RAF render loop ───────────────────────────────────────────────────────────

function frame() {
    renderOled(ctx1, wasm.get_p1_pixels());
    renderOled(ctx2, wasm.get_p2_pixels());
    renderLeds(wasm.get_led_state());
    applyFlash(wasm.get_flash_state());
    tickMoveLabels();
    requestAnimationFrame(frame);
}

// ── Button handlers ────────────────────────────────────────────────────────────

function setupSwitchLongPress(el, player, idx) {
    let timer = null;
    let fired = false;

    el.addEventListener('pointerdown', e => {
        e.preventDefault();
        fired = false;
        timer = setTimeout(() => {
            fired = true;
            wasm.wasm_show_pokemon_stats(player, idx);
        }, 500);
    });

    el.addEventListener('pointerup', () => {
        clearTimeout(timer);
        if (fired) {
            wasm.wasm_restore_screen(player);
        } else {
            wasm.press_switch(player, idx);
        }
        fired = false;
    });

    el.addEventListener('pointercancel', () => {
        clearTimeout(timer);
        if (fired) wasm.wasm_restore_screen(player);
        fired = false;
    });
}

function setupMoveLongPress(el, player, slot) {
    let timer = null;
    let fired = false;

    el.addEventListener('pointerdown', e => {
        e.preventDefault();
        fired = false;
        timer = setTimeout(() => {
            fired = true;
            wasm.wasm_show_move_detail(player, slot);
        }, 500);
    });

    el.addEventListener('pointerup', () => {
        clearTimeout(timer);
        if (fired) {
            wasm.wasm_restore_screen(player);
        } else {
            wasm.press_move(player, slot);
        }
        fired = false;
    });

    el.addEventListener('pointercancel', () => {
        clearTimeout(timer);
        if (fired) wasm.wasm_restore_screen(player);
        fired = false;
    });
}

// ── Keyboard shortcuts ────────────────────────────────────────────────────────

// P1: 1-4 = moves 0-3, a/s/d = switches 0-2
// P2: 7/8/9/0 = moves 0-3, u/i/o = switches 0-2 (2-player mode)
const P1_MOVE_KEYS   = { '1': 0, '2': 1, '3': 2, '4': 3 };
const P1_SWITCH_KEYS = { 'a': 0, 's': 1, 'd': 2 };
const P2_MOVE_KEYS   = { '7': 0, '8': 1, '9': 2, '0': 3 };
const P2_SWITCH_KEYS = { 'u': 0, 'i': 1, 'o': 2 };

document.addEventListener('keydown', e => {
    // Don't intercept when typing in the text input
    if (document.activeElement === inputEl) return;
    // Don't intercept modifier combos
    if (e.ctrlKey || e.metaKey || e.altKey) return;

    const k = e.key.toLowerCase();

    if (k in P1_MOVE_KEYS) {
        e.preventDefault();
        wasm.press_move(1, P1_MOVE_KEYS[k]);
        flashBtn(`p1-m${P1_MOVE_KEYS[k]}`);
    } else if (k in P1_SWITCH_KEYS) {
        e.preventDefault();
        wasm.press_switch(1, P1_SWITCH_KEYS[k]);
        flashBtn(`p1-s${P1_SWITCH_KEYS[k]}`);
    } else if (k in P2_MOVE_KEYS) {
        e.preventDefault();
        wasm.press_move(2, P2_MOVE_KEYS[k]);
        flashBtn(`p2-m${P2_MOVE_KEYS[k]}`);
    } else if (k in P2_SWITCH_KEYS) {
        e.preventDefault();
        wasm.press_switch(2, P2_SWITCH_KEYS[k]);
        flashBtn(`p2-s${P2_SWITCH_KEYS[k]}`);
    }
});

function flashBtn(id) {
    const el = document.getElementById(id);
    if (!el) return;
    el.classList.add('btn-flash');
    setTimeout(() => el.classList.remove('btn-flash'), 120);
}

// ── Text input ────────────────────────────────────────────────────────────────

const inputEl = document.getElementById('input');
const logEl   = document.getElementById('log');
const LOG_MAX = 400;

inputEl.addEventListener('keydown', e => {
    if (e.key === 'Enter') {
        const line = inputEl.value;
        inputEl.value = '';
        wasm.submit_text(line);
    }
});

document.addEventListener('click', e => {
    const ids = ['demo-btn', 'vs-ai-btn', 'pause-btn', 'reset-btn'];
    if (!e.target.closest('.btn') && !ids.includes(e.target.id)) inputEl.focus();
});

document.getElementById('vs-ai-btn').addEventListener('click', () => {
    wasm.wasm_enter_vs_ai_mode();
});

document.getElementById('demo-btn').addEventListener('click', () => {
    wasm.wasm_enter_demo_mode();
});

const pauseBtn = document.getElementById('pause-btn');
pauseBtn.addEventListener('click', () => {
    const paused = wasm.wasm_toggle_ai_pause();
    pauseBtn.textContent = paused ? 'RESUME' : 'PAUSE';
});

document.getElementById('reset-btn').addEventListener('click', () => {
    wasm.wasm_reset();
});
inputEl.focus();

// Scroll input into view when virtual keyboard appears on mobile
inputEl.addEventListener('focus', () => {
    setTimeout(() => inputEl.scrollIntoView({ behavior: 'smooth', block: 'end' }), 150);
});

// ── Log line trimmer ──────────────────────────────────────────────────────────
// Rust calls print_log which does insertAdjacentText; we observe mutations and trim.

// Trim log every 5 seconds to prevent unbounded growth.
setInterval(() => {
    const children = logEl.childNodes;
    if (children.length > LOG_MAX + 50) {
        const toRemove = children.length - LOG_MAX;
        for (let i = 0; i < toRemove; i++) logEl.removeChild(logEl.firstChild);
    }
}, 5000);

// ── Colored log (override Rust's print_log via global monkey-patch) ────────────
// We patch window.__print_log to colorize lines before appending.
// Rust print_log calls insertAdjacentText directly, so we intercept via a
// MutationObserver that replaces newly added text nodes with colored spans.

const TEXT_COLOR_RULES = [
    [/fainted!/,                   '#c44'],   // faint → red
    [/WINNER!/,                    '#ca0'],   // win → gold
    [/GG!/,                        '#888'],   // loss → grey
    [/TIE!/,                       '#888'],   // tie
    [/── Battle/,                  '#4a8'],   // turn separator → green
    [/─── Turn/,                   '#4a8'],   // turn marker
    [/was inflicted|is paralyzed|was burned|was frozen|fell asleep|was poisoned/, '#a4a'],  // status
    [/was cured|woke up|thawed|unfrozen/,  '#8cc'],  // cure status → cyan
    [/super effective/i,           '#ca0'],   // SE → gold
    [/critical hit/i,              '#c84'],   // crit → orange
    [/It missed|but it failed/i,   '#666'],   // miss/fail → grey
    [/took damage/,                '#c88'],   // damage → light red
    [/\[OK\]/,                     '#4a8'],   // accepted → green
    [/\[!!\]/,                     '#c44'],   // rejected → red
    [/\[EVT\]/,                    '#688'],   // event prefix
    [/Keys|build |MEGA BLASTOISE/, '#444'],   // meta lines
];

const logMutObs = new MutationObserver(mutations => {
    for (const m of mutations) {
        for (const node of m.addedNodes) {
            if (node.nodeType !== Node.TEXT_NODE) continue;
            const text = node.textContent;
            let color = null;
            for (const [pat, col] of TEXT_COLOR_RULES) {
                if (pat.test(text)) { color = col; break; }
            }
            if (color) {
                const span = document.createElement('span');
                span.style.color = color;
                span.textContent = text;
                node.replaceWith(span);
            }
        }
    }
});
logMutObs.observe(logEl, { childList: true });

// ── Boot ──────────────────────────────────────────────────────────────────────

// Set initial switch button key hints (overwritten once party data arrives)
function initSwitchHints() {
    for (let p = 1; p <= 2; p++) {
        const keys = p === 1 ? ['A', 'S', 'D'] : ['U', 'I', 'O'];
        for (let i = 0; i < 3; i++) {
            const el = document.getElementById(`p${p}-s${i}`);
            if (el) el.innerHTML = `<span class="sw-key">${keys[i]}</span><span class="sw-label">S${i + 1}</span>`;
        }
    }
}


async function run() {
    try {
        await init();
        // Wire up long-press detection for all move buttons.
        [[1,0],[1,1],[1,2],[1,3],[2,0],[2,1],[2,2],[2,3]].forEach(([p, s]) => {
            const el = document.getElementById(`p${p}-m${s}`);
            if (el) setupMoveLongPress(el, p, s);
        });
        // Wire up switch buttons (long press = party stats view).
        [[1,0],[1,1],[1,2],[2,0],[2,1],[2,2]].forEach(([p, i]) => {
            const el = document.getElementById(`p${p}-s${i}`);
            if (el) setupSwitchLongPress(el, p, i);
        });
        initSwitchHints();
        // Seed move buttons with key hints before first battle
        for (let p = 1; p <= 2; p++) {
            for (let i = 0; i < 4; i++) {
                const el = document.getElementById(`p${p}-m${i}`);
                if (el) el.innerHTML = `<span class="mv-key">${MOVE_KEYS[i]}</span>`;
            }
        }
        requestAnimationFrame(frame);
    } catch (err) {
        document.getElementById('log').textContent += `\nFailed to load WASM: ${err}\n`;
        console.error(err);
    }
}

run();
