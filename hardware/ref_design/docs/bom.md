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
| extended-part setup (JLC charges once per part number per order) | about $48 per order |

The five priciest parts are the flash ($2.18), BQ24074 ($1.28), MAX98357A ($1.15),
TPS63001 ($1.12) and RP2350B ($1.08); the backlight driver (AP3156) is next at $0.63.
Stock to watch: BQ24074RGTR (217 at JLC when checked), TPS63001DRCR (1356) and
AP3156FVG-7 (1490). Alternatives are in [design_review.md](design_review.md).

Cheaper later, not needed now: two extended parts could become basic ones for
about $6 less setup per order (27 ohm USB resistors to 22 ohm, the NTC to a basic
10k B3950).

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
| Case: tub, lid, power cap | 1 | $15-30 | resin print service; FDM works for a test fit |
| Cover lens, 1 mm clear acrylic, laser cut | 1 | $1-3 | `case/lens.dxf` |
| M2 x 16 self-tapping pan-head screws | 6 | $0.05 | |
| Light pipe, 2 mm clear acrylic rod, 5.5 mm | 2 | $0.10 | |
| Double-sided tape for the panel, foam pad under the cell | | $0.50 | the panel hangs from the lid's window ledge |

Off the board, one unit comes to about $37-68, most of it a service-printed case
(about $22-38 with the case printed at home). With the board's parts (about $14 a
board at 10, setup fees included) that is roughly $51-82 per finished unit, before
the bare boards (4 layers, ENIG, 70 x 151 mm) and JLC's per-order assembly fees.
The panel was the largest item at $25-35 (a Displaytech DT035) until the QD3525
replaced it.
