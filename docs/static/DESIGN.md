# Mega Blastoise: design

Alex's spec for the project: what he has specified and approved, nothing else. Each line ends with the
date it was decided. Where a later decision replaces an earlier one, only the later one is here. How
it is built is in the documents under `docs/` and `hardware/`, which this document overrides if they
disagree.

## 1. Purpose and goals

Mega Blastoise is a pocket-sized handheld for two players. It lies flat on a table between two
people sitting across from each other, with one shared screen in the middle split between them and
a D-pad and face buttons at each end. They fight Pokemon battles against each other, against the
console, or against players online.

Goals:

1. **Two people can sit down and play, like any video game.** Nothing to set up, a match within
   seconds, and a computer opponent when you are on your own.
2. **It feels like a purpose-built game device**, in the spirit of emulator handhelds. Its whole
   physical design language follows the DS Lite: the curves, the buttons, the feel in the hand.
3. **Its battles are Pokemon Showdown's battles.** A battle on the console plays out as it would on
   Pokemon Showdown, so the same console can take on players online.
4. **It fits in a pocket.** It is never bigger than a smartphone in its case, and every spin makes it
   as small as it can.
5. **It can be built in batches and sold** at a low price.

The technical requirements for the next spin, and what each one needs from the chip, are in
[next_spin_requirements.md](next_spin_requirements.md).

## 2. How it plays

What the players see and do.

- The console lies flat on a table between two people sitting across from each other, and can also be picked up and played standing. (2026-10-28)
- There is one screen in the middle. It is split in two halves, one per player, and the far half is drawn upside down so each player reads their own half the right way up. There is one arrangement and no orientation setting. (2026-08-06)
- During a battle the two halves join into one scene: both Pokemon face each other across the middle, and the story text shows on both halves. (2026-08-11)
- Information is open: each player can see the rival's active Pokemon, its health and its status. Only the move a player has locked in stays hidden until the turn plays. (2026-08-06)
- When a half shows something private (choosing a move, the party, the battle log), it is framed in its seat's colour, red or white, so the whole screen reads as a Poke Ball. (2026-10-28)
- Each player has the same controls: a D-pad, four face buttons (A, B, X, Y), Start and Select, laid out like the bottom half of a DS Lite. (2026-10-19)
- A confirms, B goes back, and X explains what the cursor is on (move details, Pokemon stats). (2026-10-28)
- A player can take back a locked-in choice until both players have chosen. (2026-10-28)
- A local battle can be paused: choosing Sleep from the pause menu saves the battle and turns the console off, and the next power-on goes straight back to it. Online battles can't be paused. (2026-10-29)
- A session runs: pick Gen 1 or Gen 3 (the first and third Pokemon game generations), set options, ready up in the lobby, choose moves, watch the turn play, then see the result and return to the lobby. (2026-10-28)
- The options are team size (3 against 3, or 6 against 6), text speed and a tutorial before the first game. (2026-10-28)
- Teams are random every battle, rematches included. Random teams are the only team mode. (2026-10-28)
- A single player can fight a computer opponent by holding A in the lobby. (2026-10-28)
- Pressing X shows the battle log, with damage numbers and the reasons for critical hits and type effects. (2026-10-28)
- When a Pokemon faints, its owner picks the next one while seeing the rival's side. (2026-10-28)
- Players can play against others online, as a guest on Pokemon Showdown (the free online Pokemon battle site) with no login. (2026-10-26)
- It starts fast: a match within seconds of power-on. (2026-10-28)

## 3. The hardware

What the physical device is.

- It is a two-seat handheld: one 3.5 inch IPS screen (a wide-viewing-angle screen type) shared by both seats, with DS Lite controls at each end, a battery, one USB-C port and a speaker. (2026-10-26)
- The physical design follows the DS Lite: the curves, the buttons and the feel in the hand. The aim is that it feels like holding the bottom half of a DS Lite. (2026-10-19)
- The buttons are DS Lite silicone pads (the soft rubber button sheets) pressing on bare contacts on the circuit board. (2026-09-21)
- Each seat has a D-pad, A/B/X/Y, Start and Select. (2026-10-19)
- Power is seat 1's Start button: hold it for 2 to 3 seconds to turn on, so a bump in a pocket doesn't switch it on. (2026-10-29)
- It stays about today's size or smaller, never bigger than a smartphone in its case, and every new spin is made as small as it can be. (2026-10-28)
- The circuit board has two layers (the cheaper kind) and is assembled by a factory (JLCPCB) in batches. (2026-09-24)
- There are no indicator lights on the board. (2026-09-24)
- There is no separate reset button. (2026-10-28)
- Holding seat 1's Start for about 8 seconds turns the console off, even if it has frozen; there is no shorter hold to turn it off. (2026-10-30)
- Holding seat 1's Select while turning the console on starts update mode: it shows up on a computer as a USB drive, and copying one file onto it updates it or restores it from any state. (2026-10-29)
- The board has test pads and a debug connector for bring-up. (2026-10-19)
- Debug builds can log every battle line to flash and show the log on the next boot. Production builds have this switched off. (2026-10-28)
- The battery is one lithium-ion cell. It sits in a hole in the board, under the screen, with its underside in a pocket in the case floor. It is turned 90 degrees so its long side runs along the board. (2026-10-27)
- The speaker sits in the side margin of the board beside the screen and plays up through slots in the face. (2026-10-22)
- Nothing is soldered when a unit is put together: the battery and the speaker both plug in. (2026-10-20)
- The case is 3D printed and built from two parts: a tub and one flowing top piece. (2026-10-25)
- The top piece is a full-width band at the screen, with the DS Lite slope at its two button ends only. (2026-10-26)
- The next spin uses the RP2350B (a Raspberry Pi microcontroller) with a 16 MB flash memory chip beside it, which holds the program and the sound. There is no Wi-Fi and no SD card. (2026-10-29)

## 4. The software

How it behaves.

- Battles match Pokemon Showdown exactly. It covers Gen 1 and Gen 3 singles, including abilities and held items. Showdown is the reference, and it wins every disagreement. (2026-08-16)
- The battle is split the way Showdown itself is split: a battle simulator, and a separate viewer that only reads Showdown's messages. An online battle therefore uses the same messages as a local one. (2026-10-23)
- The simulator follows Showdown's Random Battles rules by default. Gen 1 offers both the cartridge rules and the Custom Game (plain math) rules, with the cartridge rules on by default. (2026-10-23)
- A tie shows as a tie on its own draw screen, with no health tiebreak. (2026-10-23)
- Online play is as a Showdown guest only: no login, no accounts. It uses Showdown's own servers; there is no server of ours. (2026-10-28)
- Our own games run as native programs, with no WebAssembly or script engine on the device. (2026-10-27)
- Online play goes through a phone: the console plugs into an Android phone over USB, and a web page in the phone's Chrome browser passes the battle between the console and Showdown. (2026-10-29)
- At power-on the Pokemon battler starts on its own within about a second: our own Rust program, with no operating system under it. (2026-10-29)

## 5. Next spin

The next hardware spin's requirements, and what each needs from its chip, are in [next_spin_requirements.md](next_spin_requirements.md). (2026-10-28)

Last updated: 2026-10-30 07:34 EDT
