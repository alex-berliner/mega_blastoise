// The board over WebUSB: the vendor interface the firmware adds
// (crates/mega_blastoise_fw/src/subsystems/usb_v2.rs), two bulk endpoints
// carrying newline-ended lines.

export const BOARD_FILTER = { vendorId: 0xc0de, productId: 0xcafe };

export class WebUsbBoard {
  constructor(log) {
    this.log = log || (() => {});
    this.device = null;
    this.cb = () => {};
    this.out = null;
    this.in = null;
    this.open_ = false;
  }

  onLine(cb) { this.cb = cb; }

  // Must run inside a tap: Chrome asks the player to allow this one device.
  async request() {
    this.device = await navigator.usb.requestDevice({ filters: [BOARD_FILTER] });
    return this.open();
  }

  async open() {
    const d = this.device;
    await d.open();
    if (d.configuration === null) await d.selectConfiguration(1);
    let found = null;
    for (const iface of d.configuration.interfaces) {
      for (const alt of iface.alternates) {
        const bulkIn = alt.endpoints.find((e) => e.type === 'bulk' && e.direction === 'in');
        const bulkOut = alt.endpoints.find((e) => e.type === 'bulk' && e.direction === 'out');
        if (alt.interfaceClass === 0xff && bulkIn && bulkOut) found = { iface, alt, bulkIn, bulkOut };
      }
    }
    if (!found) throw new Error('no vendor interface with bulk endpoints (is this the bridge firmware?)');
    await d.claimInterface(found.iface.interfaceNumber);
    if (found.alt.alternateSetting !== 0) await d.selectAlternateInterface(found.iface.interfaceNumber, found.alt.alternateSetting);
    this.out = found.bulkOut.endpointNumber;
    this.in = found.bulkIn.endpointNumber;
    this.packet = found.bulkIn.packetSize || 64;
    this.open_ = true;
    this.log(`board open: interface ${found.iface.interfaceNumber}, in ${this.in}, out ${this.out}, ${d.productName || ''} ${d.serialNumber || ''}`);
    this.readLoop();
    return d;
  }

  get connected() { return this.open_; }

  async readLoop() {
    const dec = new TextDecoder();
    let buf = '';
    while (this.open_) {
      let r;
      try {
        // One packet at a time: a transfer longer than the packet would wait for a short one.
        r = await this.device.transferIn(this.in, this.packet);
      } catch (e) {
        this.open_ = false;
        this.log(`board read failed: ${e.message || e}`);
        this.cb('usb-lost');
        return;
      }
      if (r.status === 'stall') { await this.device.clearHalt('in', this.in); continue; }
      if (!r.data || r.data.byteLength === 0) continue;
      buf += dec.decode(r.data, { stream: true });
      let i;
      while ((i = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, i).replace(/\r$/, '');
        buf = buf.slice(i + 1);
        this.cb(line);
      }
    }
  }

  async write(line) {
    if (!this.open_) throw new Error('board not open');
    const bytes = new TextEncoder().encode(line + '\n');
    const r = await this.device.transferOut(this.out, bytes);
    if (r.status !== 'ok') throw new Error(`board write ${r.status}`);
  }

  async close() {
    this.open_ = false;
    try { await this.device.close(); } catch (_) { /* already gone */ }
  }
}
