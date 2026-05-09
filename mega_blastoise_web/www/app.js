import init, * as wasm from './pkg/mega_blastoise_web.js';

const inputEl = document.getElementById('input');
const outputEl = document.getElementById('output');

// Forward Enter key to Rust
inputEl.addEventListener('keydown', e => {
    if (e.key === 'Enter') {
        const line = inputEl.value;
        inputEl.value = '';
        wasm.submit_input(line);
    }
});

// Keep input focused on any click
document.addEventListener('click', () => inputEl.focus());
inputEl.focus();

async function run() {
    try {
        await init();
        // Game loop starts automatically via #[wasm_bindgen(start)]
    } catch (err) {
        outputEl.textContent += `\nFailed to load: ${err}\n`;
        console.error(err);
    }
}

run();
