# Bill of materials (2-layer build)

Two lists: what the factory (JLCPCB) makes and assembles, and what you buy and fit
yourself before a unit is sold. Prices are per unit at a 10-unit order, with the
50-unit figure beside it, checked in the BOM audit of 2026-10-21 unless a line says
otherwise. "Quote" means a price from a real quote, cart or listing; "estimate"
means it isn't one yet.

## Factory: JLCPCB makes and assembles the board

The exact lists are `board/fab/battler_bom_jlc.csv` and `battler_cpl_jlc.csv`,
generated from the board. `board/tools/cost.py` prices them from the JLCPCB parts
snapshot used for this design (July 2026); it leaves out JLC's attrition and minimum
order quantities, so the parts line below uses JLC's live prices with them included.

| | 10 boards | Per unit | Per unit at 50 | Source |
|---|---|---|---|---|
| Bare boards: 2 layers, 71.5 x 151.34 mm, 1.6 mm, ENIG, every hole >= 0.3 mm | $33.60 | $3.36 | $1.61 ($80.50 for 50) | JLC instant quote, 2026-10-21 (the `tools/quotes/` form, run at 71.5 mm) |
| Parts: 49 part numbers, 118 placements a board, with JLC's attrition and minimum quantities | $62.40 | $6.24 | $5.18 | JLC parts API, 2026-10-21 ($5.70 a unit at 10 without attrition) |
| Assembly fees, Economic PCBA (below) | $50.26 | $5.03 | $1.55 ($77.40 for 50) | JLC's published rates |
| USB-C through-hole tabs by hand, if JLC charges it (JLC lists the connector as SMT, so probably not) | $4.24 | $0.42 | $0.14 | JLC's published rates |
| Shipping, UPS Express Saver with duties paid at checkout (DDP); DHL DDP is $31.59 for 10 | $26.64 | $2.66 | $0.80 ($39.84 for 50) | quote, 2026-10-21; the duty itself isn't known yet |
| **Total** | **about $177** | **about $17.71** | **about $9.27** | |

| Assembly fee (Economic PCBA, 2-50 boards) | 10 boards |
|---|---|
| Setup | $8.18 |
| Stencil | $1.53 |
| Solder joints: 424 a board (JLC's joint count per part, 2026-10-21), $0.0016 each | $6.78 |
| Feeder loading: 11 extended part numbers (the thermistor and the 27 ohm USB resistors among them; U1, the ESD array, is hongjiacheng's Preferred SRV05-4A since 2026-10-20, and J4 reuses J2's part number), $3.07 each (basic and preferred parts: none) | $33.77 |
| **Total** | **$50.26** |

Economic PCBA stops at 50 boards; above that, Standard PCBA charges feeder loading on
every part, so keep each order at 50 or fewer.

The priciest parts (2026-10-21, at 10): RP2354B $1.45, the panel's connector (Hirose
FH12-50S) $0.70, the Abracon crystal $0.53 (twice July's price), the backlight op-amp
(MCP6002) $0.30, the Abracon inductor $0.28, the 22 uF VSYS capacitor $0.19, TP4056
$0.19, PicoBlade battery connector $0.14.

Stock on 2026-10-21 (JLC; LCSC had none of these): **the board can't be ordered
today.** The RP2354B (U4) is out (0, with a pre-sale backlog; 1665 on 2026-10-07) and
so is the Abracon core inductor L2 (1 left, 11 needed for 10 boards). Both need a
restock, or reels bought elsewhere and consigned to JLC. Close behind: U1, the SRV05-4A
ESD array (3 left), the JST SH debug connectors J5/J6 (2 left), and the ME6211 regulator
U3 (724, down from 357k in July: enough for 50 boards, so reserve them). The AP3156 ran
out on 2026-10-07 and was replaced by the discrete backlight sinks.

The same board elsewhere, from each maker's instant quote: PCBWay is $65.09 for the
bare boards and $165.33 for assembly labour, parts extra after their review (not
re-checked on 2026-10-21). The quotes page tracks both makers.

Not placed by JLC (no LCSC number): the 20 membrane contacts (bare gold), test
pads, locating holes and fiducials. The panel's ZIF connector
(J3) is placed by JLC like any other part.

## You: bought separately, fitted before sale

| Item | Per unit | For 10 | Must match | Source | Per unit cost | At 50 |
|---|---|---|---|---|---|---|
| EastRising ER-TFT035IPS-6 3.5in IPS panel | 1 | 12 (2 spares) | the "No Touch" option; ILI9488, 320x480, 54.50 x 83.00 x 2.30 mm, a 50-contact 0.5 mm FPC for a ZIF connector (J3 on the board; a connector bundled with the panel isn't needed) | AliExpress, buydisplay.com Store (the maker's own): $11.24 each, 5% off at 10 or more, $13.89 shipping for 12, so 12 for $142.03. The maker's own site: $9.10 each but $45.48 shipping, 12 for $154.68 (both 2026-09-24; not re-checked on 2026-10-21, the site blocks scripted access) | $14.20 | $9.86 |
| Li-ion cell 403450, about 700 mAh | 1 | 12 (spares) | 4 x 34 x 50 mm (with its protection board), protection board, Molex PicoBlade (MX1.25, often listed as "JST 1.25") plug with red to J2's `+`: order the cells with that plug; a UN38.3 test summary to ship units to customers | AliExpress, Easylander Official Store: 10 for $22.69, free shipping (the earlier MYWAY listing is gone; this one shows CE only, so ask for the UN38.3 summary) | $2.27 | $2.20 |
| Speaker: INGHAi GSPK151035PN (LCSC C530539), 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB | 1 | 10 | its 35 mm leads with the 1.25 mm 2-pin plug (it plugs into J4, a PicoBlade header) | LCSC: $0.37 at 10+ | $0.37, plus about $1.00 of LCSC shipping (estimate: $10 an order) | $0.33 + $0.20 |
| DS Lite silicone pads (NDSL): D-pad, ABXY and Start/Select pieces | 2 sets | 20 sets | "For NDSL"; the kit must include the Start/Select strip, which covers SW20 to SW23 (seat 1's Start is the power button) | AliExpress, Ganer: 5 sets for $2.69, free shipping (a new seller; JCD at $1.30 a set is the fallback) | $1.08 | $1.08 |
| DS Lite button sets: cross, A/B/X/Y and Start/Select | 2 sets | 20 sets | DS Lite; the L/R and other pieces are spares | AliExpress, LindaLiu Store: $1.35 a set, 2% off 2 or more, $4.89 shipping | $3.13 | $2.74 |
| Screws: M2 x 8 self-tapping, wafer head (4 mm, about 1 mm thick), for plastic | 6 | 60 (spares) | a head no wider than 4 mm | AliExpress, Fastener Store: 100 for $1.71 + $4.91 shipping | $0.66 | $0.20 |
| Thin double-sided tape for the panel, adhesive foam pad for the cell | | 1 roll, 1 sheet | | marketplace | estimate about $0.50 | $0.50 |
| Case: tub and lid, printed at home | 1 | 10 | PETG or PLA, 47 cm3 (about 59 g) of filament | at $20-30 a kg | about $1.60 | $1.60 |
| **Total** | | | | | **about $24.81** | **about $18.71** |

Gone from the 4-layer build's list: the cover lens, the six screws, the light pipes
and the printed power-button cap.

Hand work per unit: [assembly.md](assembly.md).

## Per unit, into your hands

About **$42.52** at 10 units (the factory's $17.71 and your $24.81) and about
**$27.98** at 50 (the factory's $9.27 and your $18.71), from the BOM audit of
2026-10-21. The previous figure was about $39-40 at 10 (2026-09-24, parts repriced
2026-10-20). Most of the rise is parts: price rises (the crystal doubled, MLCCs are up 3
to 6 times) and JLC's attrition, which that figure left out; the screws, the speaker's
shipping and the button caps add a little. Not in it: import duties (at the 25% rate an
earlier review suggested, unverified, about $3.66 a unit on the board order at 10 and
$2.08 at 50) and JLC's new-account coupons ($20 and $15 on the quote page, up to $3.50 a
unit off at 10 if they apply). The tape, the foam, the filament and the speaker's
shipping are still estimates, and the panel price dates from 2026-09-24.

The panel is the largest single item. At 50 panels the maker's own site is the
cheapest source: $493.10 with shipping, $9.86 a panel (quotes from both shops'
carts, 2026-09-24, shipping to the US by their standard methods, duties not
included).
