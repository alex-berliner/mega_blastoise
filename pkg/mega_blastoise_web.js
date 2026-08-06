/* @ts-self-types="./mega_blastoise_web.d.ts" */

/**
 * RGBA8888 for the whole 240x320 panel, composed for the current
 * orientation. Rendered on demand rather than cached: one full frame is
 * ~1 ms and it keeps the browser from ever showing stale halves.
 * @returns {Uint8Array}
 */
export function get_device_pixels() {
    const ret = wasm.get_device_pixels();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * @returns {Uint8Array}
 */
export function get_flash_state() {
    const ret = wasm.get_flash_state();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * @returns {Uint32Array}
 */
export function get_led_state() {
    const ret = wasm.get_led_state();
    var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * @returns {string}
 */
export function get_orientation() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_orientation();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @returns {Uint8Array}
 */
export function get_p1_pixels() {
    const ret = wasm.get_p1_pixels();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * @returns {Uint8Array}
 */
export function get_p2_pixels() {
    const ret = wasm.get_p2_pixels();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * The held button was released. Swallowed while a sticky latch is active —
 * on the web, letting go of the mouse doesn't end a hold.
 * @param {number} player
 */
export function hold_end(player) {
    wasm.hold_end(player);
}

/**
 * A move button crossed the 500 ms hold threshold (battle only — the lobby
 * long-press goes through wasm_lobby_long_press). On the web the hold
 * LATCHES: the pointer-up release is swallowed (see [`hold_end`]) and the
 * button stays held until clicked again or an option is committed.
 * @param {number} player
 * @param {number} slot
 */
export function hold_move(player, slot) {
    wasm.hold_move(player, slot);
}

/**
 * A party button crossed the 500 ms hold threshold.
 * @param {number} player
 * @param {number} idx
 */
export function hold_switch(player, idx) {
    wasm.hold_switch(player, idx);
}

/**
 * @returns {boolean}
 */
export function is_lobby_mode() {
    const ret = wasm.is_lobby_mode();
    return ret !== 0;
}

/**
 * True when neither seat is choosing: turn playback, which is the shared
 * moment both players watch, so it wants the full-panel landscape view.
 * @returns {boolean}
 */
export function is_playback() {
    const ret = wasm.is_playback();
    return ret !== 0;
}

/**
 * True while a pre-lobby menu owns the screen and the input.
 * @returns {boolean}
 */
export function menu_active() {
    const ret = wasm.menu_active();
    return ret !== 0;
}

/**
 * 0 gen picker, 1 lobby, 2 options — for the page's orientation logic.
 * @returns {number}
 */
export function menu_screen() {
    const ret = wasm.menu_screen();
    return ret;
}

/**
 * A — confirm. In the lobby this is the ready-up press.
 * @param {number} player
 */
export function nav_a(player) {
    wasm.nav_a(player);
}

/**
 * Hold A in the lobby to get an AI opponent, same as the hardware.
 * @param {number} player
 */
export function nav_a_hold(player) {
    wasm.nav_a_hold(player);
}

/**
 * B — back out.
 * @param {number} player
 */
export function nav_b(player) {
    wasm.nav_b(player);
}

/**
 * Tapping the panel after committing cancels it.
 * @param {number} player
 */
export function nav_cancel(player) {
    wasm.nav_cancel(player);
}

/**
 * Cursor position for a seat, so the page can highlight its on-screen pad.
 * @param {number} player
 * @returns {number}
 */
export function nav_cursor(player) {
    const ret = wasm.nav_cursor(player);
    return ret;
}

/**
 * D-pad: 0 up, 1 down, 2 left, 3 right.
 * @param {number} player
 * @param {number} dir
 */
export function nav_dpad(player, dir) {
    wasm.nav_dpad(player, dir);
}

/**
 * ? — explain whatever the cursor is on.
 * @param {number} player
 */
export function nav_info(player) {
    wasm.nav_info(player);
}

/**
 * Which list the cursor is in: 0 moves, 1 party, 2 detail.
 * @param {number} player
 * @returns {number}
 */
export function nav_mode(player) {
    const ret = wasm.nav_mode(player);
    return ret;
}

/**
 * Point the cursor straight at an item — what a direct tap on the screen
 * does. Bounded by the same limits the D-pad respects.
 * @param {number} player
 * @param {number} idx
 */
export function nav_set_cursor(player, idx) {
    wasm.nav_set_cursor(player, idx);
}

/**
 * Point at a slot and commit it in one action — what a direct tap on a move
 * cell does, so a tap is a whole turn decision rather than half of one.
 * @param {number} player
 * @param {number} idx
 */
export function nav_tap_commit(player, idx) {
    wasm.nav_tap_commit(player, idx);
}

/**
 * @param {number} player
 * @param {number} slot
 */
export function press_move(player, slot) {
    wasm.press_move(player, slot);
}

/**
 * @param {number} player
 * @param {number} idx
 */
export function press_switch(player, idx) {
    wasm.press_switch(player, idx);
}

/**
 * Has this seat committed and is now on the locked-in screen?
 * @param {number} player
 * @returns {boolean}
 */
export function seat_is_waiting(player) {
    const ret = wasm.seat_is_waiting(player);
    return ret !== 0;
}

/**
 * 0 = head-to-head (hardware), 1 = both halves upright, 2 = landscape.
 * @param {number} mode
 */
export function set_orientation(mode) {
    wasm.set_orientation(mode);
}

export function start() {
    wasm.start();
}

/**
 * @param {string} line
 */
export function submit_text(line) {
    const ptr0 = passStringToWasm0(line, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    wasm.submit_text(ptr0, len0);
}

export function wasm_enter_demo_mode() {
    wasm.wasm_enter_demo_mode();
}

export function wasm_enter_vs_ai_mode() {
    wasm.wasm_enter_vs_ai_mode();
}

/**
 * Latched buttons per player, for the page to render as "held" — bit 0-3 =
 * move buttons, bit 4-6 = party buttons.
 * @param {number} player
 * @returns {number}
 */
export function wasm_held_buttons(player) {
    const ret = wasm.wasm_held_buttons(player);
    return ret;
}

/**
 * Long-press lobby handler: `player` pressed long → their opponent becomes
 * AI-controlled (ready) and the presser proceeds to the controls picker.
 * @param {number} player
 */
export function wasm_lobby_long_press(player) {
    wasm.wasm_lobby_long_press(player);
}

export function wasm_reset() {
    wasm.wasm_reset();
}

/**
 * Advance the battle-screen sprite bobs — called every BOB_TICK_MS from JS,
 * mirroring the firmware's OLED-task tick. Each player's bob rate scales
 * with their active mon's Speed stat.
 */
export function wasm_tick_bob() {
    wasm.wasm_tick_bob();
}

/**
 * @returns {boolean}
 */
export function wasm_toggle_ai_pause() {
    const ret = wasm.wasm_toggle_ai_pause();
    return ret !== 0;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_debug_string_edece8177ad01481: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_5cd60d5cf78b4eef: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_undefined_35bb9f4c7fd651d5: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_9c31b086c2b26051: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_3fa391f3fcdb55f8: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_document_3540635616a18455: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_getElementById_78449141d07cd8ef: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_insertAdjacentText_859dd417dfaf0ece: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.insertAdjacentText(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_instanceof_Window_faa5cf994f49cca7: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_location_64bcc53b4356fa39: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_d8dfd33fa007511d: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_now_81363d44c96dd239: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_queueMicrotask_78d584b53af520f5: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_b39ea83c7f01971a: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_reload_55bba497dd160810: function() { return handleError(function (arg0) {
            arg0.reload();
        }, arguments); },
        __wbg_resolve_d17db9352f5a220e: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_scrollHeight_1987f4aa820bbd8d: function(arg0) {
            const ret = arg0.scrollHeight;
            return ret;
        },
        __wbg_setTimeout_4a8f96a1b4261aee: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_scrollTop_e4ea1f04309311f2: function(arg0, arg1) {
            arg0.scrollTop = arg1;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_02344c9b09eb08a9: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_ac6d4ac874d5cd54: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_9b2406c23aeb2023: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_b34d2126934e16ba: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_then_837494e384b37459: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_bd927500e8905df2: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 92, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___wasm_bindgen_1d8011bae8ecbaf0___JsValue__core_fccf67792830db87___result__Result_____wasm_bindgen_1d8011bae8ecbaf0___JsError___true_);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./mega_blastoise_web_bg.js": import0,
    };
}

function wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___wasm_bindgen_1d8011bae8ecbaf0___JsValue__core_fccf67792830db87___result__Result_____wasm_bindgen_1d8011bae8ecbaf0___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___wasm_bindgen_1d8011bae8ecbaf0___JsValue__core_fccf67792830db87___result__Result_____wasm_bindgen_1d8011bae8ecbaf0___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_1d8011bae8ecbaf0___convert__closures_____invoke___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined___js_sys_15fc16e421f21697___Function_fn_wasm_bindgen_1d8011bae8ecbaf0___JsValue_____wasm_bindgen_1d8011bae8ecbaf0___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('mega_blastoise_web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
