# Next spin: requirements

What the next hardware spin has to do, and what each item needs from its chip. The goals behind
it are in [DESIGN.md section 1](../DESIGN.md#1-purpose-and-goals); this file is the technical side and changes
with each spin. Decided with Alex on 2026-10-28 (queue issues
[#3](https://github.com/alex-berliner/claude-queue/issues/3) and [#4](https://github.com/alex-berliner/claude-queue/issues/4)).

## Requirements

| Feature | Status | What it needs from the chip |
|---|---|---|
| Two seats, one shared 3.5 in IPS screen, DS Lite controls per seat, battery, USB-C, speaker | have | a parallel display bus for 60 fps (8080 or RGB) |
| Random teams from Showdown's random battle sets (Gen 1 or 3, 3v3 or 6v6), the only team mode | have | runs the `no_std` Rust core (about 420 KB of RAM and 1.4 MB of code and data on the RP2350 today) |
| Open information, the battle log on `?`, taking back a locked choice | have | none beyond the core |
| GBA emulation at 60 fps (Pokemon FireRed as the test) | must-have, new | an ARM9 at about 400 MHz or more with 32 MB+ of RAM (F1C200s class); the ESP32-S3 reaches about 20 fps |
| GBA link play between two consoles over a USB-C to USB-C cable | must-have, new | a USB port that can be host or device, with USB-C role detection, so one console hosts the other; an emulator with link-cable support (not yet checked) |
| Online play against Showdown as a guest, no login | must-have, new | a network: Wi-Fi only if nearly free, else USB host for a phone tether or a Wi-Fi dongle |
| Load games and saves from a computer, over USB or SD | must-have, new | USB mass storage or an SD card, and room for a 16 MB ROM plus saves |
| Fast boot: Linux allowed if tuned, bare-metal Rust boot required | must-have, new | a Rust target and documented boot ROM and RAM init |
| Pocketable: no bigger than a smartphone in its case; a size-minimization pass every spin | must-have | small packages, no growth over today's case (78.9 x 156.48 mm in plan, 9.6 mm thick at the screen); the phone-in-case limit in mm is still to be measured |

## Open

- **How Pokemon and GBA share the chip** (decided with the chip in #4):
  - a boot select, with the bare-metal battler by default and Linux with the emulator selectable, or
  - everything on Linux.
- **One USB-C port, four jobs:** charging, the PC link (loading games), the phone tether or dongle, and console-to-console link play. It needs role switching.

Last updated: 2026-10-28
