# Bill of materials

## On the board, placed by JLCPCB

The exact list is `board/fab/battler_bom_jlc.csv` (and the placements
`battler_cpl_jlc.csv`), both generated from the board. `board/tools/cost.py`
prices it from the JLCPCB parts snapshot used for this design (July 2026):

| | 10 boards |
|---|---|
| unique part numbers | 46 (19 extended, 27 basic or preferred) |
| placements per board | 94 |
| parts per board | about $9.40 |
| extended-part setup (JLC charges once per part number per order) | about $57 per order |

The five priciest parts are the flash ($2.18), BQ24074 ($1.28), MAX98357A ($1.15),
TPS63001 ($1.12) and RP2350B ($1.08). Stock to watch: BQ24074RGTR (217 at JLC when
checked) and TPS63001DRCR (1356). Alternatives are in [design_review.md](design_review.md).

Cheaper later, not needed now: three extended parts could become basic ones for
about $9 less setup per order (27 ohm USB resistors to 22 ohm, the 3.6 ohm backlight
resistor to two basic resistors in parallel, the NTC to a basic 10k B3950).

## On the board, not placed

| Part | Qty | Notes |
|---|---|---|
| Membrane contacts SW1-SW16 | 16 | bare ENIG combs, no part |
| Speaker pads J4 | 1 | hand-soldered speaker leads |
| Test pads TP1-TP5 | 5 | bring-up probes |

## Off the board

| Item | Qty | Rough cost each | Notes |
|---|---|---|---|
| Displaytech DT035CTFT-IPS-SHB | 1 | $25-35 | distributors (RS, Digi-Key, Seacomp) |
| 103450 Li-ion cell, protected, JST PH 2.0 leads | 1 | $6-10 | check polarity against J2's `+` |
| 20 mm 8 ohm 1 W speaker, wire leads | 1 | $1-2 | 4.5 mm or thinner |
| DS Lite silicone pads (D-pad + ABXY) | 2 sets | $2-3 | aftermarket |
| DS Lite caps (cross + A/B/X/Y) | 2 sets | $2-4 | aftermarket |
| Case: tub, lid, power cap | 1 | $15-30 | resin print service; FDM works for a test fit |
| Cover lens, 1 mm clear acrylic, laser cut | 1 | $1-3 | `case/lens.dxf` |
| M2 x 16 self-tapping pan-head screws | 6 | $0.05 | |
| Light pipe, 2 mm clear acrylic rod, 5.5 mm | 2 | $0.10 | |
| Double-sided tape for the panel, foam pad under the cell | | $0.50 | |

Board fabrication (4 layers, ENIG, 70 x 151 mm) and JLC's assembly fees come on
top of the parts. For a handful of prototypes, budget roughly $30-40 per finished
unit beyond the board order itself, most of it the panel.
