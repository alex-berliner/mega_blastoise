# Bill of materials

## On the board, placed by JLCPCB

The exact list is `board/fab/battler_bom_jlc.csv` (and the placements
`battler_cpl_jlc.csv`), both generated from the board. `board/tools/cost.py`
prices it from the JLCPCB parts snapshot used for this design (July 2026):

| | 10 boards |
|---|---|
| unique part numbers | 40 (16 extended, 24 basic or preferred) |
| placements per board | 91 |
| parts per board | about $9.27 |
| extended-part fee (JLC charges once per part number per order) | about $48 per order (see below) |

The five priciest parts are the flash ($2.18), BQ24074 ($1.28), MAX98357A ($1.15),
TPS63001 ($1.12) and RP2350B ($1.08); the backlight driver (AP3156) is next at $0.63.
Stock to watch: BQ24074RGTR (217 at JLC when checked), TPS63001DRCR (1356) and
AP3156FVG-7 (1490). Alternatives are in [design_review.md](design_review.md).

Cheaper later, not needed now: two extended parts could become basic ones for
about $6 less setup per order (27 ohm USB resistors to 22 ohm, the NTC to a basic
10k B3950).

## JLC's assembly fees

Published rates (jlcpcb.com, "PCB Assembly Cost", read September 2026). The board has
358 solder joints to place and 4 through-hole tabs (the USB-C shell), which JLC may
count as hand soldering (it usually places this connector as SMT). Economic PCBA takes
2-50 boards with IC pitch down to 0.4 mm: the RP2350B's 0.4 mm pitch is right at that
limit. Above 50 boards the order goes to Standard PCBA.

| | 10 boards (Economic PCBA) | 100 boards (Standard PCBA) |
|---|---|---|
| Setup | $8.18 | $25.56 |
| Stencil | $1.53 | $8.21 |
| Solder joints, $0.0016 each | $5.73 | $57.28 |
| Feeder loading: Economic charges extended parts only (16), Standard every part number (40) | $49.12 ($3.07 each) | $61.20 ($1.53 each) |
| USB-C tabs by hand, if charged ($0.0164 a joint + $3.58 labor) | $4.24 | $10.14 |
| **Total** | **$68.80: about $6.90 a board** | **$162.39: about $1.62 a board** |

Parts per board fall from $9.27 at 10 boards to $7.92 at 100.

## Bare boards

Not quoted yet: 4 layers, 70.5 x 151.3 mm, 1.6 mm, ENIG. For scale, JLC's own
example (2024) is 100 pieces of a 100 x 100 mm 4-layer board for $106.30 in total,
about $1.06 each; this board is slightly larger and ENIG costs extra. Small orders
are mostly minimum charges. Upload `board/fab/battler_gerbers.zip` to JLC's instant
quote for the real figure. Shipping and any import duties come on top, shown at
checkout.

## On the board, not placed

| Part | Qty | Notes |
|---|---|---|
| Panel landing J3 | 1 | the panel's tail is soldered here by hand (JLC's paste tins the pads) |
| Membrane contacts SW1-SW16 | 16 | bare ENIG combs, no part |
| Speaker pads J4 | 1 | hand-soldered speaker leads |
| Test pads TP1-TP5 | 5 | bring-up probes |

## Off the board

| Item | Qty | Rough cost each | Notes |
|---|---|---|---|
| QD3525 3.5in 320x480 IPS panel (ST7796U), bare, 48-contact solder tail | 1 | $5-8 | marketplace sellers (AliExpress, Taobao); the maker's drawing is on LCDwiki with its MSP3525 module |
| 103450 Li-ion cell, protected, JST PH 2.0 leads | 1 | $6-10 | check polarity against J2's `+` |
| 20 mm 8 ohm 1 W speaker, wire leads | 1 | $1-2 | 4.5 mm or thinner |
| DS Lite silicone pads (D-pad + ABXY) | 2 sets | $2-3 | aftermarket |
| DS Lite caps (cross + A/B/X/Y) | 2 sets | $2-4 | aftermarket |
| Case: tub, lid, power cap, printed yourself (FDM, PETG or PLA) | 1 | about $1.50 | 51 cm³, about 64 g of filament at $20-30 a kg; material only |
| Cover lens, 1 mm clear acrylic, laser cut | 1 | $1-3 | `case/lens.dxf` |
| M2 x 16 self-tapping pan-head screws | 6 | $0.05 | |
| Light pipe, 2 mm clear acrylic rod, 5.5 mm | 2 | $0.10 | |
| Double-sided tape for the panel, foam pad under the cell | | $0.50 | the panel hangs from the lid's window ledge |

Off the board, one unit comes to about $23-40 with the case printed at home. With
the board's parts and JLC's assembly fees (about $16 a board at 10, $9.50 at 100) that
is roughly $39-56 per finished unit at 10 and $33-50 at 100, before the bare boards,
shipping and duties. A print service is not in that figure: at published
service rates a case would be about $12-40 in FDM or $21-60 in resin one at a time
(no quote yet). The panel was the largest item at $25-35 (a Displaytech DT035)
until the QD3525 replaced it.
