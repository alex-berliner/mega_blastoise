# Mega Blastoise: design

Alex's spec for the project: what he has specified and approved, nothing else. Each line ends with the
date it was decided. Where a later decision replaces an earlier one, only the later one is here. How
it is built is in the documents under `docs/` and `hardware/`, which this document overrides if they
disagree.

## 1. Purpose and goals

Mega Blastoise is a pocket-sized handheld for two players. It lies flat on a table between two
people sitting across from each other, with one shared screen in the middle split between them and
a D-pad and face buttons at each end. They fight Pokemon battles against each other, against the
console, or against players online, and the console also plays GBA games.

Goals:

1. **Two people can sit down and play, like any video game.** Nothing to set up, a match within
   seconds, and a computer opponent when you are on your own.
2. **It feels like a purpose-built game device**, in the spirit of emulator handhelds. Its whole
   physical design language follows the [DS Lite](#g-ds-lite): the curves, the buttons, the feel in the hand.
3. **Its battles are Pokemon Showdown's battles.** A battle on the console plays out as it would on
   [Pokemon Showdown](#g-showdown), so the same console can take on players online.
4. **It plays GBA games at full speed**, and two consoles can link with a USB-C cable to play GBA
   games together.
5. **It fits in a pocket.** It is never bigger than a smartphone in its case, and every spin makes it
   as small as it can.
6. **It can be built in batches and sold** at a low price.

The technical requirements for the next spin, and what each one needs from the chip, are in
[docs/next_spin_requirements.md](docs/next_spin_requirements.md).

## 2. How it plays

What the players see and do.

- The console lies flat on a table between two people sitting across from each other, and can also be picked up and played standing. (2026-10-28)
- There is one screen in the middle. It is split in two halves, one per player, and the far half is drawn upside down so each player reads their own half the right way up. There is one arrangement and no orientation setting. (2026-08-06)
- During a battle the two halves join into one scene: both Pokemon face each other across the middle, and the story text shows on both halves. (2026-08-11)
- Information is open: each player can see the rival's active Pokemon, its health and its status. Only the move a player has locked in stays hidden until the turn plays. (2026-08-06)
- Each player has the same controls: a D-pad, four face buttons (A, B, X, Y), Start and Select, laid out like the bottom half of a DS Lite. (2026-10-19)
- A confirms, B goes back, and X explains what the cursor is on (move details, Pokemon stats). (2026-10-28)
- A player can take back a locked-in choice until both players have chosen. (2026-10-28)
- A session runs: pick Gen 1 or Gen 3 (the first and third Pokemon game generations), set options, ready up in the lobby, choose moves, watch the turn play, then see the result and return to the lobby. (2026-10-28)
- The options are team size (3 against 3, or 6 against 6), text speed and a tutorial before the first game. (2026-10-28)
- Teams are random every battle, rematches included. Random teams are the only team mode. (2026-10-28)
- A single player can fight a computer opponent. (2026-10-28)
- Pressing X shows the battle log, with damage numbers and the reasons for critical hits and type effects. (2026-10-28)
- When a Pokemon faints, its owner picks the next one while seeing the rival's side. (2026-10-28)
- The console plays GBA (Game Boy Advance) games at full speed, with Pokemon FireRed as the test game. (2026-10-28)
- Two consoles can link with a USB-C cable to play GBA games together. (2026-10-28)
- Players can play against others online, as a guest on Pokemon Showdown (the free online Pokemon battle site) with no login. (2026-10-26)
- Games and save files load from a computer over USB, or from an SD card. (2026-10-28)
- It starts fast: a match within seconds of power-on. (2026-10-28)
- The attract demo (a self-playing screen shown while idle) is not part of the product. (2026-10-28)
- Not in scope for now: doubles battles and elaborate per-move animations. (2026-10-28)

## 3. The hardware

What the physical device is.

- It is a two-seat handheld: one 3.5 inch IPS screen (a wide-viewing-angle screen type) shared by both seats, with DS Lite controls at each end, a battery, one USB-C port and a speaker. (2026-10-26)
- The physical design follows the DS Lite: the curves, the buttons and the feel in the hand. The aim is that it feels like holding the bottom half of a DS Lite. (2026-10-19)
- The buttons are DS Lite silicone pads (the soft rubber button sheets) pressing on bare contacts on the circuit board. (2026-09-21)
- Each seat has a D-pad, A/B/X/Y, Start and Select. (2026-10-19)
- Seat 1's Start button is also the power button. (2026-10-19)
- It is never bigger than a smartphone in its case, and every new spin is made as small as it can be. (2026-10-28)
- The circuit board has two layers (the cheaper kind) and is assembled by a factory (JLCPCB) in batches. (2026-09-24)
- There are no indicator lights on the board. (2026-09-24)
- The board has test pads and a debug connector for bring-up. (2026-10-19)
- The battery is one lithium-ion cell. It sits in a hole in the board, under the screen, with its underside in a pocket in the case floor. It is turned 90 degrees so its long side runs along the board. (2026-10-27)
- The speaker sits in the side margin of the board beside the screen and plays up through slots in the face. (2026-10-22)
- Nothing is soldered when a unit is put together: the battery and the speaker both plug in. (2026-10-20)
- The case is 3D printed and built from two parts: a tub and one flowing top piece. (2026-10-25)
- The top piece is a full-width band at the screen, with the DS Lite slope at its two button ends only. (2026-10-26)
- The board's side screws sit inward, with no shoulders. (2026-10-22)
- The board may be up to 2 mm wider than first planned. (2026-10-22)
- Hardware ownership: `hardware/prod_board` and `hardware/prod_case` are Alex's own learning project, changed only when he asks. `hardware/ref_design` is Claude's complete reference design. (2026-09-24)
- The chip is being chosen (queue issue #4). The current reference design uses the RP2354B (a Raspberry Pi microcontroller with its flash memory inside). (2026-10-28)

## 4. The software

How it behaves.

- Battles match Pokemon Showdown exactly. It covers Gen 1 and Gen 3 singles, including abilities and held items. Showdown is the reference, and it wins every disagreement. (2026-08-16)
- If matching Showdown settles a choice, do it and report it. Ask Alex only when Showdown does not clearly decide. (2026-10-22)
- The code is read from Showdown's source before it is written, and tested against Showdown afterwards. (2026-08-16)
- The battle is split the way Showdown itself is split: a battle simulator, and a separate viewer that only reads Showdown's messages. An online battle therefore uses the same messages as a local one. (2026-10-23)
- Gen 1 is built first, then Gen 3. (2026-10-23)
- The simulator follows Showdown's Random Battles rules by default. Gen 1 offers both the cartridge rules and the Custom Game (plain math) rules, with the cartridge rules on by default. (2026-10-23)
- A tie shows as a tie on its own draw screen, with no health tiebreak. (2026-10-23)
- Online play is as a Showdown guest only: no login, no accounts. (2026-10-26)
- The network is Wi-Fi only if it is nearly free; otherwise a USB host port (the console acts as the computer end) lets a phone tether or a Wi-Fi dongle provide it. This replaces the earlier "no radio" decision. (2026-10-28)
- The console boots fast. Linux (a full operating system) is allowed only if it boots fast, and a bare-metal Rust boot (our own program with no operating system under it) is required. (2026-10-28)
- Games and saves load from a computer or an SD card. (2026-10-28)
- The console ships no Pokemon assets, game ROMs or GBA BIOS (the GBA's built-in startup code). The player supplies their own. (2026-10-28)

## 5. Next spin

The next hardware spin's requirements, and what each needs from its chip, are in [docs/next_spin_requirements.md](docs/next_spin_requirements.md). (2026-10-28)

Waiting on Alex:

- **Which chip, and which storage.** Being chosen in queue issue #4. (2026-10-28)
- **How Pokemon and GBA share the chip.** Either a boot select (the bare-metal battler starts by default, and Linux with the GBA emulator is a selectable boot), or everything on Linux. (2026-10-28)
- **One USB-C port has four jobs:** charging, the computer link, a phone or dongle, and console-to-console link play. It needs role switching. (2026-10-28)
- **The phone-in-case size limit** in millimetres is still to be measured. (2026-10-28)

## 6. Confirm or cut

Tick what stays and strike what goes. Each of these is probably important, or was decided earlier, but I could not tie it clearly to a current decision of yours.

- [ ] **Zero drift.** The browser build and the device show and do exactly the same things, and the device is the truth (your rule from 2026-07-09 and 2026-08). The 2026-10-28 goals dropped it as a goal. Keep it as a testing rule, or cut it?
- [ ] **Wi-Fi on the ESP32-S3 board.** On 2026-10-27 you said Wi-Fi is on from that board. On 2026-10-28 you said Wi-Fi only if nearly free. Does the ESP32-S3 board still stand?
- [ ] **No on-device interpreter.** On 2026-10-27 you said definitely no WebAssembly or interpreter on the device. Does it still hold with a Linux option?
- [ ] **Relay server.** On 2026-10-25 the plan was a small relay server run by us. On 2026-10-26 you chose guest play on Showdown's own servers. Drop the relay?
- [ ] **Power on.** The reference design lets either seat's Start turn the console on, but you specified seat 1's Start. Which one is it?
- [ ] **Reset button.** An older note wants a dedicated hold-to-reset button. The reference design has only a pinhole reset. Keep or cut?
- [ ] **Flash logging.** An older note wants every battle line saved to flash and shown on the next boot. Keep or cut?
- [ ] **Poke Ball frame.** Each private half is framed red or white so the whole screen reads as a Poke Ball. Keep or cut?
- [ ] **Lobby shortcut and tutorial.** Holding A in the lobby summons the computer opponent, and a tutorial shows before the first game. Keep both?
- [ ] **Gen 3 labelled "preview"** in the picker. Keep or drop the label?

Last updated: 2026-10-28 13:03 EDT
