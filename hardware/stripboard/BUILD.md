# Build instructions - Mega Blastoise single board (rev 3)

Generated from `gen_single_board.py` (netlist-verified: 118 pads, 65 cuts, 41 jumpers, 17 nets, no opens/shorts). Open `mega_blastoise_single_board.diy` in DIYLC alongside this - same layout.

**Coordinates** are `<column letter><hole number>` using the grid printed
on the board itself. All diagrams and coordinates view the **component
(top) side**, copper strips underneath - in this view the printed columns
read **X on the left through A on the right** (they read A..X only when
the copper side faces you). Holes are 1..55 top to bottom; P2 sits at the
top edge, P1 at the bottom.

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

Cuts are ON a hole (the hole is destroyed). These coordinates are the
board's own printed grid, so you can work directly on the copper side and
read the letters off the print - no mirroring in your head. Verify every
cut afterwards with a continuity meter (probe the two holes either side).

- **Pico line 1** - hole 46 across cols C..V (20 cuts)
- **Pico line 2** - hole 50 across cols C..V (20 cuts)
- **P1/P2 border** - hole 23 on cols D, F, G, I, K, L, M, N, O, Q, R, T, U (13 cuts)
- **Singles** - X3, X6, X9, N10, T13, I13, T30, I30, N35, X43, X46, B52

## 2. Pico and power rails

Wires first (they feed the rails from the Pico):

| net | wire (underside) |
|---|---|
| **3V3** | R48 -> B48 |
| **GND** | T48 -> W49 |

Then the Pico, on the TOP side, pin rows at holes 47 and 54, cols C..V:

- Orientation: **USB connector points LEFT** (toward col X).
- Pin 1 (GP0) is the **bottom-left** pin at V54.
- Top pin row = hole 47 (pins 40..21 left to right); bottom row = hole 54 (pins 1..20).
- VBUS (top-left pin, V47) is intentionally not connected.
- Solder headers to the Pico first, drop it in, check it sits between the
  two cut lines (holes 46 and 50 isolate its pin rows), then solder.

Rails after this section: **W = GND**, **B = 3V3**, **A = 5V**
(A stays dead until the MB102 section).

## 3. Button matrix (24 wires, 14 switches)

All the matrix nets - row nets GP5/7/8/9 and column nets GP10-13:

| net | wire (underside) |
|---|---|
| **GP5** | P52 -> Q42  ,  Q44 -> F44 |
| **GP7** | M52 -> R31  ,  R33 -> K33  ,  K31 -> D31 |
| **GP8** | L52 -> Q10  ,  Q12 -> F12 |
| **GP9** | K52 -> R10  ,  R12 -> K12  ,  K14 -> D14 |
| **GP10** | I52 -> T27  ,  T29 -> U29  ,  U26 -> G22  ,  G20 -> I19 |
| **GP11** | H52 -> I28  ,  I25 -> N5  ,  N7 -> N38  ,  N9 -> T16 |
| **GP12** | G52 -> G30  ,  G32 -> T32  ,  T34 -> U12  ,  U14 -> I10 |
| **GP13** | F52 -> I34  ,  I36 -> T10 |

Then the switches. Each spans two strips 3 columns apart; all four legs
get soldered:

| player | button | left legs (col, holes) | right legs (col, holes) |
|---|---|---|---|
| P2 | **S3** | U1 + U3 | R1 + R3 |
| P2 | **S2** | N1 + N3 | K1 + K3 |
| P2 | **S1** | G1 + G3 | D1 + D3 |
| P2 | **M4** | T5 + T7 | Q5 + Q7 |
| P2 | **M3** | I5 + I7 | F5 + F7 |
| P2 | **M2** | T20 + T22 | Q20 + Q22 |
| P2 | **M1** | I20 + I22 | F20 + F22 |
| P1 | **M1** | T24 + T26 | Q24 + Q26 |
| P1 | **M2** | I24 + I26 | F24 + F26 |
| P1 | **M3** | T39 + T41 | Q39 + Q41 |
| P1 | **M4** | I39 + I41 | F39 + F41 |
| P1 | **S1** | U43 + U45 | R43 + R45 |
| P1 | **S2** | N43 + N45 | K43 + K45 |
| P1 | **S3** | G43 + G45 | D43 + D45 |

P2's buttons are labeled from P2's seat: their M1 is at the board's
bottom-right of their screen (rows 20-22), their party row S1 S2 S3 reads
right-to-left in board coordinates. Follow the table and it comes out right.

## 4. OLED screens (8 wires, 2 modules)

I2C signals plus each screen's 3V3/GND taps to the rails:

| net | wire (underside) |
|---|---|
| **GP16** | C48 -> L30 |
| **GP17** | D48 -> M30 |
| **GP18** | F48 -> O15 |
| **GP19** | G48 -> N15 |
| **3V3** | M15 -> B15  ,  N32 -> B32 |
| **GND** | L15 -> W15  ,  O30 -> W30 |

Then the headers:

- **P1 OLED**: header at hole 27, screen hangs DOWN (rows 27-37). Pin order
  left to right: GND=O27, VCC=N27, SCL=M27, SDA=L27.
- **P2 OLED**: rotated 180 deg - header at hole 19, screen hangs UP (rows
  9-19). Pin order left to right: SDA=O19, SCL=N19, VCC=M19, GND=L19.
- Double-check your module silkscreen: if its pin order is not GND-VCC-SCL-SDA,
  tell the generator, do not improvise.

## 5. LED strip connectors (6 wires, left edge)

DIN feeds from the Pico plus the 5V/GND taps for each connector:

| net | wire (underside) |
|---|---|
| **GP20** | H48 -> X40 |
| **GP22** | K48 -> X1 |
| **5V** | X4 -> A4  ,  X45 -> A45 |
| **GND** | X7 -> W7  ,  X48 -> W48 |

The strips are off-board; their 3 wires each solder into column X:

- **LED strip P2** (top left edge): DIN=X2, 5V=X5, GND=X8.
- **LED strip P1** (bottom left edge): DIN=X41, 5V=X44, GND=X47.

## 6. MB102 supply input (1 wire)

| net | wire (underside) |
|---|---|
| **GND** | B54 -> W54 |

- **MB102 supply** (bottom right): "-" to B53, "+" to A53.
- Power plan: MB102 5V drives the LED strips via the A rail; the Pico
  runs from its own USB; grounds are shared through the W rail.

## 7. Test before power

With the meter on continuity, no power applied:

1. **Rails**: W2-W55 beeps (GND), B2-B46 beeps (3V3),
   A2-A55 beeps (5V). B53 must NOT beep to B2 (cut at
   B52), but B53-W2 must beep (shared GND).
2. **No rail shorts**: W-B, W-A, B-A all silent.
3. **Every button**: hold it pressed, meter from its row net Pico pin to its
   column net Pico pin - beeps pressed, silent released:

| net | Pico pin (physical) | net | Pico pin (physical) |
|---|---|---|---|
| GP5 (P1 moves) | 7 | GP10 (col 1) | 14 |
| GP7 (P1 party) | 10 | GP11 (col 2) | 15 |
| GP8 (P2 moves) | 11 | GP12 (col 3) | 16 |
| GP9 (P2 party) | 12 | GP13 (col 4) | 17 |

4. **OLEDs**: SDA/SCL of each header to Pico pins - P1: L27-pin21(GP16),
   M27-pin22(GP17); P2: O19-pin24(GP18), N19-pin25(GP19).
5. Plug in USB only (no MB102): OLEDs must come up. Then add the MB102 and
   the strips.

## 8. Firmware note

- P2's OLED is physically rotated 180 deg - the firmware needs to set the
  SSD1306 segment-remap + COM-scan-direction flip for the P2 display.
- Pin map is unchanged from the breadboard build (rows GP5/7/8/9, cols
  GP10-13, OLEDs GP16/17 + GP18/19, strips GP20/GP22).

