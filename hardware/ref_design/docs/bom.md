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
| Bare boards: 2 layers, 70.5 x 151.3 mm, 1.6 mm, ENIG, every hole >= 0.3 mm | $33.40 | $3.34 | quote (`tools/quotes/`) |
| Parts: 36 part numbers, 90 placements a board | $41.60 | $4.16 | JLC parts database |
| Assembly fees, Economic PCBA (below) | $42.65 | $4.27 | JLC's published rates |
| USB-C through-hole tabs by hand, if JLC charges it | $4.24 | $0.42 | JLC's published rates |
| Shipping, DHL Express with duties paid at checkout (DDP) | $29.45 | $2.95 | quote; the duty itself isn't known yet |
| **Total** | **about $151** | **about $15.10** | |

| Assembly fee (Economic PCBA, 2-50 boards) | 10 boards |
|---|---|
| Setup | $8.18 |
| Stencil | $1.53 |
| Solder joints: 332 a board, $0.0016 each | $5.31 |
| Feeder loading: 9 extended part numbers, $3.07 each (basic and preferred parts: none) | $27.63 |
| **Total** | **$42.65** |

The priciest parts: RP2354B $1.43, AP3156 $0.63, the Abracon crystal and inductor
$0.27 each, JST PH battery connector $0.18, TP4056 $0.17. Stock to watch for a
larger run: RP2354B (1454 at JLC when checked) and AP3156 (1490).

The same board elsewhere, from each maker's instant quote: PCBWay is $65.09 for the
bare boards and $165.33 for assembly labour, parts extra after their review. The
quotes page tracks both makers.

Not placed by JLC (no LCSC number): the panel's landing J3 (JLC's paste leaves its
pads tinned, which is what the hand joint wants), the 16 membrane contacts (bare
gold), the speaker pads, test pads, locating holes and fiducials.

## You: bought separately, fitted before sale

| Item | Per unit | For 10 | Must match | Source | Per unit cost |
|---|---|---|---|---|---|
| QD3525 3.5in IPS panel, bare | 1 | 11-12 (spares) | ST7796U, 320x480, no touch, 55.50 x 84.96 x 2.50 mm; a 48-contact 0.8 mm **solder** tail, not a board module or a ZIF tail; backlight anode on pin 42, cathodes on 43-48 (LCDwiki's QD3525 pinout) | marketplace, not chosen yet | estimate $5-8 |
| Li-ion cell 503450, 1000 mAh | 1 | 12 (spares) | 5 x 34 x 50 mm, protection board, JST PH 2.0 plug with red to J2's `+`; a UN38.3 test summary to ship units to customers | AliExpress, MYWAY Digital Store: 10 for $19.33 + $1.95 shipping | about $2.13 |
| Speaker, 20 mm round, 8 ohm, 1 W | 1 | 10 | 4.5 mm thick or less, wire leads | marketplace, not chosen yet | estimate $1-2 |
| DS Lite silicone pads (NDSL): D-pad and ABXY pieces | 2 sets | 20 sets | "For NDSL"; the Start/Select strip isn't used | AliExpress, HM Trading Store: 20 sets $2.45 + $5.84 shipping | $0.83 |
| DS Lite button sets: cross and A/B/X/Y | 2 sets | 20 sets | DS Lite; the L/R and other pieces are spares | AliExpress, China Game Store: $0.88 a set + $12.23 shipping | $2.98 |
| Thin double-sided tape for the panel, adhesive foam pad for the cell | | 1 roll, 1 sheet | | marketplace | estimate about $0.50 |
| Case: tub and lid, printed at home | 1 | 10 | PETG or PLA, 47 cm3 (about 59 g) of filament | at $20-30 a kg | about $1.20-1.80 |
| **Total** | | | | | **about $14-18** |

Gone from the 4-layer build's list: the cover lens, the six screws, the light pipes
and the printed power-button cap.

Hand work per unit: [assembly.md](assembly.md).

## Per unit, into your hands

About **$29-33** at 10 units: the factory's $15 and your $14-18. Not in it: import
duties (at the 25% rate an earlier review suggested, unverified, about $3 a unit)
and JLC's new-account coupons ($30 and $20 on the quote page, up to $5 a unit off if
they apply). The spread is the panel and speaker, the last estimates.
