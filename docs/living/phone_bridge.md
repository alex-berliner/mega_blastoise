# Phone bridge: the board plays on Pokemon Showdown through a phone

Purpose: VER-251 (the chip gate in `hardware/vv/verification.csv`). The stripboard
Pico 2 is a USB device on one cable to an Android phone. A static page in the
phone's Chrome opens the board by WebUSB, joins Showdown's own server as a guest
(no login, no account, no server of ours) and relays a battle between the two.
Rule (DESIGN.md:86, 2026-10-30): the bridge works with the existing hardware or
minimal additions; no antenna.

## Step 1: can a guest battle at all from a page on another origin? Yes.

Read in `smogon/pokemon-showdown` at `2ee95b4` (2026-10-30) and
`smogon/pokemon-showdown-client` at `02c6768`:

| Question | Answer | Where |
|---|---|---|
| Can a guest search the ladder? | Yes. `/search` has no `user.named` check. | `server/chat-commands/core.ts:1484-1501` |
| Can a guest challenge a user? | No: "You must choose a username". The bridge does not use challenges. | `server/chat-commands/core.ts:1531` |
| Is the search path gated on registration? | No. Only `Config.laddermodchat` (unset by default) and `forceregisterelo` (a high rating, not reached by a guest). | `core.ts:1485-1496`, `server/ladders.ts:356-374` |
| Do random battles need a team? | No. `gen1randombattle` / `gen3randombattle` are generated server side. | `config/formats.ts:4640`, `:4654` |
| Does the websocket check Origin? | No. There is no `origin` in `server/sockets.ts`. A browser WebSocket is not subject to CORS either way. | `server/sockets.ts` |
| Can a page on another origin get a name assertion from `action.php`? | No. The endpoint sends no `Access-Control-Allow-Origin` (checked with `curl -i -H 'Origin: ...'`, 2026-10-30), so a cross-origin page cannot read the reply. The bridge never needs it: it stays a guest. | `play.pokemonshowdown.com/action.php` |
| Who can a guest be matched with? | Another searcher on a different IP. Two guests behind one IP never match. | `server/ladders.ts:393-394` (`matchmakingOK`, skipped only by `Config.noipchecks`) |

Live check (2026-10-30, `wss://sim3.psim.us/showdown/websocket`, Python, no login,
`Origin: https://example.invalid`): the server greeted as `Guest 5126328`, accepted
`|/search gen1randombattle` and answered `|updatesearch|{"searching":["gen1randombattle"]...}`.
Local check against an unmodified copy of the server (two guests, `--no-security`
only to lift the same-IP rule): both generations matched the two guests and played.

Consequences:

- The page needs no proxy, no login and no server of ours. HTTPS page to `wss://` is
  not mixed content.
- On the live server the opponent is whoever else is searching that format. To pick
  the opponent, use a second device on a different network (the phone on mobile data
  and a laptop on Wi-Fi: two IPs) searching the same format. Not required by VER-251.
- Rated: random battles are laddered, so the games count for the guest names'
  ratings. Harmless; guests are anonymous and temporary.

## How it works

```
 board (Pico 2)  <--USB bulk, newline lines-->  bridge page (Chrome)  <--wss-->  Showdown
 core DeviceRunner                              site/bridge/relay.js
 + OnlineTransport                              + board.js (WebUSB)
```

- The page decides nothing about the battle. It forwards websocket lines to the board
  and writes the board's `> text` lines to the websocket, and logs everything.
- The board is the same game loop as a local battle (`DeviceRunner` in
  `mega_blastoise_core`). The one new piece is `crates/mega_blastoise_core/src/online.rs`
  (`OnlineTransport`): the server's lines go through the text codec to `ps_client`'s
  `BattleView`; `|request|` becomes the seat's prompt through `ChoiceCollector`; the
  answer goes back as `/choose ...`. Display and input behaviour is unchanged
  (zero-drift rule). The runner's loop (`runner/ps.rs`) gained a remote mode: it waits for
  the server instead of treating silence as the end.
- The board takes whichever side the server assigns (P1 or P2). The other seat is the
  remote player, never prompted. Play with the buttons of the side the board prints
  (`Showdown battle ...: this board is P2.` on the console, `info battle ... p2` in the page log).
- Wire protocol (documented in `online.rs`): page to board `hello`, `play 1|3 [auto]`,
  `cancel`, `frame`, `< <line>`, `down`; board to page `> <ws text>`, `hello mb1 fw=<commit>`,
  `info ...`, `result gen=.. outcome=.. choices=.. rejected=.. undecoded=..`.
- `play N auto` hands the board's seat to the runner's random AI (the same one as the
  lobby's AI seat), so the 10-battle soak needs no tapping. `rejected` counts the server's
  `|error|[Invalid choice]` lines; an invalid choice is asked again, so the board cannot hang.
- USB (`crates/mega_blastoise_fw/src/subsystems/usb_v2.rs`): composite device `c0de:cafe`:
  the existing CDC console, an empty WebUSB interface (BOS capability, landing page
  `https://alex-berliner.github.io/mega_blastoise/bridge/`), and the data interface (vendor class 0xff,
  one bulk IN, one bulk OUT, 64 bytes). Android Chrome claims only the vendor interface; the OS
  keeps CDC. If a phone refuses the composite device, build with `--features usb-vendor-only` (no CDC).

## Host-side test (no phone)

`tools/bridge/relay_test.mjs` runs the page's own `relay.js` between a real Showdown
server and `target/debug/bridge_board` (core's `DeviceRunner` with the USB link on
stdin/stdout, `crates/mega_blastoise_test/src/bin/bridge_board.rs`). Needs Node 22+.

```
cargo build -p mega-blastoise-test --bin bridge_board
node tools/bridge/relay_test.mjs --ws wss://sim3.psim.us/showdown/websocket --gens 1,3 --log run.log
# against a local server (a clone of smogon/pokemon-showdown, `node pokemon-showdown start --no-security 8000`):
node tools/bridge/relay_test.mjs --ws ws://localhost:8000/showdown/websocket --gens 1,3 --opponent
```

Exit status 0 only if every battle finished with 0 rejected choices, 0 undecoded lines
and 0 disconnects. Unit tests for the transport use lines captured from a real server
(`crates/mega_blastoise_core/tests/fixtures/online_gen1_open.txt`): `cargo test -p mega-blastoise-core online`.

Result, 2026-10-30 (host test, no phone; the board is core's real `DeviceRunner`, the page's real `relay.js`):

| Server | Battle | Outcome | Choices sent | Rejected | Undecoded | Disconnects | Time |
|---|---|---|---|---|---|---|---|
| live `sim3.psim.us`, guest, no login | Gen 1 random | finished (board lost to a human ladder opponent) | 18 | 0 | 0 | 0 | 139 s |
| live `sim3.psim.us`, guest, no login | Gen 3 random | finished (board lost to a human ladder opponent) | 22 | 0 | 0 | 0 | 154 s |
| local copy of the server, bot opponent | Gen 1 random | finished (won) | 82 | 0 | 0 | 0 | 168 s |
| local copy of the server, bot opponent | Gen 3 random | finished (won) | 56 | 0 | 0 | 0 | 115 s |

Both live battles matched within seconds (the Gen 1 and Gen 3 random queues had searchers). The board picks
through the runner's random AI, which waits 2 s per turn (`AI_THINK_MS`), so a battle is about 2 s per turn.


## VER-251 procedure for Alex

You need: the stripboard Pico 2 with the debug probe (to flash), an Android phone with
Chrome, a USB-C OTG adapter (USB-C plug to USB-A socket) and a USB-A to micro-B cable
(the Pico 2's own port). Hardware change to the board: none, no antenna (see below).

Once, before the first run (about 10 min, mostly waiting for Pages):

1. Merge `phone-bridge` to `main` and push `main`. `.github/workflows/pages.yml` copies
   `site/` to GitHub Pages, so the page appears at
   `https://alex-berliner.github.io/mega_blastoise/bridge/`. (About 3 min; the
   Actions tab shows the deploy.)
2. Open that URL on your computer's Chrome once: the page must load (title "Mega Blastoise phone bridge").

Each run (about 5 min to start, 15 to 25 min for the soak):

1. Flash: `cd crates/mega_blastoise_fw && cargo run --release` (probe-rs, as README.md). The release build
   embeds `git rev-parse` of the tree as the firmware commit; commit first so the log
   does not say `+dirty`.
2. Power the board from USB (battery switch off, as on the bench), unplug the probe.
3. Phone: Wi-Fi or mobile data on, Chrome (not another browser). Cable: board micro-B to USB-A to the OTG adapter to the phone.
   Chrome shows a "Mega Blastoise board" notification with the page address; tap it, or
   open the URL by hand.
4. Tap **1. Connect board (USB)**, pick the board in Chrome's chooser, tap Allow (the player's tap
   that VER-251 asks for). The log shows `hello mb1 fw=<commit>`; `board yes`.
5. Tap **2. Connect Showdown**. The log shows `ws open` and a `Guest NNN` greeting.
6. Tap **Run VER-251 soak** (5 Gen 1 then 5 Gen 3, the board plays by itself). Watch `battles`
   climb to 10, `rejected` and `disconnects` stay 0. A battle takes 1 to 3 minutes once matched;
   matching waits for another searcher (usually seconds, longer at quiet hours).
   The soak line `soak PASS: 10 of 10 battles finished, 0 rejected choices, 0 disconnects` is the verdict.
7. Tap **Save log**: Chrome downloads `mb-bridge-<time>.log` (Downloads on the phone). The file header has the
   Chrome version, the firmware commit, the page URL and the disconnect count. Attach it to VER-251.
8. Optional human check (3 to 5 min): with "board plays by itself" unticked, tap **Play Gen 1** and play one
   battle with the board's buttons, on the side the page log shows (`info battle ... p1` or `p2`).

If it fails:

- Chooser lists no device: the cable is charge-only, the adapter is not an OTG one, or the phone does
  not source VBUS. Try another cable; confirm the board's screen is on (powered by the phone).
- `board: Failed to execute 'claimInterface'`: the phone bound the interface. Rebuild with
  `--features usb-vendor-only` (`cargo run --release --features usb-vendor-only`) and retry; note that in the result.
- Searching never matches: nobody else is searching that format; use a second device on another
  network (see Step 1) or retry later. Not a bridge failure.
- `undecoded` above 0 in a `result` line: a server line the codec does not read; the page log has `info undecoded: ...`. Send the log.

## Does the board need a hardware addition?

No change to the board. The existing Pico 2 USB port is the link. For the phone to act as USB host
you need a passive USB-C OTG adapter and a USB-A to micro-B cable (cables, not electronics on the board).
No antenna, no radio: the phone supplies the network.
Open point for the gate: power. The phone sources 5 V on the cable; the board draws display and
backlight current from it. If the phone browns out, power the board from its own supply with the data
lines only (not tested here).

## Not done / limits

- Not run on a phone: the WebUSB and firmware USB paths are built and type-checked (`scripts/check.sh`) but only
  a phone proves them. The page loads in headless Chromium (Chrome 148); everything below the USB
  transfer is exercised by the host test.
- If the server ends the battle while the board's seat is choosing (a forfeit, a drop), the runner
  stops waiting for the answer and ends the battle (`runner/ps.rs`, the `carry` queue).
- Timer: the server may start its turn timer; the AI answers in 2 s, a human must answer in time.
- The firmware commit in the log is the build's git HEAD at flash time.

Last updated: 2026-10-30 13:52 EDT
