# Proposal: drive the panel over an 8-bit parallel bus (60 fps)

Status: proposal only. Nothing in `board/design.py`, the KiCad files or the firmware has
changed. Decide before the 5-board JLC order, because it needs a respin.

## Problem

The panel (EastRising ER-TFT035IPS-6, ILI9488, 320x480) is wired for 4-wire SPI. In SPI
mode the ILI9488 only takes 18-bit colour (3 bytes a pixel), so a full frame is 461 kB
(`docs/bringup.md` step 3).

| Link | Frame time | Full-frame fps |
|---|---|---|
| SPI at the datasheet's 20 MHz | 184 ms | about 5 |
| SPI at the RP2350 ceiling of about 75 MHz (panel may not take it) | 49 ms | about 20 |
| 60 fps needs | 16.7 ms | 221 Mbit/s |

The MCU is not the limit; the panel interface is. Redrawing only what changes helps (at
75 MHz about 34% of the screen fits a 60 fps frame, at 20 MHz about 9%) but full-screen
motion cannot reach 60 fps over SPI.

## Proposal

Switch the panel to the 8080 parallel interface, 8-bit bus, RGB565 pixels (two bytes a
pixel), driven by one PIO state machine plus DMA.

Verified in the datasheets (`datasheets/er-tft035ips-6.pdf`, `datasheets/ili9488.pdf`):

- The module supports 8080 parallel in 8, 9, 16 and 18-bit modes. IM2-0 selects it:
  `011` is 8-bit, `010` is 16-bit, `111` is the current 4-wire SPI.
- The FPC already carries DB17-DB0 (pins 15 to 32; DB7-DB0 are pins 25 to 32).
- Minimum parallel write cycle is 40 ns (ILI9488 timing table).
- An 8-bit bus can carry 16-bit/pixel RGB565 (ILI9488 section 4.7.3.1).

Frame time on the 8-bit bus: 153,600 pixels x 2 writes = 307,200 writes. At 40 ns that is
12.3 ms (about 81 fps at the limit). 60 fps needs a write cycle of about 54 ns, so about
25% margin. A 16-bit bus would halve that (6.1 ms) but needs about 19 GPIOs.

## RP2350 resources

- 3 PIO blocks of 4 state machines each. The display needs 1 state machine, leaving 2
  whole blocks free (plus 3 state machines in the display's block).
- Each PIO block sees 32 GPIOs at a time (GPIOBASE picks the window). The 8 data pins must
  be consecutive and inside one window; WR (the PIO side-set) must be inside it too.
- GPIOs (from the current board, branch `ref-design-35in`, a8ead6a; main's pinmap.md is
  stale): 48 total, 24 spare (0, 1, 5 to 18, 20 to 27), plus the 7 SPI-era LCD pins (32,
  33, 36 to 40) that the new bus can reuse. 8-bit needs 12 (8 data, WR, DC, RST, TE) with CS
  tied low and RD already tied high, so about 19 spares remain. The buttons already go
  through two 74HC165s, so no button moves are needed.

## Example pin assignment (unchecked against routing)

1. Data DB0-DB7 on GPIO 20 to 27, already one consecutive spare run.
2. WR on a spare inside the same PIO window (for example GPIO 18).
3. DC, RST and TE stay on 37, 40 and 32; the other SPI pins (33, 36, 38, 39) become spares.
4. IM2-0 straps from `111` to `011`.

Which run suits J3's routing best is not checked. Changes would go in `board/design.py` and
`board/tools/pinmap.py` on `ref-design-35in`, then the README pipeline.
`hardware/prod_board` is not touched.

## Cost

- Parts: about $0 (same panel and connector; the 33 ohm series resistors on the data lines
  are a few cents).
- Work: a board respin and re-route before the 5-board JLC order, and about 1 to 2 days
  for the PIO display driver. The shared core is unchanged: the panel link is platform IO.

## Open items

- Real timing on the 2-layer board with 33 ohm series resistors is untested. Check signal
  integrity at 8 data lines plus WR before committing to the respin.
- Confirm the panel module's 8080 mode on the bench: set IM to `011` on a prototype and read
  the controller ID.
- Confirm the PIO GPIOBASE options (rp2350.txt line 72117) before fixing which window holds
  the data run (GPIO 20 to 27 in the example) and the WR pin.
- Fallback with no respin: keep SPI and redraw only changed regions.

Last updated: 2026-10-27 07:48 EDT
