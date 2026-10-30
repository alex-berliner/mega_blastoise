# REQ-PWR-14 derating rule: what the sources say and what the RP2350B board needs (2026-10-30)

Why: REQ-PWR-14's numbers (0.3 V under absolute maximum for IC transients; 80% and 90% under 10 us for passives and
discretes) had no source: Claude made them up. Alex asked for a rule backed by reasons and evidence. This file reads
the recognised documents, tries three rules and the current one against the board's real parts, and proposes a
Statement. Nothing in requirements.md or any other register file is edited here: Alex approves the wording first.

Files read are in the scratchpad `derating_src/` (not committed; each was run through `pdftotext -layout` and no code
from them was run). Datasheet lines are in `hardware/ref_design/datasheets/`, cited by file and PDF page.

## 1. Answer first

1. A fixed 0.3 V and a flat percentage of absolute maximum both fail on this board's low-voltage rails. NASA says so
   itself: for devices under 5 V, "use manufacturer's recommended operating conditions" (EEE-INST-002, Section M3,
   Table 4, Note 1.1).
2. The standards do not ask for a margin under absolute maximum on a one-off transient. ECSS says only that transients
   must stay "below the steady-state specified maximum ratings" (5.4.2b), and that a transient which repeats is
   steady state (5.4.2c). What they do ask for: steady-state inside the recommended range, repeated transients treated
   as steady state, one-off transients under the absolute maximum.
3. What is left for a margin under absolute maximum is measurement uncertainty. That one is Claude's judgment, with
   an engineering reason (section 6), not a source. The proposal makes it "plus the accuracy of the check that
   measured or simulated it" and sets no fixed volts.
4. One real finding: the panel's controller (ILI9488) states VCI and IOVCC absolute maximum 3.3 V (Table 41). The
   module datasheet says 4.6 V. The board's 3V3 stress on the panel is 3.333 V (REQ-PWR-14 Notes, DF-227). Under the
   proposal that is a fail against the controller's own number. Alex decides (section 5, exceptions).

## 2. Sources

Everything quoted below was read from a file I downloaded or from a datasheet on disk. Page numbers are the document's
printed pages unless "PDF p." is given.

| Document | Revision | Clause or table | Quoted rule | URL |
|---|---|---|---|---|
| ECSS-Q-ST-30-11C, Derating - EEE components | Rev.1, 4 October 2011 (see "not obtained" for Rev.2) | 5.1, p.16 | "The term derating refers to the intentional reduction of electrical, thermal and mechanical stresses on components to levels below their specified rating." | https://escies.org/download/webDocumentFile?id=5558 |
| same | same | 5.4.2b, p.19 | "If ratings are not provided for transient or surge conditions, then it shall be assured that the transient or surge values are below the steady-state specified maximum ratings." (the standard's text is "iIf", a typo) | same |
| same | same | 5.4.2a, p.19 | "...if ratings are provided for transients or surge conditions, the same derating figures as for steady-state equivalent parameters shall be used." | same |
| same | same | 5.4.2c, p.19 | "For all periodic signals or transient conditions which are repeated or made incessant, the steady-state derating figures shall apply." | same |
| same | same | 5.4.2e, p.19 | For repeated, incessant transients where 5.4.2c is not compatible, for ceramic capacitors (voltage), resistors, diodes (current): "load ratio shall not exceed the steady state derated values +10 % or 80 % of the steady state rated values, which ever is lower" | same |
| same | same | 5.3.2g, p.18 | "Derating requirements are not applicable to fault conditions, for which the maximum rating shall not be exceeded..." | same |
| same | same | 5.2, p.16 | stress and strength are distributions; derating "provides additional design margins" and "a safety margin for design" (no number) | same |
| same | same | Table 6-19 (logic ICs), p.40 | Supply voltage: "Manufacturer recommended value +-5 % or 90 % of maximum rating." Special condition: "Turn on transient peaks or other peaks shall not exceed the maximum rating." | same |
| same | same | Table 6-21 (linear ICs), p.42 | Supply voltage "90 % of the maximum rated value", "Supply voltage shall include DC + AC ripple."; Transients: "Shall not exceed the specified maximum ratings." | same |
| same | same | Table 6-1 (ceramic capacitors), p.22 | Voltage: a graph, read from the figure: 60% of rated up to 85 C case temperature, falling to 40% at 110 C. | same |
| same | same | Table 6-34 (FETs), p.54 | VDS "80 % of rated"; VGS "75% of rated" (or the SEE safe operating area, space only) | same |
| same | same | Table 6-13 (diodes), p.34 | Forward current (IF) 75%; reverse voltage (VR) 75 % | same |
| same | same | Table 6-29 (chip resistors), p.51 | Voltage 80 %; "rms Power 50 % up to 85 C, decreasing to 0 % at 125 C" (case temperature) | same |
| NASA GSFC EEE-INST-002, Instructions for EEE Parts Selection, Screening, Qualification, and Derating | April 2008, Addendum 1 incorporated (page footers 5/03) | Section 1, 6.5, p.7 of 12 | "Derating shall be performed by the designer in accordance with the requirements set in Table 4 for each category of parts... These derating factors do not preclude further more stringent derating" | https://abbottaerospace.com/wp-content/uploads/2016/05/NASA-EEE-INST-002-Instructions-for-EEE-Parts-Selection-Screening-Qualification-and-Derating.pdf (a copy of the NASA document; NASA's own is at nepp.nasa.gov) |
| same | same | Section M3, Table 4 and Notes 1, 1.1, p.14 of 15 | Maximum supply voltage factor 0.9 digital, 0.8 linear. Note 1: "Use manufacturer's recommended operating conditions but do not exceed 90% of maximum supply voltage for digital devices and 80% of maximum supply voltage for linear devices." Note 1.1: "For low voltage (< 5V) devices, use manufacturer's recommended operating conditions." | same |
| same | same | Section M4, Table 4 (plastic encapsulated microcircuits), p.10 of 10 | Maximum supply voltage: digital "Vn.r.+0.5*(Vmax.r.-Vn.r.)", linear "Vn.r.+0.8*(Vmax.r.-Vn.r.)"; "Vn.r. is the nominal rated power supply voltage; Vmax.r. is the maximum rated power supply voltage." | same |
| same | same | Section C1, Table 4, p.22 of 23 | Ceramic: 0.60 at 110 C. Note 1: "The derating factor applies to the sum of peak AC ripple and DC polarizing voltage." | same |
| same | same | Section S1, Table 4 (transistors), p.12 of 13 | Voltage 0.75; power MOSFET gate to source 0.60, source to drain 0.75. Note 1: "Worst-case combination of DC, AC, and transient voltage should be no greater than the derated limit." | same |
| same | same | Section S1, Table 4 (diodes), p.11 of 13 | General purpose, rectifier, switching, Schottky: PIV 0.70, forward current 0.50 | same |
| same | same | Section R2, Table 4, p.23 of 24 | RM (film chip): power 0.6, voltage 0.8 | same |
| TI SZZA036B, Understanding and Interpreting Standard-Logic Data Sheets | May 2003 | Absolute Maximum Ratings, p.9 | "...stress levels that, if exceeded, may cause permanent damage to the device. However, these are stress ratings only, and functional operation of the device at these or any other conditions beyond those indicated under recommended operating conditions is not implied. Also, exposure to absolute-maximum-rated conditions for extended periods may affect device reliability." | https://e2e.ti.com/cfs-file/__key/communityserver-discussions-components-files/151/interpretingLogic_5F00_szza036b_5F00_.pdf |
| same | same | Supply Voltage, p.27 | "no data-sheet parameters are ensured when a device is operated at the absolute maximum VCC level." | same |
| TI SLUS810N (BQ24073), TI SBVS211B (TPS3421), TI SN74HC165 | on disk | section 8.1 note 1; 6.1 note 1 | the same boilerplate, in the board's own parts: "Exposure to absolute-maximum-rated conditions for extended periods may affect device reliability." | hardware/ref_design/datasheets/bq24074.pdf p.10, tps3421.pdf p.4 |
| Raspberry Pi RP2350 datasheet | Release 8, build 01/10/2026 | 14.9.1, Table 1680, PDF p.1336 | "Stresses beyond the absolute maximum ratings listed in the following table can cause permanent damage to the device." DVDD 1.21 V, IOVDD 3.63 V. | hardware/ref_design/datasheets/rp2350.pdf |
| same | same | Table 1688, note b, PDF p.1341 | DVDD "Short term transients should be within +/-100mV." (the datasheet's own transient allowance) | same |
| ILI9488 datasheet | on disk | 17.1, Table 41 and its note, PDF p.322 | VCI and IOVCC "-0.3 ~ +3.3" V. Note: "Even if the absolute maximum rating of one of the above parameters is exceeded only for a short while, the quality of the product may be degraded." | hardware/ref_design/datasheets/ili9488.pdf |
| Murata, Ceramic Capacitor Caution page | web page read 2026-10-30 | consumer-equipment MLCC note | "Consumer equipment specific MLCC products are designed so that the estimated useful life can be extended longer than 5 years under the following conditions: '80% of the rated voltage or less, Maximum operating temperature -20°C or less'." It lists "IPC-9592B class1 equipment" among the consumer uses. "*The estimated useful life data is provided for reference only." | https://www.murata.com/en-global/products/capacitor/ceramiccapacitor/help/caution |
| Samsung Electro-Mechanics CL10A106KP8NNN, CL10A106MA8NRN, CL10A475KO8NNN DC-bias curves | on disk (csv beside the datasheets) | Delta C against DC bias, 25 C, 1 kHz | used for the DC-bias column of section 4 | datasheets/samsung_cl10a*_dcbias.csv |

Table numbers: the task's notes cite RP2350 "Table 1433 p.1338" and "Table 1441 p.1343". The copy on disk
(`rp2350.pdf`, Release 8, build 01/10/2026) has absolute maximum ratings as Table 1680 on PDF p.1336 and the power
supply table as Table 1688 on PDF p.1341. The values agree (DVDD 1.05 to 1.16 V, absolute 1.21 V; IOVDD recommended
maximum 3.63 V, absolute 3.63 V); I cite what I read.

### Not obtained, and what it leaves uncertain

- ECSS-Q-ST-30-11C Rev.2 (23 June 2021): ecss.nl returned an HTML "Site unavailable" page to my downloads. I read
  Rev.1 (4 October 2011) from escies.org. The Rev.2 front matter, as listed by the ECSS site, says its changes were
  change requests, a thick-film power clause and deleting the ESCC exceptions annex (from a search summary, not
  read). So the IC, capacitor, FET, diode and resistor rows I use could differ in Rev.2. Check before Alex signs.
- IPC-9592 (any revision): the standard is paid. Search results show a 2008 scan on Scribd and an APEC 2012 slide deck
  (SlideShare); both blocked my download. Search summaries say IPC-9592B moved MLCC voltage from 80% to 90%, but I could
  not read the table, so I use none of it. Only Murata's page links it to consumer equipment ("IPC-9592B class1").
- TDK, KEMET, Samsung statements on class II MLCC voltage derating: a search found none that gives a percentage;
  only Murata's. Samsung's DC-bias curves are on disk and used.
- Analog Devices' absolute-maximum FAQ (analog.com/en/resources/faqs/faq_absolute_max.html) and its MLCC DC-bias
  article: my downloads failed. I saw them only in a search result's excerpt, so I quote neither. TI's SZZA036B and the
  datasheets' own notes carry the same message.
- JEDEC definitions of absolute maximum and recommended operating conditions: not searched.

## 3. What consumer practice is

In plain words, with what each statement rests on:

1. **ICs: run inside the datasheet's recommended operating range; the absolute maximum is a stress limit, not a
   design limit.** Every datasheet on this board says the absolute maximum is a "stress rating only", that function is
   "not implied" beyond the recommended conditions, and that "exposure to absolute-maximum-rated conditions for extended
   periods may affect device reliability" (SZZA036B p.9; bq24074.pdf p.10; tps3421.pdf p.4). The RP2350 datasheet says the
   same (Table 1680). No consumer-electronics standard I could reach sets a percentage for ICs. The nearest recognised
   rule is NASA's, which itself falls back on the manufacturer for low-voltage parts: under 5 V, "use manufacturer's
   recommended operating conditions" (M3 Table 4 Note 1.1).
2. **Transients: not above absolute maximum.** ECSS 5.4.2b, Table 6-19 and Table 6-21 (space; but this is a "do not
   exceed the rating" rule, not a margin, so it is no stricter than a consumer would accept).
3. **MLCC voltage: 80% of rated** for a >5-year life at a maximum operating temperature 20 C under the part's limit,
   for consumer parts, from the maker (Murata). Space standards go to 60% (ECSS Table 6-1, NASA C1 Table 4); that is
   stricter than consumer practice.
4. **FETs, diodes, resistors: I found no consumer source.** Only the space numbers (ECSS 80% / 75%, diode 75%,
   resistor power 50% and voltage 80%; NASA a little tighter in places). Those are stricter than consumer practice;
   the board passes them (section 4), so adopting them costs nothing and has a source.

Space and military standards (ECSS, NASA) apply to flight hardware with a mission life in years and no repair. They
are stricter than a consumer handheld needs, and I use them only as the one documented set of numbers for FETs,
diodes and resistors. For ICs and the low-voltage rails the standards themselves defer to the manufacturer's
recommended range, which is also what consumer parts are built and tested to.

## 4. The board's parts against each rule

Which board: the RP2350B board as the register describes it. `design.py` in the working tree is the ESP32-S3 build
(its docstring says so); the RP2354B circuits are at tag `vv-rp2354b-final`, and the RP2350B board has no design yet
(hardware/vv/PLAN.md). Stresses below come from the register (REQ-PWR-01, -02, -04, VER-12, VER-71), from the
datasheet limits I read, and from the old board's circuits where they carry over (U3 RT9013, U2 BQ24073, backlight
sinks, buttons). They are limits and design values, not measurements: no RP2350B board exists (every check is
NOT_RUN or NOT_BUILT).

The four rules:

- (a) the current Statement: steady at or below the recommended maximum; each transient at least 0.3 V below the
  absolute maximum; passives and discretes 80%, 90% for transients under 10 us.
- (b) half-gap: each transient at least half the gap between recommended and absolute maximum below absolute
  maximum (limit = absolute minus (absolute minus recommended) / 2).
- (c) flat 90% of absolute maximum, steady and transient (90% is what both space standards use for IC supply).
  80% (the old blanket rule) is tighter and fails everything (c) fails.
- (d) the proposal in section 5: steady inside the recommended range, repeated transients as steady, and every stress
  plus the check's accuracy A at or below the absolute maximum (the maker's own transient allowance where it gives
  one; the recommended maximum standing in where the datasheet gives no absolute maximum).

"A" is the accuracy of the VER row that measures or simulates the stress. The register's rows (VER-12, VER-71) leave
the accuracy column empty, so the (d) column gives the headroom each check's A plus overshoot must fit in.

### IC pins

| Pin (datasheet, PDF p.) | Stress here | Recommended max | Absolute max | (a) current | (b) half-gap | (c) 90% | (d) proposal |
|---|---|---|---|---|---|---|---|
| RP2350 DVDD, core 1.1 V, on-chip regulator (rp2350.pdf Tables 1688, 1680, 1689) | 1.10 V; 1.133 V at the regulator's +3% (Table 1689) | 1.16 V | 1.21 V; maker's transient allowance 1.1 +- 0.1 V (note b) | FAIL: transient limit 0.91 V is under the 1.05 V minimum, no running chip can meet it | pass: limit 1.185 V (but tighter than the maker's own 1.2 V) | FAIL: limit 1.089 V is under the 1.10 V nominal | steady pass (1.133 <= 1.16); transient limit the lower of 1.2 V and 1.21 V - A; pass if A + overshoot fits |
| RP2350 IOVDD and QSPI_IOVDD, 3.3 V (same tables) | 3.40 V max (REQ-PWR-04, 3.30 V +-3%) | 3.63 V | 3.63 V (the same number) | FAIL: transient limit 3.33 V is under the 3.40 V steady maximum | pass, but the limit is 3.63 V: zero margin, allows sitting at absolute maximum | FAIL: limit 3.267 V | pass, headroom 0.23 V (3.63 - 3.40) for A plus overshoot |
| RP2350 USB_OTP_VDD, ADC_AVDD, VREG_AVDD (Table 1688) | 3.40 V | 3.63 V | none listed (Table 1680 gives DVDD, IOVDD and pin limits only) | cannot be evaluated | cannot be evaluated | cannot be evaluated | recommended maximum stands in: pass, headroom 0.23 V |
| RP2350 VREG_VIN (Table 1688) | 3.40 V (from 3V3) | 5.5 V | none listed | cannot be evaluated | cannot be evaluated | cannot be evaluated | pass, headroom 2.1 V |
| U3 RT9013 VIN (rt9013.pdf p.3) | 4.5 V steady (BQ24073 VO(REG) 4.4 V, bq24074.pdf p.6); 4.6 V transient (VER-71 limit) | 5.5 V | 6 V | pass | pass (limit 5.75 V) | pass (limit 5.4 V) | pass, headroom 1.4 V |
| U3 RT9013 EN (p.3) | 4.8 V steady (R14 divider of VBUS 5.5 V); 5.7 V hot-plug limit (VER-12) | none given | 6 V | pass at 5.7 V (limit 5.7 V, no room) | cannot be evaluated | FAIL: 5.7 V against a 5.4 V limit | pass if A <= 0.3 V |
| U2 BQ24073 IN (bq24074.pdf p.10) | 5.5 V steady (REQ-PWR-01); 6.2 V hot plug (VER-12 limit) | 6.4 V (IN operating, the '73) | 28 V | pass | pass (limit 17.2 V) | pass (limit 25.2 V) | pass, headroom 21.8 V |
| U2 EN1 (p.10) | 3.3 V steady; 6.7 V hot-plug limit (VER-12) | none given | 7 V | pass at the limit (6.7 V) | cannot be evaluated | FAIL: 6.7 V against a 6.3 V limit | pass if A <= 0.3 V |
| U2 OUT (VSYS) and BAT (p.10) | OUT 4.6 V max; BAT 4.2 V | none given for voltage | OUT 7 V; BAT 5 V | pass | cannot be evaluated | pass (OUT 6.3 V, BAT 4.5 V) | OUT headroom 2.4 V; BAT 0.8 V |
| U10 MCP6001, U6 MCP6002 VDD (mcp6001.pdf p.3) | 3.40 V | 6.0 V | 7.0 V | pass | pass (limit 6.5 V) | pass (limit 6.3 V) | pass, headroom 3.6 V |
| U7 FM8002A VDD from VSYS (fm8002a.pdf p.3; Chinese datasheet) | 4.5 V steady; 4.6 V transient | 6 V | 6 V (the same number) | pass (limit 5.7 V) | pass, but limit 6.0 V: zero margin | pass (limit 5.4 V) | pass, headroom 1.4 V |
| U8 SN74HC165 VCC (sn74hc165.pdf p.4) | 3.40 V | 6 V | 7 V | pass | pass (limit 6.5 V) | pass (limit 6.3 V) | pass, headroom 3.6 V |
| TPS3421 VCC, from 3V3 (tps3421.pdf p.4) | 3.40 V | 6.5 V | 7 V | pass | pass (limit 6.75 V) | pass (limit 6.3 V) | pass, headroom 3.6 V |
| Panel ER-TFT035IPS-6, VCI and VDDI (er-tft035ips-6.pdf p.11) | 3.333 V (3.350 V at 125 C), REQ-PWR-14 Notes, DF-227 | 3.3 V | module sheet: IOVCC 4.6 V; controller (ili9488.pdf p.322): VCI and IOVCC 3.3 V | steady FAIL (named exception); vs the module's 4.6 V transient pass; vs the controller's 3.3 V FAIL | same | module 4.14 V pass; controller 2.97 V FAIL | steady over the recommended maximum by 33 mV (exception); vs the controller's 3.3 V absolute: FAIL (3.333 + A > 3.3). Needs Alex, section 5 |
| GD25Q128 flash VCC | 3.40 V | NO DATASHEET ON DISK | NO DATASHEET ON DISK | not evaluable | not evaluable | not evaluable | not evaluable (the datasheet is not downloaded) |

### Passives and discretes

| Part class (datasheet, page) | Stress here | Rating | (a) 80%, 90% under 10 us | (b) half-gap | (c) 90% | (d) proposal |
|---|---|---|---|---|---|---|
| MLCC C1, CL10A106MA8NRNC 10 uF (rating from the LCSC listing in pin_review/passives.json; Samsung datasheet not on disk) on VBUS | 5.5 V steady, 6.2 V hot plug | 25 V | 22%, 25%: pass | not an IC rule | pass | 80% steady: pass; one-off transient under 25 V - A: pass |
| MLCC C2, CL10A475KO8NNNC 4.7 uF on VSYS | 4.5 V, 4.6 V | 16 V | 28%: pass | n/a | pass | pass |
| MLCC C3 on VBAT (4.2 V), C5 on VSYS (4.6 V), C6/C7 on 3V3 (3.4 V), CL10A106KP8NNNC 10 uF | worst C5 46% | 10 V | pass | n/a | pass | pass (80% = 8 V; ECSS and NASA's 60% = 6 V also pass) |
| MLCC 0402 on 3V3, CL05A105KA5NQNC 25 V; CL05B104KO5NNNC 16 V (21% at 3.4 V) | <= 21% | 16 to 25 V | pass | n/a | pass | pass |
| MLCC on the 1.1 V core (RPi minimal design's part numbers) | 1.133 V | not on disk | not evaluable | n/a | not evaluable | not evaluable (core capacitor part numbers and ratings are not on disk) |
| MOSFET AO3400A Q5, Q6 (ao3400a.pdf p.1): VDS 30 V, VGS +-12 V | Vds under VSYS 4.6 V (15%); Vgs under 3.4 V (28%) | 30 V; 12 V | pass | n/a | pass | Vds 80%, Vgs 75%: pass |
| MOSFET 2N7002 Q1, Q7 (2n7002_cj.pdf p.1): VDS 60 V, VGS +-20 V | Vds 3.4 V (6%); Vgs under 4.6 V (23%) | 60 V; 20 V | pass | n/a | pass | pass |
| Diodes BAT54C D2, BAT54WS D3, D8 (bat54c.pdf p.2, bat54ws.pdf p.1): VR 30 V, IF 200 mA | VR up to 6.2 V (21%); IF under 1 mA through 47 k | 30 V; 200 mA | pass | n/a | pass | VR 75%, IF 75%: pass (NASA's IF 50% also passes) |
| TVS SRV05-4 U1 (srv05-4.pdf p.2): working voltage 5 V | 3.6 V on D+/D- (72%) | 5 V | pass | n/a | pass | pass (it lists no absolute maximum voltage) |
| Resistor arrays RN (uniroyal_4d02.pdf p.6): 1/16 W, 50 V working | 3.4 V across 47 k: 0.25 mW per element (0.4%); 6.8% of 50 V | 62.5 mW; 50 V | pass | n/a | pass | power 50%, voltage 80%: pass |
| NTC TH1 (tctr0805.pdf p.2): 130 mW | under 0.3 mW (divider from 3V3) | 130 mW | pass | n/a | pass | pass |
| Other chip resistors, e.g. the 2.2 ohm backlight sense (56 mA = 6.9 mW) | 6.9 mW | NO DATASHEET ON DISK for the 0402WGF and 0603 series | not evaluable | n/a | not evaluable | not evaluable |
| Inductor L2 (AOTA 3.3 uH), crystal X1, connectors | | NO DATASHEET ON DISK | not evaluable | | | not evaluable |

DC bias is a separate effect from voltage derating. From the Samsung curves on disk, the capacitance lost at the voltage
this board applies (the voltage rating stays well inside 80%; the loss is large anyway):

| Capacitor | Rated | Voltage | Capacitance lost |
|---|---|---|---|
| CL10A106KP8NNNC 10 uF (C3, C5, C6, C7) | 10 V | 3.4 V (3V3) | 57% |
| same | 10 V | 4.4 V (VSYS) | 67% |
| CL10A106MA8NRNC 10 uF (C1) | 25 V | 5.25 V | 55% |
| CL10A475KO8NNNC 4.7 uF (C2) | 16 V | 4.4 V | 44% |

Which drives the rule: **for a 3.3 V to 5 V rail the DC-bias loss, not the voltage derating, sets the capacitor.** A
10 V part at 34% to 46% of rating still loses more than half its capacitance. The 80% voltage rule is the reliability
rule (Murata's 5-year statement); the capacitance at bias must still meet each consumer's minimum, which is a
function check (REQ-PWR-13's "at its DC bias", VER-142), not derating.

NASA's plastic-encapsulated-parts formula as a cross-check on the IC supplies (M4 Table 4: Vn.r. + factor x
(Vmax.r. - Vn.r.), factor 0.5 digital, 0.8 linear, Vn.r. the nominal supply): RP2350 DVDD digital
1.1 + 0.5 x 0.11 = 1.155 V (board 1.133 V: pass); IOVDD digital 3.3 + 0.5 x 0.33 = 3.465 V (board 3.40 V: pass, 65 mV
to spare); MCP6001 linear 3.3 + 0.8 x 3.7 = 6.26 V; SN74HC165 digital 3.3 + 0.5 x 3.7 = 5.15 V (both pass). The panel
has Vn.r. = Vmax.r. = 3.3 V so the formula leaves it nothing. This shows a "fraction of the gap" is a recognised
form, but anchored at nominal for steady-state supply, not at the recommended maximum for transients (what the
half-gap proposal did).

## 5. Proposed REQ-PWR-14

Statement (proposed):

> In every mode (off, cell, USB, charging, hot plug, unplug), each IC pin's steady-state stress, and any transient
> that repeats, shall be inside the datasheet's recommended operating range, and every stress, steady or transient,
> plus the accuracy of the check that measured or simulated it, shall be at or below the datasheet's absolute maximum
> (where the maker gives its own transient allowance, that allowance; where the datasheet gives no absolute maximum, the
> recommended maximum in its place). Each passive and discrete shall be at or under: MLCC voltage 80% of rated (DC plus
> AC peak); FET Vds 80% and Vgs 75%; diode VR 75% and IF 75%; resistor voltage 80% and power 50%; and a one-off
> transient on any of them at or below its rating less the check's accuracy. Every exception is named here.

Rationale, each number and its source:

| Element | Source |
|---|---|
| IC steady state inside the recommended range | NASA EEE-INST-002 M3 Table 4 Note 1.1 ("For low voltage (< 5V) devices, use manufacturer's recommended operating conditions") and ECSS Table 6-19 ("Manufacturer recommended value +-5 %"). Every IC rail on this board is under 5 V except the BQ24073's IN, U3's VIN and EN and the FM8002A, whose recommended maxima (6.4, 5.5, 6) the datasheets give. |
| A repeated transient counts as steady state | ECSS 5.4.2c |
| Transient at or below the absolute maximum | ECSS 5.4.2b, Table 6-19 special condition, Table 6-21 "Transients: Shall not exceed the specified maximum ratings." |
| The maker's own transient allowance where given | ECSS 5.4.2a (ratings for transients derate like steady-state ones); RP2350 Table 1688 note b: DVDD "Short term transients should be within +/-100mV." |
| Recommended maximum stands in where no absolute maximum is listed | judgment (ECSS 5.4.2b asks for "below the steady-state specified maximum ratings"; the recommended maximum is the only maximum the datasheet gives: RP2350's USB_OTP_VDD, ADC_AVDD, VREG_AVDD, VREG_VIN) |
| "plus the accuracy of the check" | judgment (section 6) |
| MLCC voltage 80% | Murata, consumer MLCC: "80% of the rated voltage or less" for a >5 year estimated life at a maximum operating temperature 20 C under the part's limit |
| One-off transient on a passive or discrete at or below its rating less the check's accuracy | ECSS 5.4.2b (below the specified maximum rating) plus the same accuracy judgment |
| FET Vds 80%, Vgs 75% | ECSS Table 6-34 (NASA is tighter: 75% Vds, 60% Vgs; both pass on this board) |
| Diode VR 75%, IF 75% | ECSS Table 6-13 (NASA: PIV 70%, IF 50%; both pass on this board) |
| Resistor voltage 80%, power 50% | ECSS Table 6-29 (power 50% up to 85 C case temperature; NASA R2: 60% and 80%) |

Dropped from the old Statement, and why: the 0.3 V (no source; fails DVDD and IOVDD), the 90% under 10 us (no source:
ECSS's rule for repeated transients on ceramics is 80% or the derated value +10%, and a one-off is "below the maximum
rating"), and the blanket 80% for FETs, diodes and resistors (the ECSS tables give 75% to 80% by class).

The Murata 80% carries a temperature condition: the part's maximum operating temperature minus 20 C. For X5R (85 C)
that is 65 C. The register's thermal rows (REQ-THM) must keep every MLCC at or under 65 C for the 80% to carry its
5-year basis; I did not read them.

Exceptions the board needs:

1. **Panel VCI and VDDI at up to 3.333 V (3.350 V at 125 C)** against a 3.3 V recommended maximum (ER-TFT035IPS-6
   p.11; DF-227; already named in REQ-PWR-14's Notes). Reason: one 3V3 rail feeds the RP2350 (recommended maximum
   3.63 V) and the panel (3.3 V); the 3V3 regulator's accuracy puts 33 to 50 mV over. **This exception is not enough
   on its own:** the ILI9488's Table 41 lists VCI and IOVCC absolute maximum 3.3 V, and its note says "Even if the
   absolute maximum rating of one of the above parameters is exceeded only for a short while, the quality of the product
   may be degraded." The module sheet says 4.6 V (p.11). The two disagree. If the controller's number is the real
   one, the panel's supply is above absolute maximum in steady state, and the fix is in the supply (a lower 3V3
   upper limit, or a panel-side drop), not in the rule. Alex's call; I did not resolve it.
2. **U10's input at VBAT/2 with VDD at 0 V** is already named: inside the MCP6001's own limits (analog inputs VSS - 1.0
   V to VDD + 1.0 V, 2 mA; mcp6001.pdf p.3; REQ-PWR-14 Notes).
3. No exception for IOVDD or the FM8002A, where recommended and absolute maximum are the same number: the "plus the
   accuracy" clause covers them (headroom 0.23 V and 1.4 V).

## 6. What is still Claude's judgment

1. **"Plus the accuracy of the check".** No source sets a margin under absolute maximum for a one-off transient. The
   reason for one: a scope or a simulation reads the stress with an uncertainty; absolute maximum is a limit on the true
   value, so the reading must be that far under it for the true value to be under it. The same reason TI gives for the
   limit itself: "exposure to absolute-maximum-rated conditions for extended periods may affect device reliability".
   The value of A is not set anywhere: the accuracy column of VER-12 and VER-71 is empty. Each check must state its
   own A (instrument and probe; for a simulation, the model's stated tolerance) before it can pass. Examples of what
   must fit: DVDD headroom 0.01 V (1.21 against the maker's 1.2 V allowance, so the allowance governs), IOVDD 0.23 V,
   U2's EN1 and U3's EN 0.3 V.
2. **The recommended maximum stands in where no absolute maximum is listed.** A reading of ECSS 5.4.2b, not its text.
3. **The choice of ECSS over NASA where they differ** (FET gate 75% not 60%, diode IF 75% not 50%, resistor power 50%
   not 60%). Both pass on this board; I took one document so every number traces to one table.
4. **Taking space standards' numbers for FETs, diodes, resistors** at all: no consumer source found.
5. **Treating a transient as "repeated" when it recurs in normal use** (each switching cycle, each button press):
   ECSS 5.4.2c says "repeated or made incessant" without a count. A hot plug or the unplug hand-over is one-off; a
   button press is repeated.
6. **Stresses in section 4** are design limits and the old board's circuits, not measurements of an RP2350B board.

## 7. Open items for Alex

1. The panel's supply against the ILI9488's 3.3 V absolute maximum (section 5, exception 1).
2. Download ECSS-Q-ST-30-11C Rev.2 (ecss.nl) and read 5.4.2, Tables 6-1, 6-13, 6-19, 6-21, 6-29, 6-34 for changes.
3. Datasheets missing from disk: GD25Q128, 0402/0603 chip resistors, L2, X1, the RPi minimal design's core capacitors.

Last updated: 2026-10-30 10:40 EDT
