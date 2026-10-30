# The power-off design against the datasheets (VER-258), 2026-10-30

Why: Alex closed the reviews of REQ-PWR-09, REQ-PWR-10, REQ-PWR-21 and REQ-SOC-04 until the power-off design is
confirmed against the parts' documents. It rested on memory. Now read.

Documents (hardware/ref_design/datasheets, downloaded 2026-10-30, rows in SOURCES.md; line numbers are
`pdftotext -layout` lines of the `.txt` beside each PDF, which are not committed):

- `tps3421.pdf`: TI SBVS211B (Rev. B, April 2015), the TPS342x family. sha256 69e92fb4...f245.
- `rp2350.pdf`: Raspberry Pi RP2350 datasheet, Release 8, build 01/10/2026. sha256 a638d678...2a39. The errata are
  its Appendix D; there is no separate errata PDF.
- LCSC C2067262 is TPS3421EGDRYR (LCSC product record fetched 2026-10-30; TI part marking AC, tps3421.txt:1224).

The design as written: seat 1's Start joins VSYS to PWR_EN through a diode; the firmware latches PWR_HOLD after
2.5 s; a TPS3421 fed from 3V3, watching Start's 3V3-side buffered line, pulls RUN low after about 8 s; a reset
releases PWR_HOLD (R42 4.7k pull-down); the unit turns off once Start is released; the firmware must not re-latch
on a press that began before it booted.

## Answers

### 1. Variants and delay: CONFIRMED

The TPS3421 delay is chosen by the variant letter and the TS pin (tps3421.txt:308-309, Table 2 at :668;
datasheet p.5 and p.11). The only TPS3421 variants are TPS3421EC (80 ms pulse, marking AB) and TPS3421EG (400 ms
pulse, marking AC) (:1220-1228). For both, tTIMER is:

| TS pin | tTIMER min / typ / max | Use |
|---|---|---|
| GND | 6 s / 7.5 s / 9 s | the hard reset |
| VCC | 0 s | factory test only; "not intended for normal operation" (:320, note 1) |

There is no 8 s option and no 10 s option (12.5 s exists only on TPS3420D, which has the wrong output, see 2).
The nearest is 7.5 s with TS tied to GND. The table is "over the operating temperature range of -40 to 125 C
and 1.6 V to 6.5 V" (:257 and :300, p.5), so the -20%/+20% (6 s to 9 s) already covers temperature and supply: inside
6 s to 10 s. The minimum, 6 s, is 2.4 times the 2.5 s turn-on hold, so the turn-on hold never resets the unit.
Two cautions: (a) the minimum sits exactly on REQ-SOC-04's 6 s lower edge, no margin; (b) TS must be wired to GND
and never switched while running (p.3, :171-175; p.11, :670-674).

### 2. Output after the delay: CHANGED (a fixed pulse, not held)

- Fixed-width pulse, not held while pressed: "For the TPS3421, RST remains low for a factory-programmed fixed time.
  For the TPS3420, RST remains low until one of the PBx inputs is released" (p.1; :166-168; p.11 Inputs, :686-690).
  C2067262 is the G variant: tRST 320 ms / 400 ms / 480 ms (:313).
- Open-drain, active low (p.1; :166), 7 V maximum on RST, pull-up to 6.5 V allowed (:703-705). VOL 0.3 V at
  3.3 V and 5 mA (:293). The RP2350 RUN pin has an internal pull-up (rp2350.txt:86605, Table 1677 "Reset State
  Pull-Up"); the datasheet gives no resistor value, so an external pull-up on RUN is optional.
- Supply 1.6 V to 6.5 V (:222-232), so 3V3 fits. Supply current 250 nA typical at 3.3 V standby, 1 uA maximum at
  6.5 V and up to 85 C (3.3 uA over the full range) (:261-266); 6 uA typical, 12 uA maximum with the timer running
  (at 6.5 V, :270-272).
- Inputs: two of them, PB1 and PB2, and BOTH must be low for the whole delay (p.3, :160-165; p.1 "when both inputs
  are low"). On the TPS3421 they are high-impedance (input current +-50 nA) with NO internal pull-up; "external
  pullup resistors to VCC are required" (:565, :687). The internal 65k pull-up is the TPS3422's only. VIH 0.7 VCC,
  VIL 0.3 VCC (:275-283), i.e. 2.31 V and 0.99 V at 3.3 V. Absolute maximum on PB 7 V, -0.3 V (:197, p.4).
- No latch, but it must re-arm: "At least one input pin must be released (go high) and then driven low for the
  tTIMER duration before RST asserts again" (:690-692; Figure 14 "No Second Pulse", :734-739). So a Start held through
  and past the pulse gives one reset only. A change at PB1/PB2 during tRST is seen only after the pulse ends
  (:740-743).
- Not in the design as written, and needed: PB1 and PB2 must both see Start (tie them together, or one to GND).
  A PB2 left open or only pulled up never fires. The 3V3-side buffer (a FET stage with its pull-up on 3V3, DF-266)
  supplies the required pull-up.

### 3. The diode-OR path when the 8 s hold fires with Start held: CHANGED

The RP2350 is NOT held in reset until release. RUN goes low for 320 ms to 480 ms, then is released whatever Start
does, and the chip reboots with Start still held and the regulator still powered through the diode-OR.
During the pulse, PWR_HOLD's pad is back at its reset state ("Output buffer is high-impedance, Input buffer is
disabled, Pulled low", rp2350.txt:36000-36004; "The pads return to the reset state" on RUN low, :36034-36038),
and R42 pulls it low as well, so the latch is open; only Start keeps the rails up. The TPS3421 does not fire
again until Start is released and pressed for a further 6 s to 9 s.

When Start is released, nothing in either datasheet is involved any more: PWR_HOLD is low, so PWR_EN falls and the
regulator turns off. How fast the rails fall below 0.5 V is a property of the regulator and the load, which
neither document gives; REQ-PWR-09's 2 s stays a design figure checked by VER-239 (paper) and VER-243, VER-240
(bench). Verdict on "the unit turns off once Start is released": CONFIRMED for the logic, UNPROVEN by these
documents for the 2 s.

### 4. Telling a held press after boot: CHANGED (the rule is needed, and it is not the one written)

The premise "the TPS3421 holds RUN low until release" is false (answer 2), so the chip does reboot with Start held
and the re-latch rule is NOT redundant: without it the unit restarts after every 8 s hold (DF-268).

The reset-reason register is POWMAN CHIP_RESET (offset 0x02c, Table 521, rp2350.txt:28822-29000, datasheet
p.427-428; sources listed in 7.3.3, :30409-30460). Read-only bits flag each source separately:

| Bit | Name | Line |
|---|---|---|
| 16 | HAD_POR (power-on reset) | :28979 |
| 17 | HAD_BOR (brown-out) | :28968 |
| 18 | HAD_RUN_LOW (RUN pin) | :28957 |
| 24, 23, 22, 28 | HAD_WATCHDOG_RESET_SWCORE, _POWMAN, _POWMAN_ASYNC, _PSM | :28888, :28900, :28912, :28833 |

The SDK's watchdog (rp2350.txt:72040) is a PSM-level reset, so a watchdog hang shows HAD_WATCHDOG_RESET_PSM, not
HAD_RUN_LOW.

The written rule has a flaw. "Latch only on a press that begins after boot" also forbids the normal turn-on: the
press that powers the regulator always begins BEFORE the firmware boots (the regulator, bootrom and image take a
few hundred ms). The rule that works, and costs nothing:

- reset reason HAD_POR (nothing else set): a Start held at boot IS the turn-on press; time the 2.5 s from power-up
  (time since reset), not from the first read;
- any other reason (HAD_RUN_LOW, a watchdog bit, HAD_BOR): do not latch until Start has been seen released since
  boot, then a new press held 2.5 s.

The datasheet says each flag means "last reset was ..." but does not say in one sentence that a new reset clears
the others (the bits are read-only latches). So VER-240 must read CHIP_RESET at the boot after an 8 s hold and
record that HAD_RUN_LOW is set and HAD_POR is clear. If that fails, fall back to the Start-released-since-boot rule
for every boot except the very first after the cell is connected. The Start-released rule alone (no register) is
the safe default: it can only err towards "ask the user to press again".

RUN double-tap: two RUN pulses close together set CHIP_RESET.DOUBLE_TAP and the bootrom may enter BOOTSEL
(rp2350.txt:30389, :30426), only if the OTP flag BOOT_FLAGS1.DOUBLE_TAP is set. Two TPS3421 pulses are at least
6 s apart (re-arming needs a release and a full delay), so this does not arise.

### 5. The watchdog: CONFIRMED

LOAD is 24 bits, "the maximum setting is 0xffffff which corresponds to approximately 16 seconds" (rp2350.txt:72185,
section 12.9 and Table 1360, p.1108); CTRL.TIME reads in microseconds (:72171), so the resolution is 1 us. 2 s is
settable (the SDK takes milliseconds, :72040-72075). A watchdog reset resets the watchdog (:71972).

Pads: the SDK's watchdog code sets PSM_WDSEL to every stage except ROSC and XOSC (:72040), which includes SIO,
the RESETS block and the processors, so a watchdog reset runs the sequence from the RESETS stage on (7.4.1 steps
5-12, :30514-30528) with every peripheral back in reset (RESET register reset values 0x1 for PADS_BANK0, IO_BANK0,
:30892). The datasheet never says in one line that PADS_BANK0 is reset by a PSM watchdog reset; the pad list that
"returns to the reset state" names brown-out, RUN, SW-DP and rescue only (:36034-36038). So the drop of PWR_HOLD
after a watchdog reset is carried by SIO's reset (the output enable clears) plus R42, not by the pads' reset
alone: CONFIRMED with R42 mandatory, and VER-243 stays the bench proof. Side note for REQ-SOC-10: watchdog scratch
registers reset on a RUN reset (:72014-72016) and on a watchdog chip-level reset (:30294), so the failed-boot
counter survives a PSM watchdog reset but not the 8 s hard reset, which matches "a hard reset does not count".

### 6. The pad erratum behind R42: CONFIRMED, with a limit on who needs it

RP2350-E9, Appendix D.5.1 (rp2350.txt:87983-88066, datasheet p.1358-1359): "Increased leakage current on Bank 0
GPIO when pad input is enabled". Affects RP2350 A2 only; "Fixed by RP2350 A3, Documentation" (:88066; the
A3 hardware-fixes list, :87417). With input enable set, output disabled and the pad between VIL and VIH, the pad
sources about 120 uA and sits near 2.2 V; the internal pull-down cannot overcome it. "Driving / pulling the pad
input low with a low impedance source of 8.2 kOhm or less will overcome the erroneous leakage" (:88001). Right after
a PoR or RUN reset the input enable is clear, so the internal pull-down works (:88045). R42 4.7k is under 8.2k with
a factor 1.7. The A2 flaw only matters if the chip is an A2 and input enable is set on GPIO44 (a watchdog reset
before the pad registers are reset); on A3 and later R42 is harmless but not needed for E9. Cost: 3.3 V / 4.7k =
0.70 mA from 3V3 whenever PWR_HOLD is high. The stepping of the JLC parts is not known (analysis/errata_rp2350.md).

### 7. REQ-PWR-10's arithmetic: CONFIRMED

Watchdog 2 s (settable, answer 5) plus the release of REQ-PWR-09's 2 s is 4 s; 10 s / 4 s is 2.5 times margin. The
TPS3421 path (6 s to 9 s, answer 1, plus the pulse and the 2 s release) is a user's way out, not part of the count,
as REQ-PWR-10's rationale already says. The statement needs no change.

## Other things the documents show

- Flash safeguard (REQ-PWR-21): 4 s plus an erase or write under 2 s ends before 6 s, but the TPS3421's minimum is
  exactly 6.0 s, so there is zero margin if an operation takes the full 2 s; and the TPS3421 starts counting at the
  press (power-up on a turn-on press), the firmware at its own boot. Count the hold from power-up when the press
  began before boot (as for the turn-on hold above). The margin is Alex's call; the numbers are unchanged here.
- A user who holds Start for the turn-on and keeps holding gets the unit turned on at 2.5 s and reset at 6 s to
  9 s after power-up (the TPS3421 starts when 3V3 comes up with Start low; tSD 300 us, :318).
- While Start is held after the 8 s reset the unit is reset but powered until release; REQ-PRD-08.B's "reset and
  turn off the unit" is still true (it turns off at the release). The firmware may idle the display. No change.

## Verdicts

| # | Claim | Verdict |
|---|---|---|
| 1 | a TPS3421 delay inside 6 s to 10 s: EG, TS = GND, 6 s to 9 s | CONFIRMED |
| 2 | output "reset pulse"; open-drain; 3V3 fed | CHANGED: a fixed 400 ms pulse (0.32 s to 0.48 s); PB1 and PB2 both must see Start, with external pull-ups |
| 3 | Start held after the fire | CHANGED: reboot with Start held after 0.4 s, the unit off at the release |
| 4 | no re-latch on a held press | CHANGED: the rule is needed, and must not apply after a power-on reset |
| 5 | watchdog 2 s, PWR_HOLD drops | CONFIRMED (the drop rests on R42 plus SIO's reset) |
| 6 | R42 4.7k against E9 | CONFIRMED (A2 only; 8.2k limit) |
| 7 | 2 s + 2 s against 10 s | CONFIRMED |

## Proposed final Statements (all stay PROPOSED, for Alex to sign)

REQ-SOC-04: With no reset button, the RP2350B shall come out of reset by its power-on reset at every power-up, a
hung firmware shall reset by the watchdog within 2 s, and holding seat 1's Start for 8 s (6 s to 10 s: Alex,
2026-10-29) shall pull the RP2350B's RUN pin low once for a fixed pulse, through a push-button controller fed from
3V3, and so reset the RP2350B whatever the firmware is doing, whenever the unit is on, with USB in or not; the
unit then turns off when Start is released (REQ-PWR-09).

REQ-PWR-09: On the cell with no USB, after a watchdog reset, a panic or the TPS3421's hard reset (the 8 s hold of
seat 1's Start, REQ-SOC-04), the power hold shall release and every rail fall below 0.5 V within 2 s of the reset,
or, when seat 1's Start is still held, within 2 s of its release, 20 of 20 tries after the 8 s hold held on to
12 s; and after any reset that is not a power-on reset (the RUN pin's, the watchdog's, a brown-out) the firmware
shall latch PWR_HOLD only on a press of Start that begins after it has seen Start released.

REQ-PWR-10: unchanged: Without USB, no firmware hang shall keep the device on for more than 10 s; every image,
the self-test included, shall enable the watchdog before any blocking init.

REQ-PWR-21: Seat 1's Start shall be the power key: pressing it shall power the regulator, the firmware shall latch
PWR_HOLD only after Start has been held 2.5 s (2 to 3 s, counted from power-up when the press began before the
firmware booted), and if it is released earlier the power shall drop and the unit be fully off; the only button
power-off shall be the hard-reset hold of about 8 s (REQ-SOC-04), with no shorter firmware power-off hold, and a
tap of Start still opening the pause menu; the firmware shall not start a flash erase or write once Start has been
held 4 s or more, and after a reset that is not a power-on reset shall latch PWR_HOLD only on a Start press that
begins after it has seen Start released (REQ-PWR-09). There shall be no power slider, slide switch, case slot for
one or board-edge switch.

## Design changes the documents force

1. TPS3421EG, TS pin to GND: no change of part or delay (7.5 s, 6 s to 9 s).
2. PB1 and PB2 both on Start's buffered line (and a pull-up to 3V3 on it, which DF-266's buffer has).
3. The firmware rule: reset-reason aware (HAD_POR versus the rest), replacing "only a press that begins after boot".
4. No circuit change to R42 or PWR_HOLD.

Last updated: 2026-10-30 (the day of the download; time of day not recorded).
