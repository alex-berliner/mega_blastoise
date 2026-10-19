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
| Bare boards: 2 layers, 70.5 x 151.34 mm (as quoted), 1.6 mm, ENIG, every hole >= 0.3 mm | $33.40 | $3.34 | quote (`tools/quotes/`) |
| Parts: 49 part numbers, 117 placements a board | $47.70 | $4.77 | JLC parts database |
| Assembly fees, Economic PCBA (below) | $53.29 | $5.33 | JLC's published rates |
| USB-C through-hole tabs by hand, if JLC charges it | $4.24 | $0.42 | JLC's published rates |
| Shipping, DHL Express with duties paid at checkout (DDP) | $29.45 | $2.95 | quote; the duty itself isn't known yet |
| **Total** | **about $168** | **about $16.81** | |

| Assembly fee (Economic PCBA, 2-50 boards) | 10 boards |
|---|---|
| Setup | $8.18 |
| Stencil | $1.53 |
| Solder joints: about 421 a board (JLC's count was 386 before the 9 parts added 2026-10-05; the backlight sinks that replaced the AP3156 on 2026-10-07 add about 15), $0.0016 each | $6.74 |
| Feeder loading: 12 extended part numbers (the thermistor, the 27 ohm USB resistors and, since ProTek's Preferred SRV05-4 ran out on 2026-10-07, the Semtech SRV05-4.TCT ESD array are three; the AP3156 went the same day, and the backlight sinks that replaced it are all Basic or Preferred), $3.07 each (basic and preferred parts: none) | $36.84 |
| **Total** | **$53.29** |

The priciest parts: RP2354B $1.43, the panel's connector (Hirose FH12-50S) $0.81,
the Abracon crystal and inductor $0.27 each, the backlight op-amp (MCP6002) $0.24, PicoBlade
battery connector $0.13, TP4056 $0.17. Stock to watch for a larger run: RP2354B (1665 at
JLC on 2026-10-07) and the Abracon inductor (757); the AP3156 ran out (0 on 2026-10-07)
and was replaced by the discrete backlight sinks.

The same board elsewhere, from each maker's instant quote: PCBWay is $65.09 for the
bare boards and $165.33 for assembly labour, parts extra after their review. The
quotes page tracks both makers.

Not placed by JLC (no LCSC number): the 20 membrane contacts (bare gold), the
speaker pads, test pads, locating holes and fiducials. The panel's ZIF connector
(J3) is placed by JLC like any other part.

## You: bought separately, fitted before sale

| Item | Per unit | For 10 | Must match | Source | Per unit cost |
|---|---|---|---|---|---|
| EastRising ER-TFT035IPS-6 3.5in IPS panel | 1 | 12 (2 spares) | the "No Touch" option; ILI9488, 320x480, 54.50 x 83.00 x 2.30 mm, a 50-contact 0.5 mm FPC for a ZIF connector (J3 on the board; a connector bundled with the panel isn't needed) | AliExpress, buydisplay.com Store (the maker's own): $11.24 each, 5% off at 10 or more, $13.89 shipping for 12, so 12 for $142.03. The maker's own site: $9.10 each but $45.48 shipping, 12 for $154.68 | $14.20 |
| Li-ion cell 403450, about 700 mAh | 1 | 12 (spares) | 4 x 34 x 50 mm (with its protection board), protection board, Molex PicoBlade (MX1.25, often listed as "JST 1.25") plug with red to J2's `+`: order the cells with that plug; a UN38.3 test summary to ship units to customers | AliExpress, MYWAY Digital Store: 10 for $19.33 + $1.95 shipping | about $2.13 |
| Speaker, 15 mm round, 8 ohm, 0.5-1 W | 1 | 10 | 4 mm thick or less, wire leads | marketplace, not chosen yet | estimate $1-2 |
| DS Lite silicone pads (NDSL): D-pad and ABXY pieces | 2 sets | 20 sets | "For NDSL"; the Start/Select strip isn't used | AliExpress, HM Trading Store: 20 sets $2.45 + $5.84 shipping | $0.83 |
| DS Lite button sets: cross and A/B/X/Y | 2 sets | 20 sets | DS Lite; the L/R and other pieces are spares | AliExpress, China Game Store: $0.88 a set + $12.23 shipping | $2.98 |
| Screws: M2 x 8 self-tapping, wafer head (4 mm, about 1 mm thick), for plastic | 6 | 60 (spares) | | marketplace | estimate about $0.10 |
| Thin double-sided tape for the panel, adhesive foam pad for the cell | | 1 roll, 1 sheet | | marketplace | estimate about $0.50 |
| Case: tub and lid, printed at home | 1 | 10 | PETG or PLA, 47 cm3 (about 59 g) of filament | at $20-30 a kg | about $1.20-1.80 |
| **Total** | | | | | **about $23-25** |

Gone from the 4-layer build's list: the cover lens, the six screws, the light pipes
and the printed power-button cap.

Hand work per unit: [assembly.md](assembly.md).

## Per unit, into your hands

About **$40-42** at 10 units: the factory's $16.81 and your $23-25. Not in it: import
duties (at the 25% rate an earlier review suggested, unverified, about $3 a unit
on the board order)
and JLC's new-account coupons ($30 and $20 on the quote page, up to $5 a unit off if
they apply). The spread is the speaker, tape and filament, the last estimates.

The panel is the largest single item. At 50 panels the maker's own site is the
cheapest source: $493.10 with shipping, $9.86 a panel (quotes from both shops'
carts, 2026-09-24, shipping to the US by their standard methods, duties not
included).
