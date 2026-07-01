# Build instructions - Mega Blastoise single board (rev 3)

Generated from `gen_single_board.py` (netlist-verified: 118 pads, 65 cuts, 41 jumpers, 17 nets, no opens/shorts). Open `mega_blastoise_single_board.diy` in DIYLC alongside this - same layout.

**Coordinates** are `<column letter><hole number>`: columns A..X left to
right, holes 1..55 top to bottom, looking at the **component (top) side**
with the copper strips running vertically underneath. P2 sits at the top
edge, P1 at the bottom.

Each section below is self-contained: solder its jumper wires (thin
insulated wire on the UNDERSIDE, soldered only at the two end holes), then
its components, then move to the next section.

## 0. Parts and tools

- 1x stripboard, 24 strips x 55 holes (A..X)
- 1x Raspberry Pi Pico + 2x20 male headers
- 14x 6 mm tactile switch (legs on a 0.2" x 0.3" footprint)
- 2x SSD1306 0.96" OLED (4-pin header: GND VCC SCL SDA)
- 2x WS2812B LED strip (off-board; 3 wires each)
- MB102 breadboard supply (5 V out) + 2 wires to the board
- ~41 insulated jumper wires, thin (30 AWG wire-wrap wire is ideal)
- spot face cutter (or 3 mm drill bit), soldering iron, multimeter

## 1. Mark and cut the tracks (65 cuts)

Do ALL cuts first - several strips are shared between sections, so cutting
later risks slicing under an already-soldered part.

Cuts are ON a hole (the hole is destroyed). To avoid mirror-image mistakes:
push a pin through each listed hole from the component side, then cut the
copper around the pin hole on the strip side. Verify every cut afterwards
with a continuity meter (probe the two holes either side of the cut).

- **Pico line 1** - hole 46 across cols C..V (20 cuts)
- **Pico line 2** - hole 50 across cols C..V (20 cuts)
- **P1/P2 border** - hole 23 on cols D, E, G, H, J, K, L, M, N, P, R, S, U (13 cuts)
- **Singles** - A3, A6, A9, K10, E13, P13, E30, P30, K35, A43, A46, W52

## 2. Pico and power rails

Wires first (they feed the rails from the Pico):

| net | wire (underside) |
|---|---|
| **3V3** | G48 -> W48 |
| **GND** | E48 -> B49 |

Then the Pico, on the TOP side, pin rows at holes 47 and 54, cols C..V:

- Orientation: **USB connector points LEFT** (toward col A).
- Pin 1 (GP0) is the **bottom-left** pin at C54.
- Top pin row = hole 47 (pins 40..21 left to right); bottom row = hole 54 (pins 1..20).
- VBUS (top-left pin, C47) is intentionally not connected.
- Solder headers to the Pico first, drop it in, check it sits between the
  two cut lines (holes 46 and 50 isolate its pin rows), then solder.

Rails after this section: **B = GND**, **W = 3V3**, **X = 5V** (X stays
dead until the MB102 section).

## 3. Button matrix (24 wires, 14 switches)

All the matrix nets - row nets GP5/7/8/9 and column nets GP10-13:

| net | wire (underside) |
|---|---|
| **GP5** | I52 -> H42  ,  H44 -> S44 |
| **GP7** | L52 -> G31  ,  G33 -> N33  ,  N31 -> U31 |
| **GP8** | M52 -> H10  ,  H12 -> S12 |
| **GP9** | N52 -> G10  ,  G12 -> N12  ,  N14 -> U14 |
| **GP10** | P52 -> E27  ,  E29 -> D29  ,  D26 -> R22  ,  R20 -> P19 |
| **GP11** | Q52 -> P28  ,  P25 -> K5  ,  K7 -> K38  ,  K9 -> E16 |
| **GP12** | R52 -> R30  ,  R32 -> E32  ,  E34 -> D12  ,  D14 -> P10 |
| **GP13** | S52 -> P34  ,  P36 -> E10 |

Then the switches. Each spans two strips 3 columns apart; all four legs
get soldered:

| player | button | left legs (col, holes) | right legs (col, holes) |
|---|---|---|---|
| P2 | **S3** | D1 + D3 | G1 + G3 |
| P2 | **S2** | K1 + K3 | N1 + N3 |
| P2 | **S1** | R1 + R3 | U1 + U3 |
| P2 | **M4** | E5 + E7 | H5 + H7 |
| P2 | **M3** | P5 + P7 | S5 + S7 |
| P2 | **M2** | E20 + E22 | H20 + H22 |
| P2 | **M1** | P20 + P22 | S20 + S22 |
| P1 | **M1** | E24 + E26 | H24 + H26 |
| P1 | **M2** | P24 + P26 | S24 + S26 |
| P1 | **M3** | E39 + E41 | H39 + H41 |
| P1 | **M4** | P39 + P41 | S39 + S41 |
| P1 | **S1** | D43 + D45 | G43 + G45 |
| P1 | **S2** | K43 + K45 | N43 + N45 |
| P1 | **S3** | R43 + R45 | U43 + U45 |

P2's buttons are labeled from P2's seat: their M1 is at the board's
bottom-right of their screen (rows 20-22), their party row S1 S2 S3 reads
right-to-left in board coordinates. Follow the table and it comes out right.

## 4. OLED screens (8 wires, 2 modules)

I2C signals plus each screen's 3V3/GND taps to the rails:

| net | wire (underside) |
|---|---|
| **GP16** | V48 -> M30 |
| **GP17** | U48 -> L30 |
| **GP18** | S48 -> J15 |
| **GP19** | R48 -> K15 |
| **3V3** | L15 -> W15  ,  K32 -> W32 |
| **GND** | M15 -> B15  ,  J30 -> B30 |

Then the headers:

- **P1 OLED**: header at hole 27, screen hangs DOWN (rows 27-37).
  Pin order left to right: GND=J27, VCC=K27, SCL=L27, SDA=M27.
- **P2 OLED**: rotated 180 deg - header at hole 18, screen hangs UP
  (rows 8-18). Pin order left to right: SDA=J18, SCL=K18, VCC=L18, GND=M18.
- Double-check your module silkscreen: if its pin order is not GND-VCC-SCL-SDA,
  tell the generator, do not improvise.

## 5. LED strip connectors (6 wires, left edge)

DIN feeds from the Pico plus the 5V/GND taps for each connector:

| net | wire (underside) |
|---|---|
| **GP20** | Q48 -> A40 |
| **GP22** | N48 -> A1 |
| **5V** | A4 -> X4  ,  A45 -> X45 |
| **GND** | A7 -> B7  ,  A48 -> B48 |

The strips are off-board; their 3 wires each solder into column A:

- **LED strip P2** (top left edge): DIN=A2, 5V=A5, GND=A8.
- **LED strip P1** (bottom left edge): DIN=A41, 5V=A44, GND=A47.

## 6. MB102 supply input (1 wire)

| net | wire (underside) |
|---|---|
| **GND** | W54 -> B54 |

- **MB102 supply** (bottom right): "-" to W53, "+" to X53.
- Power plan: MB102 5V drives the LED strips via the X rail; the Pico runs
  from its own USB; grounds are shared through the B rail.

## 7. Test before power

With the meter on continuity, no power applied:

1. **Rails**: B2-B55 beeps (GND), W2-W46 beeps (3V3), X2-X55 beeps (5V).
   W53 must NOT beep to W2 (cut at W52), but W53-B2 must beep (shared GND).
2. **No rail shorts**: B-W, B-X, W-X all silent.
3. **Every button**: hold it pressed, meter from its row net Pico pin to its
   column net Pico pin - beeps pressed, silent released:

| net | Pico pin (physical) | net | Pico pin (physical) |
|---|---|---|---|
| GP5 (P1 moves) | 7 | GP10 (col 1) | 14 |
| GP7 (P1 party) | 10 | GP11 (col 2) | 15 |
| GP8 (P2 moves) | 11 | GP12 (col 3) | 16 |
| GP9 (P2 party) | 12 | GP13 (col 4) | 17 |

4. **OLEDs**: SDA/SCL of each header to Pico pins - P1: M27-pin21(GP16),
   L27-pin22(GP17); P2: J18-pin24(GP18), K18-pin25(GP19).
5. Plug in USB only (no MB102): OLEDs must come up. Then add the MB102 and
   the strips.

## 8. Firmware note

- P2's OLED is physically rotated 180 deg - the firmware needs to set the
  SSD1306 segment-remap + COM-scan-direction flip for the P2 display.
- Pin map is unchanged from the breadboard build (rows GP5/7/8/9, cols
  GP10-13, OLEDs GP16/17 + GP18/19, strips GP20/GP22).

