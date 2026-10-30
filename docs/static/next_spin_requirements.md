# Next spin: requirements

What the next hardware spin has to do, and what each item needs from its chip. The goals behind
it are in [DESIGN.md section 1](DESIGN.md#1-purpose-and-goals); this file is the technical side and changes
with each spin. Decided with Alex on 2026-10-29 (queue issue
[#4](https://github.com/alex-berliner/claude-queue/issues/4) and chat).

## Requirements

| Feature | Status | What it needs from the chip |
|---|---|---|
| Two seats, one shared 3.5 in IPS screen, DS Lite controls per seat, battery, USB-C, speaker | have | a display bus fast enough for the 320 x 480 panel |
| Random teams from Showdown's random battle sets (Gen 1 or 3, 3v3 or 6v6), the only team mode | have | runs the `no_std` Rust core (about 420 KB of RAM and 1.4 MB of code and data on the RP2350 today) |
| Open information, the battle log on X, taking back a locked choice | have | none beyond the core |
| Sound done right | must-have | room for sound samples: a 16 MB flash chip the chip reads as memory |
| Online play against Showdown as a guest, no login | must-have | a USB device port: an Android phone's Chrome browser (WebUSB) passes the battle between the console and Showdown |
| Fast boot: the battler on screen within about a second | must-have | bare-metal Rust, no operating system |
| Pocketable: about today's size or smaller; a size-minimization pass every spin | must-have | small packages, no growth over today's case (78.9 x 156.48 mm in plan, 9.6 mm thick at the screen) |

## Chosen (Alex, 2026-10-29)

| Part | Choice | Price at 30 (JLC, 2026-10-29) |
|---|---|---|
| Chip | Raspberry Pi RP2350B, QFN-80, no flash inside (C42415655) | $1.14 |
| Flash | GigaDevice GD25Q128, 16 MB QSPI, holds the program and the sound (C2758105) | $1.35 |
| Core layout | Raspberry Pi's 2-layer RP2350B minimal design, flash included, copied as drawn | |
| Panel | today's ILI9488 3.5 in IPS | |
| Network | none on the board; the phone web bridge | |
| Software | the bare-metal Rust battler only | |

Gate before the board design: the phone bridge test (Showdown's server accepts a battle from our
bridge page in Android Chrome). The bridge works with the existing hardware or with minimal additions
to it; no antenna (Alex, 2026-10-30).

Last updated: 2026-10-30 13:01 EDT
