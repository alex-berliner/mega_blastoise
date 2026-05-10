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

// LED IDs in display order: P1 = 0-7 HP, 8-10 party, 11 status
//                           P2 = 12-23 (mirrored)
// We re-map to match firmware layout (P1 leds 0-11, P2 leds 12-23).
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

// ── RAF render loop ───────────────────────────────────────────────────────────

function frame() {
    renderOled(ctx1, wasm.get_p1_pixels());
    renderOled(ctx2, wasm.get_p2_pixels());
    renderLeds(wasm.get_led_state());
    applyFlash(wasm.get_flash_state());
    requestAnimationFrame(frame);
}

// ── Button handlers ────────────────────────────────────────────────────────────

// Switch buttons: long press shows party stats; short press selects.
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

// Long-press detection for move buttons (500 ms threshold).
// Short tap → press_move; hold → show move detail view until release.
// Long press is always available for both players regardless of whose turn it is.
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

// ── Text input ────────────────────────────────────────────────────────────────

const inputEl = document.getElementById('input');

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

// ── Boot ──────────────────────────────────────────────────────────────────────

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
        requestAnimationFrame(frame);
    } catch (err) {
        document.getElementById('log').textContent += `\nFailed to load WASM: ${err}\n`;
        console.error(err);
    }
}

run();
