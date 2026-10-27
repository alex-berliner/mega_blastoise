# Proposal: next revision (multi-game platform, SD, USB bridge, 60 fps)

Status: proposal only, from the 2026-10-26 ideation chat. Nothing in `board/design.py`, the
KiCad files, the case or the firmware has changed. Several items need a respin, so decide
before the 5-board JLC order. Items marked "unverified" were not checked against a datasheet
or a live source.

Related: `docs/display_parallel_proposal.md` (the 60 fps display link, in full detail),
`docs/bom.md`. The current board is on branch `ref-design-35in` (worktree
`mega_blastoise_35in`, commit a8ead6a when this was written); its `docs/pinmap.md` is the
GPIO map this proposal uses. Main's `docs/pinmap.md` is stale (direct-GPIO buttons and
revision straps, both gone).

## 1. Direction

Pokemon may become one game of many instead of the first-class game. That raises the value of
removable storage and a way to load teams, saves and data without reflashing.

Decision (Alex, 2026-10-27): both. Some games are built into the firmware, others are game
files, and the launcher finds game files on both the SD card and an internal flash
filesystem.

### Game catalog

The launcher builds one list from three sources:

1. Built-in games, compiled into the firmware (Pokemon today).
2. Game files on an internal flash filesystem (a littlefs partition in the 2 MB; its size
   depends on how big the firmware ends up, not checked).
3. Game files on the SD card, under one folder (for example `/games`).

Rules:

- Each game file carries a manifest with an id and a version. If the same id is in more than
  one place, the launcher shows it once and runs the newest.
- A game on SD is listed only while the card is in. An "install" action copies it to
  internal flash so it works without the card.
- Saves go next to where the game runs from, or to internal flash if the card might be
  removed (to decide).
- Mass storage mode (section 2) exposes the SD card only; the internal filesystem is reached
  through the device's own UI or the USB bridge.

### Game file format: WASM (proposed)

A game file is a `.wasm` module plus its manifest. The firmware runs it in a WASM
interpreter (for example `wasmi`, which supports `no_std`; fit in 520 kB of RAM not
checked). The web build runs the same file in the browser's own WASM engine.

Why WASM over native code or Lua:

- One file runs on both the device and the web build. That carries the zero-drift rule over
  to loaded games: the device and the browser run the same game file, not two ports.
- Sandboxed: a broken or hostile game file cannot corrupt the firmware or the flash.
- Speed: an interpreter is much slower than native code (roughly 10 to 50 times, not
  measured), so the host does the heavy work. Games call host functions to draw (blit, fill,
  text), read input, play audio and read or write saves. Turn-based and simple 2D games fit;
  fast full-screen games may need a native built-in instead.

Alternative kept in reserve: native position-independent Cortex-M33 code installed to
internal flash and run in place (XIP). Fast, but no sandbox and no web-build parity.

Host API, manifest format and RAM budget are not designed yet.

## 2. Storage

| Option | Verdict |
|---|---|
| microSD slot | Add. Push-in slot about $0.20 to $0.50 at JLC (unverified), about 1.85 mm tall, so no profile change against an 11.1 to 13.4 mm stack. Cost is about 15 x 15 mm of board and a card cutout in the case wall. |
| 16 MB SPI flash (W25Q128, about $0.50) | Optional. With SD it is mostly redundant; keep only a small settings area in the internal 2 MB. |
| MTP | No. No first-class embedded Rust stack, no native macOS support. |
| USB mass storage (MSC) | Yes, for the SD card. The firmware hands raw blocks to the host and never parses FAT in that mode. Expose it only in a plugged-in, not-in-battle mode so the host and firmware never write at once. |

Hot-pulling a card mid-write can corrupt FAT, so firmware should not write during battle
and should tolerate a missing card.

## 3. Connectivity (no radio this revision)

Decision: no radio now. A phone or PC over USB is the bridge.

- The battler stays a plain USB device. The phone or PC is the USB host and runs the bridge
  in a web page (WebUSB) that joins the device to the Showdown websocket.
- Use a vendor-specific USB interface with bulk endpoints (embassy-usb), not CDC: desktop
  OSes claim CDC before WebUSB can. Windows needs Microsoft OS 2.0 descriptors so WinUSB
  binds; Linux needs a udev rule; macOS needs nothing.
- Optional later: a composite device that also exposes a CDC serial port with the same byte
  protocol, for Firefox and Safari (no WebUSB) and scripting, plus logs. About 2 hours extra.
- Supported: Chrome or Edge on Android, Windows, macOS, Linux. Not supported: iPhone and iPad
  (every iOS browser uses WebKit, which has no WebUSB; EU alternative engines unverified).
- True USB tethering (the phone shares its internet) is impossible on this board: the board
  would have to be the USB host (CC as a host, a 5 V OTG boost, an IP stack and TLS).
- The phone or PC powers and charges the board over the cable.
- Check before relying on it: plug any USB device into the Android phone and open a WebUSB
  demo in Chrome.

### Showdown, guest only

- Custom clients are fine technically: the raw websocket
  `wss://sim3.psim.us/showdown/websocket` accepted a foreign `Origin` and sent a guest session
  and a `|challstr|` (checked 2026-10-26). `[Gen 3] OU`, `[Gen 1] OU` and Gen 1 and Gen 3
  Random Battle are in the live format list.
- The login POST (`action.php`) returned no CORS header, so a browser page cannot read it. Alex
  chose guests only for now: no login, no proxy. A registered login would need a small proxy
  (for example on the home box's Caddy) or a native app.
- Official rules do not mention bots or custom clients; a Smogon staff post says bots are
  tolerated and staff would rather work with developers. Keep turn timing human-like and do
  not spam.
- Unverified: whether a guest can challenge or accept a challenge, and whether guests can use
  the ladder (the server's session message includes an `allowLadderBots` setting whose meaning
  was not found). Test with two guest connections before building on it.
- Design problem: Showdown singles has one local player and the screen has two seats. Online
  mode needs a layout (seat 2's half shows the opponent, or is unused).
- Software to add: the vendor USB interface in the firmware (about half a day), a WebUSB
  transport in the web build (about half a day), and the Showdown websocket in the page. The
  device speaks the protocol through `ps_client` (see the Showdown split rewrite plan).

## 4. Wi-Fi options considered (not chosen)

Prices are single-unit LCSC listings seen on 2026-10-26 and need rechecking.

| Option | Change | Unit cost |
|---|---|---|
| A. RP2354B plus ESP32-C3-MINI-1-N4 as a network co-processor over UART | Keep everything else; add a module, UART and enable line | about $2.44 to $2.72 for MCU plus module (RP2354B $1.43 plus $1.01 to $1.29) |
| B. RP2354B plus Raspberry Pi RM2 module | No second firmware (embassy `cyw43`), TLS then runs on the RP2350 (unverified) | not priced |
| C. Single chip, ESP32-S3 | Full firmware port, loses the RP2350 minimal-design reference | FN8 chip $2.77 at 1,000, $3.84 single, 0 in stock; ESP32-S3-MINI-1 modules not priced |

Option C costs about the same or more per unit than A and far more engineering. If Wi-Fi is
wanted later, A is the pick. Consequences of any Wi-Fi option: an SSID and password entry UI
in the shared core, about 100 to 200 mA while the radio is on (cell is 700 mAh), antenna
keepout and case review, and a respin. Cheap hedge for this revision: an unpopulated UART
header footprint (3 GPIOs; section 6 has room).

## 5. Display: 60 fps

Full detail in `docs/display_parallel_proposal.md`. Summary:

- The panel link is the limit, not the MCU. SPI at the datasheet's 20 MHz is about 5 fps full
  frame; 60 fps needs 221 Mbit/s.
- The module and controller support 8080 parallel (IM2-0 `011` for 8-bit). RGB565 over an
  8-bit bus is two bytes a pixel; a frame is 307,200 writes, about 12.3 ms at the 40 ns
  minimum write cycle (about 81 fps at the limit, about 25% margin over 60 fps).
- One PIO state machine plus DMA drives it. 3 PIO blocks of 4 state machines exist, so 2
  whole blocks stay free.
- Fallback with no respin: keep SPI and redraw only changed regions.

## 6. GPIO budget

From `ref-design-35in`'s `docs/pinmap.md` (a8ead6a): 16 buttons already go through two
chained 74HC165s (SR_PL GPIO 19, SR_CLK 29, SR_DATA 30); the 4 Start and Select buttons are
on direct GPIOs (2, 3, 4, 35); the revision straps were removed on 2026-10-24. Of 48 GPIOs, 24
are used (7 of them the SPI panel link: 32, 33, 36 to 40) and 24 are spare: 0, 1, 5 to 18,
20 to 27.

| Change | New pins | Spare after |
|---|---|---|
| Today | | 24 |
| 8-bit parallel panel: 12 pins (8 data, WR, DC, RST, TE; CS tied low, RD tied high), 7 reused from the SPI link | 5 | 19 |
| SD over SDIO 4-bit (CLK, CMD, D0 to D3) plus card detect | 7 | 12 |
| Unpopulated radio UART header (TX, RX, enable) | 3 | 9 |

Everything discussed fits with about 9 GPIOs to spare. SD in SPI mode instead of SDIO would
save 2 more.

- 8 consecutive spares already exist for the data bus: GPIO 20 to 27 (or any 8 of 5 to 18).
  WR must be in the same PIO window (GPIOBASE picks which 32; check rp2350.txt line 72117).
  Which run is best depends on routing from J3, not checked.
- A 16-bit panel bus (about 20 pins, about 6.1 ms a frame) would also fit, with about 1
  spare, but needs 16 consecutive pins: moving SR_PL off GPIO 19 opens 5 to 27 as one run.
  8-bit is still the pick: half the traces and enough margin for 60 fps.

## 7. Compliance and selling

- Any digital device sold in the US with a clock above 1.705 MHz falls under FCC Part 15
  Subpart B (47 CFR 15.101), so the current version needs EMC testing before it is sold, with
  or without wireless. Wi-Fi adds module-condition constraints; a pre-certified module covers
  only the radio testing.
- Rough lab cost: about $2,000 (FCC and ISED, simple device) up to $3,000 to $8,000 with a
  pre-certified module; CE adds several thousand more (about $4,500 in one estimate). These
  are vendor and lab figures, not independent surveys.
- Personal use and a few home-built units for yourself fall under the 47 CFR 15.23 hobbyist
  exemption. Selling units does not. Enforcement against tiny local sales is unlikely but the
  exposure is Alex's. This is not legal advice.
- The 700 mAh cell with a charger is a separate fire and liability risk that EMC testing does
  not address.
- A pre-compliance scan at a lab or makerspace (a few hundred dollars, unverified) lowers risk.

## 8. Decisions needed

1. Decided 2026-10-27: both, with game files found on SD and internal flash. Still open: WASM
   as the game file format (section 1)?
2. Respin before the 5-board JLC order for the parallel display and the SD slot, or keep SPI
   and redraw only changed regions for now?
3. SD over SDIO 4-bit (faster, 6 pins plus card detect) or SPI mode (4 pins)? Both fit.
4. Is the 30-board run for yourself and friends, or for sale (decides whether EMC testing is
   on the critical path)?
5. Add an unpopulated UART header for a later Wi-Fi module, or leave it out?
6. Android phone, iPhone or both among the people you would hand a device to?

## 9. Not verified

- SD slot part, price and LCSC stock; the ESP32-S3-MINI-1 and RM2 prices.
- Real signal integrity of an 8-bit parallel bus on this 2-layer board with 33 ohm series
  resistors.
- The PIO GPIOBASE window options and the 32-instruction block limit.
- Whether guests can challenge, and whether they can use the ladder.
- That the EastRising module's 8080 mode works on the bench (set IM `011`, read the ID).
- Chrome on Android WebUSB with this device (plug-in test above).

Last updated: 2026-10-27 07:52 EDT
