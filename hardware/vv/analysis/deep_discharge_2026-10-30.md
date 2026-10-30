# Deep discharge fact check (REQ-BAT-10)

Repo state read: main at ceb8e0b plus tag vv-rp2354b-final (c8ac9dd). Local datasheets: hardware/ref_design/datasheets/.
Downloaded datasheets (kept for re-reading): scratchpad dl_dw01/szlcsc.pdf (DW01x), dl_fs312/futura.pdf and fs312_generic.pdf (FS312), dl_core/core.pdf (a DW01+8205A PCM sheet), dl_cell/ja.pdf (a J&A 803450 pack spec), dl_ocv/slyt402.pdf (TI LiCoO2 OCV curve).

| Claim | Verdict | One-line correction |
|---|---|---|
| 1. Off drain about 25-30 uA; charger ship mode | PARTLY | Budget sums correctly (26.8 uA at 4.2 V, 25.3 uA at 3.5 V) but it is a near-max budget; datasheet-typical is about 15 uA. The BQ24073 has NO ship mode or battery-FET-off input; only the BQ24075/BQ24079 have SYSOFF. |
| 2. DW01-class UVP is 2.4-2.5 V, not 3.0 V | PARTLY | TRUE for DW01A (2.40 +-0.10 V). FALSE for FS312F-G: 2.90 +-0.08 V, close to the register's 3.0 V. The register's real error is the capacity share below the cutoff, not the trip voltage. |
| 3. Charger releases the PCM and precharges | PARTLY | Release and precharge exist, but the DW01A releases only when the cell itself reaches 3.0 V, and the PCM's body-diode drop can make the BQ24073 leave precharge early. A cell that sinks below about 1.5 V cannot be charged through a DW01A at all. |

Months from the 3.50 V cutoff to the PCM trip at 25 C: **about 5 months nominal (typical drain), 0.9 to 2.6 months worst case**. The details are in section 4.

---

## 1. Off-state drain and ship mode: PARTLY

### The budget adds up
`python3 hardware/vv/tools/paper_checks.py shelf` (evidence hardware/vv/evidence/2026-10-28_VER-103_shelf.txt) itemises:

| Part | uA used | Datasheet figure | Kind |
|---|---|---|---|
| U2 BQ24073 IBAT(PDWN) | 6.5 | 4.3 typ, 6.5 max at TJ 85 C (bq24074.txt:714, SLUS810N p12) | max, at 85 C |
| U3 LDO shutdown | 1.0 | TLV757P 0.1 typ, 1 max (tlv757p.txt:265) | max |
| R17 into U10's unpowered input | 7.9 at 4.2 V, 6.4 at 3.5 V | (VBAT - 0.5 V clamp) / 470k; the clamp voltage is an estimate | estimate |
| U7 FM8002A shutdown | 8.4 | 4.2 typ, no max printed (fm8002a.txt:123) | 2 x typ stand-in |
| Q3 2N7002 off | 1.0 | 80 nA at VDS 60 V (2n7002_cj.txt:57) | stand-in, 12x the sheet |
| Q5, Q6 AO3400A off | 2.0 | 1 at VDS 30 V, 5 at TJ 55 C (ao3400a.txt:69-71, the unit is garbled in the text extraction) | stand-in |
| **Total** | **26.8 at 4.2 V, 25.3 at 3.5 V** | | |

Re-added by hand: 6.5 + 1.0 + 7.9 + 8.4 + 1.0 + 2.0 = 26.8 uA. I checked design.py at the tag for anything else on VBAT, VSYS or AMP_VDD while off: C2, C3, C5, C34, C35 (MLCC, nA leakage), and SW21/SW23 (open). Nothing is missing.

Datasheet-typical total at 3.5 V: 4.3 + 0.1 + 6.4 + 4.2 + about 0.3 = **about 15 uA**. So "designed at 25-30 uA" is the conservative budget, not the expected value.

Register inconsistency (small): REQ-PWR-07's rationale names U3 as C47773, which is the RT9013-33GB at tag vv-rp2354b-final (design.py U3; ISHDN 0.7 typ / 1.5 max, rt9013.txt:164). paper_checks.py uses the TLV757P's 1 uA, and main's design.py has TLV75733PDBVR (C485517). DEF-69 says "U3 1.5". The difference is 0.5 uA.

"Typical devices keep off-state drain tiny" is only half right. TI's own sleep-mode example (SLUS810N p31, 9.4.1) uses 10 uA and says the cell's self-discharge is "typically five times higher". Products that need months on the shelf use a ship mode, which brings the drain under 1 uA.

### Ship mode: the BQ24073 has none
- SLUS810N p6, Device Comparison Table: the BQ24073's optional function is **TD**. **SYSOFF** exists only on the BQ24075, '76, '78 and '79.
- SLUS810N p8, pin table: "Connect SYSOFF high to turn off the FET connecting the battery to the system output. When an adapter is connected, charging is also disabled." It is internally pulled up to VBAT through about 5 Mohm.
- SLUS810N p19: with no input "The Q2 FET that connects BAT to OUT is ON. (If SYSOFF is high, Q2 is off)". On the '73, VSYS is always live from the cell. CE high only stops charging (p2 change log: "OUT is active and battery supplement mode is still available").

What a SYSOFF part would save: everything on VSYS (U3 1.0, U7 8.4, Q3 1.0, Q5/Q6 2.0) = **12.4 uA of the 25.3**. IBAT(PDWN) and R17 sit on VBAT and would remain (about 13-14 uA). The swap is not drop-in:
- The BQ24075's OUT regulates at 5.5 V, not 4.4 V (p6).
- The '75 loses TD.
- Start's power-on contact is on VSYS, which would be dead in ship mode, so the turn-on path needs redesign.

R17 (7.9 uA) is the single largest term and is not a charger question.

## 2. PCM over-discharge trip and release: PARTLY

### DW01A (Fortune Semiconductor, DW01x-DS-17 Rev 1.7, May 2014)
- p6, Product Name List: DW01A-G VODP **2.40 +-0.100 V**, VODR **3.0 +-0.100 V**, 0 V charge NO, standby release "AUTO recovery".
- p9, DW01A-G table:
  - ICC 3.0 typ / 6.0 max uA at VCC 3.9 V. This matches the register's 3-6 uA.
  - **IOD (after over-discharge) 1.5 typ / 3.5 max uA at VCC 2.0 V.** The DW01A does NOT power down, so it keeps draining the cell after it trips.
  - TOD 40 typ / 100 max ms.
  - **0 V battery charge inhibition voltage V0INH max 1.5 V ("0V Battery Charging Unavailable").**
- p14, Description of Operation:
  - "discharging is inhibited by turning off the discharge control MOSFET ... immediately released when the voltage of the battery cell becomes higher than overdischarge release voltage (VODR) through charging."
  - "charging is still permitted through the parasitic diode of MOSFET."
  - For the DW01A-G: "The battery voltage rising to the overdischarge release voltage (VODR) or higher is the only required condition for the IC to return to the normal state."
  - The power-down variants (DW01+G, DW01-G, DW01B-G) drop to 0.1 uA instead.
- A shipped DW01 + 8205A PCM (Core Electronics CE06795 sheet, model RD-1306, p1) specifies over-discharge at **2.50 +-0.075 V** (measured 2.503), IDD 6.0 uA max (measured 2.2).

### FS312F-G (Fortune, FS312 TD-0404004 Rev 1.4)
- p2, Product Name List row FR (the FS312F-G): VOCU 4.250 V, VODL **2.9 +-0.08 V**, VODR **3.0 +-0.08 V**, VOI1 150 mV. The older generic FS312 sheet lists the same FR row.
- p5: IDD 3.0 typ / 6.0 max uA at 3.9 V; **IPD 0.3 typ / 0.6 max uA at 2.0 V**; TOD 5 / 17 / 30 ms; charger detection VCH -1.0 / -1.35 / -1.7 V.
- p7, sections 4 to 6: after it trips, the FS312 enters power-down. A charger releases power-down. The discharge FET turns on when VDD > VODR, or, with a charger detected (VCSI < VCH), as soon as VDD > VODL.

### Other packs
Real 34 x 50 mm packs vary. The J&A JA-803450P spec (Mar 2013) p7 and p9 use a Ricoh R5400-class PCM at **2.75 +-0.062 V** and 2.0-6.0 uA. Battery University BU-808a: "the protection circuit of a Li-ion cuts off between 2.2 and 2.9V/cell".

### Our cell's PCM: unknown
The register names no PCM. bom.md:50 and VER-37 source the cell from the AliExpress MYWAY Digital Store, not LCSC/JLC. VER-37 (the spec sheet) is NOT_RUN. VER-88 (the bench UVP test) allows 2.4-3.0 V. I found no LCSC-listed 403450 with a named PCM, so I cannot say which PCM those cells "typically" carry. **Correction: say "2.3 to 3.0 V depending on the PCM (DW01A 2.4, FS312F-G 2.9)" until VER-37 or VER-88 reads it.**

### Why the trip voltage matters less than claimed
On a LiCoO2 cell's resting curve, almost no charge lies between 3.0 V and 2.4 V (section 3). Whether the PCM trips at 2.4 or 2.9 V changes the months below by only a few percent. The register's error was the **5-10% capacity share** below the cutoff, not the 3.0 V.

## 3. Recharge after a trip, damage thresholds, capacity, self-discharge: PARTLY

### The BQ24073's precharge (SLUS810N, local bq24074.txt)
- p13: VLOWV (pre-charge to fast-charge) **2.9 / 3.0 / 3.1 V**.
- p13: IPRECHG = KPRECHG / RISET, KPRECHG **70 / 88 / 106 A-ohm**. With R5 = 3.3k 1%: **26.7 mA nominal, 21.0 to 32.4 mA** (0.038 C on 700 mAh).
- p13: tPRECHG **1440 / 1800 / 2160 s** with TMR floating, which design.py has (pin 14 None).
- p13 and p24: if BAT < VBAT(SC) 1.6 / 1.8 / 2.0 V, the charger first sources IBAT(SC) 4 / 7.5 / 11 mA until BAT rises above it.
- p27, 9.3.5.4: a battery-detection step that "indicates that the battery is missing or the protector is open. Next, the precharge current is applied for tDET to close the protector if possible." TI designed for tripped packs.
- p28: if the precharge timer expires before VLOWV, the charger faults (CHG flashes at 2 Hz). The fault clears only by toggling CE or input power.
- Precharge delivers 8.4 to 19.4 mAh before the timer ends. The cell maker J&A (spec p12, 2.3) asks for low-current precharge until the cell passes 3 V within 16-30 min, else stop.

### Corrections to "plugging in releases the PCM and the charger precharges"
1. **DW01A:** plugging in does not by itself release the discharge FET. Charge flows through the FET's body diode, and the PCM releases only when the **cell** reaches VODR 3.0 V. The FS312F-G can release at VODL (2.9 V) once it detects a charger. Either way charging works, so the user-visible outcome holds.
2. **The precharge can be skipped** (my inference from the two datasheets; measure it at VER-266). While the PCM is tripped, the BQ24073's BAT pin sees the cell plus the body-diode drop (about 0.6-0.8 V). The charger therefore leaves precharge when the cell is only about 2.2-2.4 V. It then fast-charges at 270 mA (0.39 C) through the diode until the cell reaches 3.0 V. That is a few mAh, a few minutes, and about 0.2 W in the 8205A. It is not harmful at this depth, but it is not the "trickle first" behaviour claimed.
3. **Hard floor:** a DW01A refuses all charging below V0INH (at most 1.5 V, p9). A cell that sinks that far cannot be recharged through its pack. After its trip the DW01A still draws 1.5-3.5 uA (it does not power down), plus the cell's self-discharge. How long a cell takes to fall from 2.4 V to about 1.5 V is **not sourced**: the charge below 3.0 V is a fraction of a percent, so it could be weeks to months. This is the real limit on "still charges", and VER-266's 2-month soak is the right test.

### When real damage starts
- Guo, Lu, Ouyang, Feng, *Sci. Rep.* 6, 30248 (2016), 25 Ah NCM/graphite:
  - Cu collector dissolution at the anode potential's "approximately 3.4~3.5 V", at about **-12% SOC**. That means past 0 V, driven by forced over-discharge.
  - "Neither ISCr nor capacity fading occurs if the overdischarge is terminated before -12% SOC". Such cells were recharged "with only minor side effects". The paper does not cover swelling.
- Fear, Juarez-Robles, Jeevarajan, Mukherjee, *J. Electrochem. Soc.* 165 (9) A1639 (2018): copper dissolved once cells reached their minimum, negative voltage, with the anode near 4.8 V vs Li/Li+.
- Battery University BU-808a (updated 3-Nov-2021): do not boost cells "that have dwelled below 1.5V/cell for a week or longer. Copper shunts may have formed".
- **Summary:** at the PCM trip (2.3-3.0 V) there is no copper dissolution and no measurable capacity loss. Risk starts when the resting voltage sinks to about **1.5 V and stays there for about a week**. Forced over-discharge to 0 V and below causes real damage. I found no primary threshold for swelling at shallow over-discharge.

### Capacity between 3.50 V under ~100 mA and the PCM trip
- TI SLYT402 (Analog Applications Journal 1Q 2011, p1, Fig. 1), LiCoO2 OCV vs DOD, read from the plot:
  - 3.52 V at about 82% DOD; 3.47 V at about 89%; 3.35 V at about 95%; 3.18 V at about 98%; about 3.05 V at 100%.
  - Below about 3.05 V there is essentially no charge left, so 3.0 V, 2.9 V and 2.4 V trips are equivalent within 1-2%.
- At the cutoff the load is 100 mA, and the cell's resistance is about 0.2-0.5 ohm (estimate; the J&A pack is at most 150 mohm AC). The resting voltage is therefore about **3.52-3.55 V**, which is **about 15-20% left (105-140 mAh)** on TI's curve.
- That is one curve from an older LCO cell, and the slope there is only about 7 mV per % of capacity. A 50 mV difference in a modern cell's curve moves the answer by about 7%.
- **Correction: use about 15% (105 mAh) nominal and keep 5% (35 mAh) as the worst case** until VER-101/VER-153 measure the real cell.

### Self-discharge at low charge, 25 C
- Battery University BU-802b (updated 2-Nov-2021):
  - "5 percent in the first 24 hours and then loses 1-2 percent per month".
  - "Self-discharge increases with rising temperature and higher SoC".
  - It also says "the protection circuit adds another 3 percent per month". That does not fit a DW01: 3-6 uA is 2.2-4.4 mAh, or 0.3-0.6% of 700 mAh a month.
- Cell spec limits are looser and are taken from full charge: Akyga LP502248 more than 90% left after 30 days at 25 C; J&A 803450P at least 85% after 28 days (spec p5).
- **Realistic at low charge and 25 C: about 1% a month (7 mAh), 2% (14 mAh) as the worst case.** The register's 3% is conservative.

## 4. Arithmetic: months from the 3.50 V cutoff to the PCM trip, 25 C

Monthly loss = (board + PCM) uA x 730 h + self-discharge x 700 mAh. After the trip the board is disconnected by the PCM.

| Case | Board | PCM | Self | mAh/month | 105 mAh (15%) | 70 mAh (10%) | 35 mAh (5%) |
|---|---|---|---|---|---|---|---|
| Nominal (datasheet typical) | 15.3 uA | 3 uA | 1% | 20.4 | **5.2 mo** | 3.4 | 1.7 |
| Register budget | 25.3 uA | 3 uA | 1% | 27.7 | 3.8 | 2.5 | 1.3 |
| Worst (REQ-PWR-07 limit) | 30 uA | 6 uA | 2% | 40.3 | 2.6 | 1.7 | **0.9 mo** |
| Worst, register's 3% | 30 uA | 6 uA | 3% | 47.3 | 2.2 | 1.5 | 0.7 |

- Nominal (about 15% left, typical drain): **about 5 months**.
- Worst drain with the curve read at face value (15%): **about 2.6 months**. With the 5% worst capacity: **about 0.9 months**.
- After the trip, a DW01A keeps drawing 1.5-3.5 uA plus self-discharge. A DW01A cell that sinks below about 1.5 V becomes unchargeable; an FS312F-G drops to 0.3 uA instead.

Reproduce:
```
python3 -c "m=lambda b,p,s:(b+p)*0.73+s*700; print(105/m(15.3,3,.01), 105/m(30,6,.02), 35/m(30,6,.02))"
```

## Sources
- TI SLUS810N, BQ2407x datasheet (Oct 2021): hardware/ref_design/datasheets/bq24074.txt, pages as cited.
- Fortune Semiconductor DW01x-DS-17 Rev 1.7 (May 2014): https://atta.szlcsc.com/upload/public/pdf/source/20160616/1466072038371.pdf
- Fortune Semiconductor FS312 TD-0404004 Rev 1.4: https://forums.futura-sciences.com/attachments/electronique/447863d1634839538-led-12-volts-alimentee-usb-5-v-decharge-5-v-fs312-fortunesemiconductor-1-.pdf (older generic sheet: https://matthewbaggett.github.io/Tamagotchya/References/FS312.pdf)
- DW01 + 8205A PCM sheet RD-1306: https://core-electronics.com.au/attachments/CE06795%20-%20PCM%20Datasheet.pdf
- J&A JA-803450P spec: https://www.olimex.com/Products/Power/BATTERY-LIPO1400mAh/resources/JA803450-Spec-Data-Sheet--J-A.pdf
- Akyga LP502248 spec: https://asset.conrad.com/media10/add/160267/c1/-/gl/003385835DS00/karta-katalogowa-3385835-pakiet-akumulatorow-akumulator-specjalny-lipo-akyga-lp502248-37-v-450-mah.pdf
- TI SLYT402: https://www.ti.com/lit/pdf/slyt402
- Guo et al. 2016: https://pmc.ncbi.nlm.nih.gov/articles/PMC4957210/
- Fear et al. 2018: https://iopscience.iop.org/article/10.1149/2.0671809jes/meta
- Battery University BU-808a: https://batteryuniversity.com/article/bu-808a-how-to-awaken-a-sleeping-li-ion
- Battery University BU-802b: https://batteryuniversity.com/article/bu-802b-what-does-elevated-self-discharge-do

Last updated: 2026-10-29 19:12 EDT
