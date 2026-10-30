// The relay between the board and Pokemon Showdown's websocket. The page and
// the Node test (tools/bridge/relay_test.mjs) run this same file, so what the
// test exercises is what the phone runs.
//
// It decides nothing about the battle. Lines go both ways:
//   Showdown -> board:  "frame", then "< <line>" for each line of the message
//   board -> Showdown:  "> <text>" is sent as the websocket text, verbatim
// Every other board line (hello, info, result) only goes in the log.
// The line protocol is documented in crates/mega_blastoise_core/src/online.rs.

export const RELAY_VERSION = 'bridge-1';

export class Relay {
  // board:  { write(line) -> Promise, onLine(cb) } (WebUSB on the phone, a child process in the test)
  // opts.makeSocket(url) -> a WebSocket-like object (the global WebSocket by default)
  // opts.now() -> epoch ms (Date.now by default)
  // opts.onLog(entry) -> called for each log entry (the page paints it)
  constructor(board, opts = {}) {
    this.board = board;
    this.makeSocket = opts.makeSocket || ((url) => new WebSocket(url));
    this.now = opts.now || (() => Date.now());
    this.onLog = opts.onLog || (() => {});
    this.entries = [];
    this.ws = null;
    this.wsOpen = false;
    this.inBattle = false;
    this.disconnects = 0;
    this.results = [];
    this.fw = null; // the firmware commit, from the board's hello
    this.waiters = [];
    this.queue = Promise.resolve();
    board.onLine((line) => this.fromBoard(line));
  }

  log(dir, text) {
    const e = { t: this.now(), dir, text };
    this.entries.push(e);
    this.onLog(e);
  }

  // The log as a file: a header (who, what, when), then one line per entry.
  logText(extra = {}) {
    const head = [`# mega_blastoise phone bridge log, relay ${RELAY_VERSION}`];
    for (const [k, v] of Object.entries(extra)) head.push(`# ${k}: ${v}`);
    head.push(`# firmware: ${this.fw || 'unknown (no hello from the board)'}`);
    head.push(`# saved: ${new Date(this.now()).toISOString()}`);
    head.push(`# disconnects: ${this.disconnects}`);
    const body = this.entries.map((e) => `${new Date(e.t).toISOString()} ${e.dir} ${e.text}`);
    return head.concat(body).join('\n') + '\n';
  }

  async toBoard(line) {
    this.log('->board', line);
    await this.board.write(line);
  }

  // Lines for the board go out strictly in order, one websocket message at a time.
  enqueue(fn) {
    this.queue = this.queue.then(fn).catch((e) => this.log('error', String(e && e.message || e)));
    return this.queue;
  }

  connect(url) {
    return new Promise((resolve, reject) => {
      this.log('ws', `connecting ${url}`);
      const ws = this.makeSocket(url);
      this.ws = ws;
      ws.onopen = () => { this.wsOpen = true; this.log('ws', 'open'); resolve(); };
      ws.onerror = () => { this.log('ws', 'error'); if (!this.wsOpen) reject(new Error('websocket failed to open')); };
      ws.onclose = (ev) => {
        const was = this.wsOpen;
        this.wsOpen = false;
        this.log('ws', `closed code=${ev && ev.code}`);
        if (was && this.inBattle) this.disconnects++;
        this.enqueue(() => this.toBoard('down'));
      };
      ws.onmessage = (ev) => {
        const data = typeof ev.data === 'string' ? ev.data : String(ev.data);
        this.log('<-ws', data.length > 4000 ? `${data.slice(0, 4000)}... (${data.length} chars)` : data);
        this.enqueue(async () => {
          await this.board.write('frame');
          for (const line of data.split('\n')) await this.board.write(`< ${line}`);
        });
      };
    });
  }

  fromBoard(line) {
    this.log('<-board', line);
    if (line.startsWith('> ')) {
      const text = line.slice(2);
      if (this.wsOpen) {
        this.ws.send(text);
        this.log('->ws', text);
      } else {
        this.log('error', `dropped, websocket not open: ${text}`);
      }
    } else if (line.startsWith('hello mb1')) {
      const m = line.match(/fw=(\S+)/);
      this.fw = m ? m[1] : 'unknown';
    } else if (line.startsWith('info battle ')) {
      this.inBattle = true;
    } else if (line.startsWith('result ')) {
      this.inBattle = false;
      const r = Object.fromEntries(line.slice(7).split(' ').map((kv) => kv.split('=')));
      this.results.push(r);
      for (const w of this.waiters.splice(0)) w(r);
    } else if (line === 'info search cancelled') {
      for (const w of this.waiters.splice(0)) w(null);
    }
  }

  hello(text) { return this.enqueue(() => this.toBoard(`hello ${text}`)); }

  // Asks for a battle; resolves with the board's `result` (null if cancelled).
  play(gen, auto) {
    const done = new Promise((resolve) => this.waiters.push(resolve));
    this.enqueue(() => this.toBoard(`play ${gen}${auto ? ' auto' : ''}`));
    return done;
  }

  cancel() { return this.enqueue(() => this.toBoard('cancel')); }

  close() { if (this.ws) this.ws.close(); }
}
