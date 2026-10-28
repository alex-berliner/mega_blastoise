# Bill of materials (2-layer build)

Two lists: what the factory (JLCPCB) makes and assembles, and what you buy and fit
yourself before a unit is sold. Prices are per unit at a 10-unit order, checked
2026-09-24. "Quote" means a price from a real quote, cart or listing; "estimate"
means it isn't one yet.

## Factory: JLCPCB makes and assembles the board

The exact lists are `board/fab/battler_bom_jlc.csv` and `battler_cpl_jlc.csv`,
generated from the board. `board/tools/cost.py` prices them from the JLCPCB parts
snapshot used for this design (July 2026).

| | 10 boards | Per unit | Source |
|---|---|---|---|
| Bare boards: 2 layers, 72.4 x 151.64 mm (72.5 x 151.74 over the outline), 1.6 mm, ENIG, every hole >= 0.3 mm | $33.40 | $3.34 | the quote for 70.5 x 151.34 (`tools/quotes/`): NOT re-quoted for the 3.5 in board, which is 3% more area |
| Parts: 44 part numbers (14 extended), 118 placements a board | $57.80 | $5.78 | JLC parts database snapshot (`tools/cost.py`, run 2026-10-26 on the 3.5 in board's BOM; the earlier figure was $4.73 for 49 numbers) |
| Assembly fees, Economic PCBA (below) | $53.35 | $5.34 | JLC's published rates |
| USB-C through-hole tabs by hand, if JLC charges it | $4.24 | $0.42 | JLC's published rates |
| Shipping, DHL Express with duties paid at checkout (DDP) | $29.45 | $2.95 | quote; the duty itself isn't known yet |
| **Total** | **about $168** | **about $16.78** | |

| Assembly fee (Economic PCBA, 2-50 boards) | 10 boards |
|---|---|
| Setup | $8.18 |
| Stencil | $1.53 |
| Solder joints: about 425 a board (JLC's count was 386 before the 9 parts added 2026-10-05; the backlight sinks that replaced the AP3156 on 2026-10-07 add about 15, J4's header 4 more on 2026-10-20), $0.0016 each | $6.80 |
| Feeder loading: 12 extended part numbers (the thermistor and the 27 ohm USB resistors among them; since 2026-10-22 U1, the ESD array, is Semtech's SRV05-4.TCT (C13612) and U3, the 3V3 LDO, Richtek's RT9013-33GB (C47773), both Extended and both stocked at JLC so a 30-board order needs no pre-order; J4 reuses J2's part number), $3.07 each (basic and preferred parts: none) | $36.84 |
| **Total** | **$53.35** |

The priciest parts: RP2354B $1.43, the panel's connector (Hirose FH12-50S) $0.81,
the Abracon crystal and inductor $0.27 each, the backlight op-amp (MCP6002) $0.24, PicoBlade
battery connector $0.13, TP4056 $0.17. Stock to watch for a larger run: RP2354B (1665 at
JLC on 2026-10-07) and the Abracon inductor (757); the AP3156 ran out (0 on 2026-10-07)
and was replaced by the discrete backlight sinks.

The same board elsewhere, from each maker's instant quote: PCBWay is $65.09 for the
bare boards and $165.33 for assembly labour, parts extra after their review. The
quotes page tracks both makers.

Not placed by JLC (no LCSC number): the 20 membrane contacts (bare gold), test
pads, locating holes and fiducials. The panel's ZIF connector
(J3) is placed by JLC like any other part.

## You: bought separately, fitted before sale

| Item | Per unit | For 10 | Must match | Source | Per unit cost |
|---|---|---|---|---|---|
| EastRising ER-TFT035IPS-6 3.5in IPS panel | 1 | 12 (2 spares) | the "No Touch" option; ILI9488, 320x480, 54.50 x 83.00 x 2.30 mm, a 50-contact 0.5 mm FPC for a ZIF connector (J3 on the board; a connector bundled with the panel isn't needed) | AliExpress, buydisplay.com Store (the maker's own): $11.24 each, 5% off at 10 or more, $13.89 shipping for 12, so 12 for $142.03. The maker's own site: $9.10 each but $45.48 shipping, 12 for $154.68 | $14.20 |
| Li-ion cell 403450, about 700 mAh | 1 | 12 (spares) | 4 x 34 x 50 mm (with its protection board), protection board, Molex PicoBlade (MX1.25, often listed as "JST 1.25") plug with red to J2's `+`: order the cells with that plug; a UN38.3 test summary to ship units to customers | AliExpress, MYWAY Digital Store: 10 for $19.33 + $1.95 shipping | about $2.13 |
| Speaker: INGHAi GSPK151035PN (LCSC C530539), 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB | 1 | 10 | its 35 mm leads with the 1.25 mm 2-pin plug (it plugs into J4, now a JST SH 2-pin header, SM02B-SRSS-TB, LCSC C160402, so the cell's PicoBlade plug cannot enter it: **the speaker's leads need a 1.0 mm SH plug, SHR-02V-S, fitted by hand; not yet ordered**, V&V round 3) | LCSC | about $0.42 |
| DS Lite silicone pads (NDSL): D-pad and ABXY pieces | 2 sets | 20 sets | "For NDSL"; the Start/Select strip isn't used | AliExpress, HM Trading Store: 20 sets $2.45 + $5.84 shipping | $0.83 |
| DS Lite button sets: cross and A/B/X/Y | 2 sets | 20 sets | DS Lite; the L/R and other pieces are spares | AliExpress, China Game Store: $0.88 a set + $12.23 shipping | $2.98 |
| Screws: M2 x 6 self-tapping, wafer head (4 mm, about 1 mm thick), for plastic | 6 | 60 (spares) | | marketplace | estimate about $0.10 |
| Thin double-sided tape for the panel, adhesive foam pad for the cell | | 1 roll, 1 sheet | | marketplace | estimate about $0.50 |
| Case: tub and top, printed at home | 1 | 10 | PLA, 32 cm3 (about 50 g with the purge), and about 7 g of Bambu Support For PLA for the top's supports (a second filament) | PLA at $20-30 a kg, the support filament at about $40 a kg | about $1.20-1.70 |
| **Total** | | | | | **about $22.5-23.5** |

Gone from the 4-layer build's list: the cover lens, the six screws, the light pipes
and the printed power-button cap.

Hand work per unit: [assembly.md](assembly.md).

## Per unit, into your hands

About **$39-40** at 10 units: the factory's $16.47 and your $22.5-23.5. Not in it: import
duties (at the 25% rate an earlier review suggested, unverified, about $3 a unit
on the board order)
and JLC's new-account coupons ($30 and $20 on the quote page, up to $5 a unit off if
they apply). The spread is the tape and filament, the last estimates.

The panel is the largest single item. At 50 panels the maker's own site is the
cheapest source: $493.10 with shipping, $9.86 a panel (quotes from both shops'
carts, 2026-09-24, shipping to the US by their standard methods, duties not
included). The 2.4in ER-TFT024IPS-3 (2026-10-21 to 2026-10-26) would have been about $12.01 at 10 and
$7.13 at 50, but a case that is 23.5 mm shorter; the 3.5in panel is back (Alex, 2026-10-26) with the case rebuilt round it. Its active area (48.96 x 73.44),
its FPC's folded shape and its 2.35 +-0.1 thickness are the old design's numbers, not re-read from the datasheet (the maker's site was behind
Cloudflare and the Wayback copy was down on 2026-10-26).


## Changed in V&V round 3 (2026-10-27)

| Ref | Was | Now | Decision |
|---|---|---|---|
| R61 | 3.9k (C51721) | 3.74k 1% (C15172, Extended) | DEC-31: worst-case backlight 117.7 mA against the panel's 120 |
| R14 | 10k | 47k (C25792, Basic) | DEC-17 (no clamp: U3's EN stays inside 6 V up to about 9.1 V of VBUS) |
| R43 | 100k | 220k (C25767, Preferred) | DEC-17 (EN1 safe to about 12.4 V) |
| RN1 to RN4 | 10k x4 (C25725) | 47k x4 (C2028, 4D02WGJ0473TCE, Extended) | DEC-24 (membrane pill margin) |
| TH1 | 10k B3950 | 10k B4460 0805 (C525375, TCTR0805F10K0F4460T) | charge window: 48.2 C hot edge worst case, 45 C not reachable with a stocked 10k NTC |
| U10, C54 | none | MCP6001T-I/OT (C116490) buffer, 100n (C1525) | cutoff: GPIO47 leakage no longer flows through the divider (DEC-07) |
| J4 | PicoBlade 2-pin | JST SM02B-SRSS-TB (C160402) | speaker plug cannot enter the cell's port |

## Manufacturing and cost

**Route.** JLCPCB makes the 2-layer board and assembles one side (Economic PCBA). The panel,
cell, speaker, DS Lite silicone and caps, screws and tape are bought separately and fitted by hand.
The case is printed at home on FDM. [assembly.md](assembly.md) is the
order and build procedure; [tools/quotes/](../tools/quotes/README.md) re-runs
JLCPCB's and PCBWay's instant-quote forms headlessly when the design changes.

**Cost, with the date of each figure:**

| Figure | Per unit | Date | Source |
|---|---|---|---|
| v1 parts, excluding enclosure | about $40 | spring 2026 | [ELECTRONICS.md](../../../docs/archive/v1/ELECTRONICS.md) |
| Stripboard v2 parts, bought in hobby packs (one unit) | about $125 | 2026-08 | BUILD.md at tag `stripboard-v2-archive` |
| Reference design bare board, 4-layer with 0.25 mm holes, at 10 | $9.74 | 2026-09-24 | quotes |
| Reference design bare board, 2-layer, at 10 | $3.34 | 2026-09-24 | quotes |
| Reference design, factory part (board, parts, assembly fees, shipping) at 10 | about $16.47 | 2026-09-24, parts repriced 2026-10-20 | bom.md, until the audit replaced it |
| Reference design, all in, at 10 | about $39-40 | same | same |
| **Reference design, latest, at 10** | **$42.52** | **2026-10-21** | BOM audit, now in [bom.md](bom.md) |
| **Reference design, latest, at 50** | **$27.98** | **2026-10-21** | same |

The bom.md figures exclude import duties (not yet known), JLC's new-account coupons and hand
labour. The panel is the
largest single item (about $14 at 10, under $10 at 50); the case is about $1.50 of filament.

**Supply risks.** The 2026-10-21 BOM audit found the RP2354B and its core inductor (L2) out of
stock at JLC and LCSC. **That blocks ordering** until stock returns or an alternative is designed
in. Stock has bitten before: on 2026-10-07 both the AP3156 backlight driver (replaced by discrete
sinks) and the first USB ESD array (replaced by another maker's part) ran out. The panel has a
single maker; its glass class has other sources that would need only case and pin-map changes.

**Order plan.** The validation plan proposes 3 to 5 assembled boards plus 5 bare ones from the same
order (for sectioning, rework practice and the contact test), and a dry run of JLC's order flow
with screenshots of every polarised part before paying.
