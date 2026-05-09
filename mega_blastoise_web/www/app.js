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

// ── Active-player button highlight ───────────────────────────────────────────

let lastActivePlayer = 0;

function updateActiveHighlight(active) {
    if (active === lastActivePlayer) return;
    lastActivePlayer = active;
    document.getElementById('panel-p1').classList.toggle('active', active === 1);
    document.getElementById('panel-p2').classList.toggle('active', active === 2);
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
    updateActiveHighlight(wasm.get_active_player());
    applyFlash(wasm.get_flash_state());
    requestAnimationFrame(frame);
}

// ── Button handlers (global so inline onclick works) ─────────────────────────

window.pressMove   = (player, slot) => wasm.press_move(player, slot);
window.pressSwitch = (player, idx)  => wasm.press_switch(player, idx);

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
    if (!e.target.closest('.btn')) inputEl.focus();
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
        requestAnimationFrame(frame);
    } catch (err) {
        document.getElementById('log').textContent += `\nFailed to load WASM: ${err}\n`;
        console.error(err);
    }
}

run();
