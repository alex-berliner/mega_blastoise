#!/usr/bin/env python3
"""Paper checks of classes 4 (values and worst case) and 6 (states and sequencing), from the design files.

    python3 hardware/vv/tools/paper_checks.py <check>           # exit 1 when the check's limit fails
    python3 hardware/vv/tools/paper_checks.py <check> --seed    # the same check on a seeded fault: must exit 1
    python3 hardware/vv/tools/paper_checks.py --list

  latch     VER-33   both Start paths into U3's EN: the sure-on, sure-off and ghost-press bands, E9 after a reset
  usb       VER-191  R21/R22 against Espressif's 22/33 ohm and their distance to the module; D+ and D- capacitance
  audio     VER-100  the PWM filter, Ci/Ri and the bridge gain at every tolerance corner: band edges and the carrier
  module    VER-204  the ESP32-S3 module's supply: its 22 uF and 0.1 uF at pin 2, EN's RC, 3V3 through a Wi-Fi burst
  shelf     VER-103  the off-state budget with the cell in, and the months from the cutoff to the PCM's UVP
  ampstate  VER-196  U7 shut down in every state but the firmware's AMP_EN high
  hotplug   VER-12   a 2 m cable plugged live into R41/C1 and U2's IN (an ODE): VBUS, EN1 and PWR_EN peaks
  mating    VER-140  the cell plugged into J2: the peak into the VBAT and VSYS capacitors, and PWR_EN through the edge
  mlccbias  VER-30   each MLCC's bias against its part's minimum: the derating the curves must show
  backlight VER-05   the two LED current sinks: phase margin at every corner of a nodal model, DC, headroom at 3.5 V,
                     the off states and a USB plug-in while lit (2026-10-28; was the ngspice run, DEF-45)

Each check reads design.py (through design_numbers.design()) and, for 'usb' and 'core', the routed board
(pcbnew). A datasheet number design.py does not hold is a named constant below with its file and line.
'<name>.txt:<line>' is the pdftotext -layout copy of <name>.pdf in hardware/ref_design/datasheets (tools/ds.py;
tools/datasheet_hashes.py records their sha256). A number from a
datasheet that is not on disk is taken from board/pin_review/<part>.json (which quotes the page) and is
marked 'not on disk': VER-34 confirms it. A stand-in is a number no file gives, chosen pessimistic, and
says so where it is printed.

--seed applies the check's SEED (a design value moved to where the check must fail) to the parts in
memory, so each check is shown failing once (README: a check that can be a script can fail).

Python 3 standard library, plus pcbnew for 'usb' and 'core'.
"""
import cmath
import itertools
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

# ---- datasheet numbers ----------------------------------------------------------------------------------
# TLV75733P (U3 since 39cc07f), SBVS322C, tlv757p.pdf
U3_VIH = 1.0                     # V: EN pin high voltage (enabled), minimum (tlv757p.txt:328)
U3_VIL = 0.3                     # V: EN pin low voltage, maximum (tlv757p.txt:331)
U3_IEN = 1e-6                    # A: EN pin current: 10 nA typical only (tlv757p.txt:333); 1 uA stand-in
U3_ISHDN = 1e-6                  # A: shutdown current at VEN 0.4 V or less, maximum (tlv757p.txt:265)
U3_EN_ABS = 6.0                  # V: EN absolute maximum (tlv757p.txt:179)
U3_EN_REC = 5.5                  # V: EN recommended maximum (tlv757p.txt:208)
U3_TSTR = 550e-6                 # s: start-up time, typical (tlv757p.txt:326)
# BQ24073 (U2), SLUS810N, bq24074.txt
U2_IBAT_PDWN = 6.5e-6            # A: sleep current into BAT, input not detected, TJ 85 C, maximum (bq24074.txt:714)
U2_OVP_MIN = 6.4                 # V: input OVP threshold, minimum (bq24074.txt:699)
U2_EN_ABS = 7.0                  # V: EN1, EN2, CE ... absolute maximum (bq24074.txt:598-599)
U2_EN_PD = 285e3                 # ohm: EN1's internal pull-down, approximately (bq24074.txt:512)
U2_VO_REG = (4.3, 4.5)           # V: OUT regulation with input (bq24074.txt:729)
U2_BAT_OUT_R = 0.1               # ohm: VBAT - VOUT 100 mV max at 1 A, VIN 0 (bq24074.txt:724)
# ESP32-S3 (U4's chip since 39cc07f), esp32-s3.pdf (series datasheet v2.2) and the module's, esp32-s3-wroom-1.pdf
ESP_IIN = 50e-9                  # A: IIH and IIL, maximum, at 25 C (esp32-s3.txt:3748-3749)
ESP_VOH = 2.64                   # V: VOH 0.8 x VDD at 3.3 V (esp32-s3.txt:3750), into a high-impedance load (3768)
ESP_VDD_MIN = 3.0                # V: the module's VDD33, minimum (esp32-s3-wroom-1.txt:1596)
ESP_TX_PEAK = 0.355              # A: 802.11b 1 Mbps at 20.5 dBm, the module's TX peak (esp32-s3-wroom-1.txt:1662)
ESP_EN_RC = (10e3, 1e-6)         # ohm, F: CHIP_PU's RC delay 'usually R = 10 kOhm and C = 1 uF' (esp32-s3-hw-design-guidelines.txt:338)
ESP_USB_R = (22.0, 33.0)         # ohm: USB series resistors, 'initial value can be 22/33' (esp32-s3-hw-design-guidelines.txt:874)
ESP_3V3_C = (22e-6, 0.1e-6)      # F: the module's peripheral schematic, 22 uF and 0.1 uF on 3V3 (esp32-s3-wroom-1.txt:2875)
ESP_PWM_CLK = 80e6               # Hz: LEDC from the 80 MHz APB clock; AUDIO_PWM at 8 bits is 312.5 kHz (design.py, pinmap.md)
# Parts whose datasheets are on disk since 2026-10-28
Q_VTH = (1.0, 2.5)               # V: 2N7002 Vth(GS) minimum, maximum (2n7002_cj.txt:55)
VF_01MA, VF_DECADE = 0.240, 0.080  # V: BAT54C 240 mV max at 0.1 mA, 320 mV at 1 mA (bat54c.txt:113-115); BAT54WS the same (bat54ws.txt:75-77)
U1_C = 5e-12                     # F: SRV05-4 junction capacitance, maximum (srv05-4.txt:105)
U7_ISD = 4.2e-6                  # A: FM8002A shutdown current IOFF, typical only (fm8002a.txt:123)
SW24_R = (0.1, 0.5)              # ohm: TPS850 contact resistance, 100 mOhm and 500 mOhm max (rocpu_tps850.txt:20, 137)
# MCP6002 (U6) and AO3400A (Q5, Q6) since 2026-10-28 (VER-05 on paper); backlight() re-reads each line through ds.quote()
U6_GBW = 1.0e6                   # Hz: gain bandwidth product, typical only (mcp6001.txt:176)
U6_AOL = 10 ** (88 / 20)         # DC open-loop gain, 88 dB minimum (mcp6001.txt:151)
U6_VOS = 4.5e-3                  # V: input offset, maximum, at VCM = VSS (mcp6001.txt:135)
U6_SWING = 0.025                 # V: output swing from either rail at VDD 5.5 V (mcp6001.txt:154)
U6_CIN = 6e-12 + 3e-12           # F: common-mode 6 pF and differential 3 pF input capacitance (mcp6001.txt:145-146)
U6_VDD_MIN = 1.8                 # V: supply minimum (mcp6001.txt:159)
Q_CISS, Q_COSS, Q_CRSS = 630e-12, 75e-12, 50e-12   # F: typical, VGS 0 V, VDS 15 V, 1 MHz (ao3400a.txt:85-87)
Q_RG_INT = 4.5                   # ohm: the AO3400A's gate resistance, maximum (ao3400a.txt:88)
AO_VTH = (0.65, 1.45)            # V: AO3400A VGS(th) at 250 uA, minimum and maximum (ao3400a.txt:73)
AO_RDS_25 = 0.048                # ohm: AO3400A RDS(ON) at VGS 2.5 V, maximum (ao3400a.txt:80)
LED_VF_MAX = 3.2                 # V: the panel's backlight Vf at 120 mA, maximum (er-tft035ips-6.txt:758)
FH12_R = 0.05                    # ohm: FH12 contact resistance, maximum (hirose_fh12.txt:56)
# Stand-ins: no file gives these; each is pessimistic and printed as a stand-in
STAND_U7_ISD = 2 * U7_ISD        # A: twice the FM8002A's typical, as no maximum is printed
STAND_IDSS = 1e-6                # A: a 2N7002 or AO3400A off at a few volts (the usual 1 uA IDSS at rated VDS)
STAND_CLAMP_V = dn.CLAMP_V       # V: U10's unpowered input clamp (design_numbers.py, the cutoff check)
STAND_ESP_USB_C = 10e-12         # F: the ESP32-S3's USB pad (esp32-s3.pdf gives none; a pin is 2 pF, esp32-s3.txt:3745)
STAND_TRACK_C = 0.1e-12          # F/mm: a track over the B.Cu pour on 1.6 mm FR4, about twice the usual 0.05 pF/mm
STAND_XTAL_STRAY = (2e-12, 5e-12)  # F: board and pin stray at XIN/XOUT
STAND_CELL_MAH = 700.0           # mAh: REQ-PRD-03.R's 'about 700 mAh' until VER-37 reads the cell
STAND_LOW_SHARE = (0.05, 0.10)   # share of the capacity between 3.50 V and the PCM's UVP (REQ-BAT-10's 5-10%)
STAND_PCM = (3e-6, 6e-6)         # A: a DW01-class PCM's own drain (REQ-BAT-10), until VER-37
STAND_SELF = (0.01, 0.03)        # share a month: the cell's self-discharge at 25 C (REQ-BAT-10), doubled at 45 C
STAND_CABLE = dict(r=(0.13, 0.85), l=(1.0e-6, 2.0e-6))  # ohm and H, 2 m loop: 20-28 AWG at 0.033-0.213 ohm/m a
                                                       # conductor; 0.5-1.0 uH/m of loop inductance
STAND_C1_EFF = (0.3, 1.0)        # share of C1's 10 uF left at 5 V (no DC-bias curve on disk: VER-30)
CLOSE_MM = 5.0                   # mm: this check's reading of the datasheet's 'close to the pin', pad centre to pad centre
R_TOL = dn.R_TOL                 # every resistor in design.py's RES table is 1%
C_TOL = {"K": 0.10, "M": 0.20, "J": 0.05}   # MPN tolerance letter: Samsung CLxx and FH 0402 codes


def seeded(parts, seed):
    for ref, val in seed.items():
        parts[ref] = dict(parts[ref], value=val)
    return parts


def load(seed):
    parts, ns = dn.design()
    return (seeded(parts, seed) if seed else parts), ns


def r(parts, ref):
    return dn.ohms(parts[ref]["value"])


def c(parts, ref):
    return dn.farads(parts[ref]["value"])


def ctol(parts, ref):
    mpn = parts[ref]["mpn"] or ""
    if mpn.startswith("CL"):                     # Samsung: CL05B104KO5..., tolerance after the value code
        return C_TOL[mpn[8]]
    if mpn.startswith("0402"):                   # FH: 0402CG150J500NT, 0402B472K500NT
        return C_TOL[mpn.rstrip("NT")[-4]]
    return 0.2


def vf_max(i):
    """Schottky forward voltage, the datasheets' maximum line (240 mV at 0.1 mA, 80 mV a decade)."""
    return VF_01MA + VF_DECADE * math.log10(max(i, 1e-9) / 1e-4)


def bisect(f, lo, hi, n=80):
    flo = f(lo)
    for _ in range(n):
        mid = (lo + hi) / 2
        if (f(mid) > 0) == (flo > 0):
            lo, flo = mid, f(mid)
        else:
            hi = mid
    return (lo + hi) / 2


# ---- VER-33 -----------------------------------------------------------------------------------------------
SEED_LATCH = {"R13": "1k"}           # PWR_BTN_RAW's pull-down so stiff the button's path cannot lift EN (and Q1's gate)


def latch_point(vsys, rs, rpd, ren, vf_on, ien):
    """The button's contact (rs) from VSYS into the RAW node (rpd to GND, a Schottky into PWR_EN, ren to GND)."""
    def diode_i(v):
        if v <= 0:
            return 0.0
        if not vf_on:
            return v / ren + ien
        if v <= vf_max(ien):
            return 0.0
        return bisect(lambda i: v - vf_max(i) - max(i - ien, 0.0) * ren, ien, v / ren + ien)

    raw = bisect(lambda v: (vsys - v) / rs - v / rpd - diode_i(v), 0, vsys)
    i = diode_i(raw)
    en = max(0.0, (i - ien) * ren) if vf_on else raw
    return raw, en


def latch(seed=None):
    """39cc07f: one power contact, SW24 (a TPS850 tact switch in seat 1's deck, VSYS to PWR_BTN_RAW); the Starts are plain
    buttons to GND. Bands of contact resistance and leakage across SW24, the hold through PWR_HOLD and D2, and the
    release after a reset or crash (R42 against the ESP32-S3's pad, which has no erratum like the RP2350's E9)."""
    parts, _ = load(seed)
    lo, hi = 1 - R_TOL, 1 + R_TOL
    r15 = r(parts, "R15")
    ok = True
    print(f"U3 TLV75733P EN: VIH {U3_VIH} V min, VIL {U3_VIL} V max (tlv757p.txt:328, 331), IEN {U3_IEN * 1e6:.0f} uA (stand-in; "
          f"10 nA typical, 333); Q1 2N7002 VGS(th) {Q_VTH[0]}-{Q_VTH[1]} V (2n7002_cj.txt:55)")
    print(f"Schottky VF max {VF_01MA * 1e3:.0f} mV at 0.1 mA, {VF_DECADE * 1e3:.0f} mV a decade (bat54c.txt:113-115, "
          "bat54ws.txt:75-77); for the off cases VF is taken as 0, the worst there")
    sw = [ref for ref, p in parts.items() if p["bom"] and set(p["pins"].values()) == {"VSYS", "PWR_BTN_RAW"}]
    starts = [ref for ref, p in parts.items() if ref.startswith("SW") and "START" in p["value"]]
    on_vsys = [ref for ref in starts if "VSYS" in parts[ref]["pins"].values()]
    ok &= len(sw) == 1 and not on_vsys
    print(f"power contacts (VSYS to PWR_BTN_RAW): {', '.join(sw) or 'none'}; Starts ({', '.join(starts)}) on VSYS: "
          f"{', '.join(on_vsys) or 'none, they are plain buttons to GND'}")
    rpd = r(parts, "R13")
    for vsys in (3.42, 3.0):
        raw, en = latch_point(vsys, SW24_R[1] + 1e3, rpd * lo, r15 * lo, True, U3_IEN)
        on = en >= U3_VIH and raw >= Q_VTH[1]
        ok &= on
        print(f"SW24 at its 500 mOhm maximum (rocpu_tps850.txt:137) plus 1 kohm of dirt, VSYS {vsys} V: Q1's gate {raw:.3f} V "
              f"({raw - Q_VTH[1]:+.3f} V over its 2.5 V maximum threshold), EN {en:.3f} V ({en - U3_VIH:+.3f} V over VIH): "
              f"{'on' if on else 'NOT SURE ON'}")
    raw, en = latch_point(4.4, 1e6, rpd * hi, r15 * hi, False, 0)
    off = en <= U3_VIL and raw < Q_VTH[0]
    ok &= off
    print(f"1 Mohm across SW24 at VSYS 4.4 V: RAW {raw:.3f} V, EN at most {en:.3f} V (VIL {U3_VIL}); Q1's gate against its "
          f"1.0 V minimum threshold: {'off' if off else 'NOT SURE OFF'}")
    on_max = bisect(lambda rs: min(latch_point(3.0, rs, rpd * lo, r15 * lo, True, U3_IEN)[1] - U3_VIH,
                                   latch_point(3.0, rs, rpd * lo, r15 * lo, True, U3_IEN)[0] - Q_VTH[1]), 1, 1e6)
    ghost = bisect(lambda rs: latch_point(4.4, rs, rpd * hi, r15 * hi, False, 0)[0] - Q_VTH[0], 1, 1e8)
    offmin = bisect(lambda rs: latch_point(4.4, rs, rpd * hi, r15 * hi, False, 0)[1] - U3_VIL, 1, 1e8)
    print(f"  bands: sure on (EN and Q1 both) up to {on_max / 1e3:.1f} kohm at VSYS 3.0 V (the switch is rated 500 mOhm); Q1 may "
          f"read a ghost press under {ghost / 1e3:.0f} kohm of leakage at 4.4 V; EN may turn on under {offmin / 1e3:.0f} kohm; "
          f"sure off above {max(ghost, offmin) / 1e3:.0f} kohm (1 Mohm is {'above' if 1e6 > max(ghost, offmin) else 'NOT above'} it)")
    ok &= 1e6 > max(ghost, offmin) and on_max >= 1e3          # REQ-PWR-08's 1 kohm of dirt in series
    # after a reset or crash: GPIO21 resets as an input with no pull (pinmap.md 'Pins at reset'), R42 pulls it down
    r42 = r(parts, "R42")
    leak = ESP_IIN * r42 * hi
    rel = leak < U3_VIL
    ok &= rel
    print(f"after a reset or crash: PWR_HOLD (GPIO21) an input, its {ESP_IIN * 1e9:.0f} nA leakage maximum (esp32-s3.txt:3748) "
          f"through R42 {parts['R42']['value']}: {leak * 1e3:.2f} mV, so EN falls once SW24 is up (the ESP32-S3 has no pad "
          "erratum like the RP2350's E9: R42 could be 100k)")
    hold = latch_point(ESP_VOH, 1e-3, r42 * lo, r15 * lo, True, U3_IEN)[1]
    ok &= hold >= U3_VIH
    print(f"held by firmware (PWR_HOLD at the pad's VOH 0.8 x VDD = {ESP_VOH} V, esp32-s3.txt:3750, into a light load): EN "
          f"{hold:.3f} V ({hold - U3_VIH:+.3f} V over VIH); GPIO21 sources {ESP_VOH / (r42 * lo) * 1e3:.2f} mA into R42 "
          "(VOH is specified into a high impedance, 3768: at 0.56 mA the pad stays near it)")
    # the start-up chain a press must cover before firmware can set PWR_HOLD
    en_rc = ESP_EN_RC[0] * ESP_EN_RC[1]
    t_en = -en_rc * math.log(1 - 0.75)
    print(f"a press must last: U3's start-up {U3_TSTR * 1e6:.0f} us (tlv757p.txt:326) + EN's RC to 0.75 x VDD "
          f"(VIH_nRST, esp32-s3.txt:3761) {t_en * 1e3:.1f} ms with R23/C12 + the ROM and bootloader (not in any datasheet: "
          "VER-73 times it) + pinmap.md's 1 s hold rule")
    r14 = r(parts, "R14")
    for vbus in (4.75, 5.25):
        v = latch_point(vbus, r14 * hi, 1e12, r15 * lo, True, U3_IEN)[1]
        print(f"USB at {vbus} V: PWR_EN {v:.2f} V through R14 {parts['R14']['value']} and D3 (turns the device on: {v >= U3_VIH})")
    return dn.result(ok)


# ---- VER-196 -----------------------------------------------------------------------------------------------
SEED_AMP = {"R37": "100M"}           # AMP_EN's pull-down so weak the pad's leakage nears Q3's threshold


def ampstate(seed=None):
    parts, _ = load(seed)
    r36, r37 = r(parts, "R36") * (1 + R_TOL), r(parts, "R37") * (1 + R_TOL)
    gate = dn.I_LEAK * r37
    margin = Q_VTH[0] - gate
    ok = margin >= 0.4
    print(f"Q3 2N7002 VGS(th) {Q_VTH[0]}-{Q_VTH[1]} V (2n7002_cj.txt:55); AMP_EN (GPIO45) held by R37 {parts['R37']['value']} "
          f"(+1%) against the pad's leakage ({dn.I_LEAK * 1e6:g} uA stand-in, 20 x IIH's 50 nA, esp32-s3.txt:3748): gate at most "
          f"{gate:.3f} V, {margin:.2f} V under the minimum threshold (limit 0.4 V)")
    states = [
        ("off, VSYS live", "U4 unpowered, its pad passes no current; R37 holds the gate at 0 V"),
        ("power-up before firmware", "GPIO45 resets with its weak pull-down (WPD, IE: esp32-s3.txt:901) and has no power-up "
         "glitch (Table 2-2, 958-1006); it is the VDD_SPI strapping pin, so it must read 0 at reset anyway (VER-203)"),
        ("EN low (reset)", "the chip is held in reset: its pads as at power-up"),
        ("watchdog or panic reset", "a system reset returns the pads to their reset state: input with GPIO45's pull-down"),
        ("light or deep sleep", "firmware rule: drive AMP_EN low before sleep (or hold it with the RTC's pad hold)"),
        ("USB in, cell flat", "VSYS is U2's OUT, 4.3-4.5 V (bq24074.txt:729); AMP_EN as above"),
    ]
    for name, why in states:
        print(f"  {name}: AMP_EN <= {gate:.3f} V, Q3 off, U7 shut down: {why}")
    for vsys in (3.0, 4.4):
        shdn = vsys - STAND_IDSS * r36
        print(f"VSYS {vsys} V: AMP_SHDN at least {shdn:.2f} V (R36 {parts['R36']['value']} with Q3's off leakage, "
              f"stand-in {STAND_IDSS * 1e6:.0f} uA; 80 nA at 60 V, 2n7002_cj.txt:57): within {vsys - shdn:.2f} V of VDD, "
              "the 'supply voltage' the FM8002A names for shutdown (fm8002a.txt:75-76; it prints no VIH: limit 95% of VDD)")
        ok &= shdn >= 0.95 * vsys
    off = STAND_U7_ISD + STAND_IDSS
    print(f"U7's shutdown {U7_ISD * 1e6:.1f} uA typical (fm8002a.txt:123, no maximum; {STAND_U7_ISD * 1e6:.1f} uA as a stand-in "
          f"maximum) and R36's {STAND_IDSS * 1e6:.0f} uA (Q3's leakage): {off * 1e6:.1f} uA of the off budget; "
          "the whole budget is 'shelf' (VER-103), against VER-65's 30 uA")
    total = off_budget(parts, 4.2, 25)[0]
    ok &= total <= 30e-6
    print(f"the whole off budget at 4.2 V, 25 C: {total * 1e6:.1f} uA (limit 30 uA)")
    return dn.result(ok)


# ---- VER-103 ------------------------------------------------------------------------------------------------
def off_budget(parts, vbat, temp_c):
    """The cell's current with the device off and no USB, part by part (A), and the lines printed."""
    warm = 2.0 if temp_c > 30 else 1.0
    r17, r18 = r(parts, "R17"), r(parts, "R18")
    vdiv = min(vbat * r18 / (r17 + r18), STAND_CLAMP_V)
    lines = [
        ("U2 sleep current into BAT", U2_IBAT_PDWN, "6.5 uA max at TJ 85 C (bq24074.txt:714): used at 25 and 45 C"),
        ("U3 shut down", U3_ISHDN * warm, f"1 uA max (tlv757p.txt:265){'; doubled at 45 C (stand-in)' if warm > 1 else ''}"),
        ("R17 into U10's unpowered input", (vbat - vdiv) / r17,
         f"VBAT_DIV held at {vdiv:.2f} V by U10's input clamp (estimate {STAND_CLAMP_V} V; mcp6001.txt:115-117 allows it)"),
        ("U7 shut down on VSYS", STAND_U7_ISD * warm, "FM8002A 4.2 uA typical (fm8002a.txt:123): 2 x typical, stand-in"),
        ("Q3 off (R36 from VSYS)", STAND_IDSS * warm, "2N7002 IDSS, stand-in"),
        ("Q5, Q6 off (the backlight LEDs from VSYS)", 2 * STAND_IDSS * warm, "AO3400A IDSS, stand-in"),
    ]
    return sum(i for _, i, _ in lines), lines


SEED_SHELF = {"R17": "47k", "R18": "47k"}     # a stiff divider: 10 x the drain


def shelf(seed=None):
    parts, _ = load(seed)
    ok = True
    print("off, no USB, cell in: VSYS is live through U2's battery FET; 3V3, the module and the panel are off")
    for temp in (25, 45):
        tot, lines = off_budget(parts, 4.2, temp)
        print(f"-- {temp} C, VBAT 4.2 V")
        for name, i, why in lines:
            print(f"  {name:44s} {i * 1e6:5.1f} uA  {why}")
        print(f"  board total {tot * 1e6:.1f} uA (REQ-PWR-07: 30 uA at 25 C, 60 uA at 45 C): "
              f"{'within' if tot <= (30e-6 if temp == 25 else 60e-6) else 'OVER'}")
        ok &= tot <= (30e-6 if temp == 25 else 60e-6)
        at35 = off_budget(parts, 3.5, temp)[0]
        warm = 2 if temp > 30 else 1
        for case, k in (("nominal", 0), ("worst", 1)):
            pcm = STAND_PCM[k]
            selfd = STAND_SELF[k] * warm * STAND_CELL_MAH
            low = STAND_CELL_MAH * STAND_LOW_SHARE[1 - k]
            month = 730.0                                         # h
            drain = (at35 + pcm) * 1e3 * month + selfd             # mAh a month from the cutoff
            m_cut = low / drain
            m_full = STAND_CELL_MAH * (1 - STAND_LOW_SHARE[1 - k]) / ((tot + pcm) * 1e3 * month + selfd) + m_cut
            print(f"  {case}: board {at35 * 1e6:.1f} uA at 3.5 V + PCM {pcm * 1e6:.0f} uA + self-discharge "
                  f"{STAND_SELF[k] * warm * 100:.0f}% a month = {drain:.1f} mAh a month; {low:.0f} mAh from 3.50 V to the "
                  f"UVP: {m_cut:.1f} months from the cutoff (limit 2), {m_full:.1f} months from full (limit 24)")
            if temp == 25 or case == "worst":
                ok &= m_cut >= 2 and (temp > 30 or m_full >= 24)
    print("stand-ins: capacity 700 mAh (REQ-PRD-03.R), 5-10% of it between 3.50 V and the UVP, a DW01-class PCM's "
          "3-6 uA and 1-3% a month of self-discharge (REQ-BAT-10), all until VER-37 reads the cell; the 45 C period "
          "is Alex's to set (REQ-BAT-10), so 45 C is printed and counted only at the worst case from the cutoff")
    return dn.result(ok)


# ---- VER-191 -------------------------------------------------------------------------------------------------
SEED_USB = {"R21": "100"}


def usb(seed=None):
    """39cc07f: R21/R22 against Espressif's 22/33 ohm, placed close to the module (esp32-s3-hw-design-guidelines.txt:873-875,
    1307), read here as their pad within CLOSE_MM of the module's pin; D+ and D- capacitance to ground."""
    parts, _ = load(seed)
    pcb, b = dn.board()
    mm = pcb.ToMM
    ok = True
    for ref in ("R21", "R22"):
        v = r(parts, ref)
        lo, hi = v * (1 - R_TOL), v * (1 + R_TOL)
        good = ESP_USB_R[0] * (1 - R_TOL) <= lo and hi <= ESP_USB_R[1] * (1 + R_TOL)
        ok &= good
        print(f"{ref} {parts[ref]['value']} ohm 1%: {lo:.2f}-{hi:.2f} ohm against Espressif's 'initial value can be 22/33' "
              f"(esp32-s3-hw-design-guidelines.txt:874): {'inside' if good else 'OUTSIDE 22-33 ohm'}")
    print("ZDRV (USB 2.0's 28-44 ohm with the series resistor) also needs the PHY's own output impedance, which esp32-s3.pdf "
          "does not give: 22/33 ohm is Espressif's starting value for that sum, to be tuned on the board (874)")
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    u4 = {p.GetNumber(): p for p in fps["U4"].Pads()}
    length, vias = {}, {}
    for t in b.GetTracks():
        n = t.GetNetname().split("/")[-1]
        if n in ("USB_DP", "USB_DM", "USB_DP_MCU", "USB_DM_MCU"):
            if t.GetClass() == "PCB_TRACK":
                length[n] = length.get(n, 0) + mm(t.GetLength())
            else:
                vias[n] = vias.get(n, 0) + 1
    for ref, net in (("R21", "USB_DP_MCU"), ("R22", "USB_DM_MCU")):
        pin = next(k for k, v in parts["U4"]["pins"].items() if v == net)
        pad = next(p for p in fps[ref].Pads() if p.GetNetname().split("/")[-1] == net)
        d = math.hypot(mm(pad.GetPosition().x - u4[pin].GetPosition().x), mm(pad.GetPosition().y - u4[pin].GetPosition().y))
        good = d <= CLOSE_MM
        ok &= good
        print(f"{ref}'s pad on {net} is {d:.1f} mm from the module's pin {pin} in a straight line, {length.get(net, 0):.1f} mm of "
              f"track: 'place them close to the chip' (esp32-s3-hw-design-guidelines.txt:875, 1307), read here as {CLOSE_MM:g} mm: "
              + ("close" if good else "NOT close: the line between the pin and the resistor is unterminated (at full speed's "
                 "4-20 ns edges a 20 mm stub is 0.13 ns, so the cost is EMI and ringing, not timing)"))
    for line in ("DP", "DM"):
        mm_ = length.get(f"USB_{line}", 0) + length.get(f"USB_{line}_MCU", 0)
        nv = vias.get(f"USB_{line}", 0) + vias.get(f"USB_{line}_MCU", 0)
        ctot = U1_C + STAND_ESP_USB_C + mm_ * STAND_TRACK_C + nv * 0.5e-12
        good = ctot <= 100e-12
        ok &= good
        print(f"D{'+' if line == 'DP' else '-'}: track {mm_:.1f} mm ({mm_ * STAND_TRACK_C * 1e12:.1f} pF at "
              f"{STAND_TRACK_C * 1e12:.1f} pF/mm, stand-in), {nv} vias (0.5 pF each), U1 {U1_C * 1e12:.0f} pF max "
              f"(srv05-4.txt:105), the module's pad {STAND_ESP_USB_C * 1e12:.0f} pF (stand-in): {ctot * 1e12:.1f} pF "
              f"against 100 pF")
    mismatch = abs(length.get("USB_DP", 0) + length.get("USB_DP_MCU", 0) - length.get("USB_DM", 0) - length.get("USB_DM_MCU", 0))
    print(f"D+/D- length mismatch {mismatch:.2f} mm (VER-108 owns the limit)")
    return dn.result(ok)


# ---- VER-100 ------------------------------------------------------------------------------------------------
SEED_AUDIO = {"C37": "47n", "C38": "47n"}


def audio_h(f, r32, c37, r33, c38, c39, r34, r35, rsrc):
    """Bridge output over the PWM's swing: two RC stages, then Ci and Ri into the inverting input's virtual
    ground (the FM8002A's -IN; gain 2 x Rf / Ri, u7.json pin 4)."""
    w = 2 * math.pi * f
    zc = lambda cap: 1 / (1j * w * cap)                     # noqa: E731
    zin = zc(c39) + r34                                     # into the virtual ground
    z2 = 1 / (1 / zc(c38) + 1 / zin)                        # node AUDIO_F2
    zb = r33 + z2
    z1 = 1 / (1 / zc(c37) + 1 / zb)                         # node AUDIO_F1
    v1 = z1 / (z1 + r32 + rsrc)
    v2 = v1 * z2 / zb
    return 2 * r35 * v2 / zin


def audio(seed=None):
    parts, _ = load(seed)
    names = ["R32", "C37", "R33", "C38", "C39", "R34", "R35"]
    nom = [r(parts, n) if n[0] == "R" else c(parts, n) for n in names]
    tol = [R_TOL if n[0] == "R" else ctol(parts, n) for n in names]
    carrier = ESP_PWM_CLK / 256
    worst = {"hi3db": 1e9, "lo3db": 0, "carrier": 0}
    for signs in itertools.product((-1, 1), repeat=len(names)):
        vals = [v * (1 + s * t) for v, s, t in zip(nom, signs, tol)]
        for rsrc in (0.0, 170.0):                 # the pad's own resistance, 0 to 170 ohm (a weak drive: stand-in)
            h = lambda f: abs(audio_h(f, *vals, rsrc))   # noqa: E731
            mid = h(1e3)
            hi = bisect(lambda f: h(f) - mid / math.sqrt(2), 2e3, 2e5)
            lo = bisect(lambda f: h(f) - mid / math.sqrt(2), 1.0, 900.0)
            att = 20 * math.log10(mid / h(carrier))
            worst["hi3db"] = min(worst["hi3db"], hi)
            worst["lo3db"] = max(worst["lo3db"], lo)
            worst["carrier"] = min(worst["carrier"] or 1e9, att)
    h0 = lambda f: abs(audio_h(f, *nom, 0.0))    # noqa: E731
    print("parts: " + ", ".join(f"{n} {parts[n]['value']} {t * 100:.0f}%" for n, t in zip(names, tol)) +
          " (C tolerances from the MPNs' letter: K 10%, M 20%)")
    print(f"nominal: gain at 1 kHz {20 * math.log10(h0(1e3)):.1f} dB over the PWM's swing (2 x Rf/Ri, u7.json pin 4); "
          f"-3 dB at {bisect(lambda f: h0(f) - h0(1e3) / math.sqrt(2), 1.0, 900.0):.1f} Hz and "
          f"{bisect(lambda f: h0(f) - h0(1e3) / math.sqrt(2), 2e3, 2e5) / 1e3:.1f} kHz; the carrier at "
          f"{carrier / 1e3:.1f} kHz (LEDC from the 80 MHz APB clock / 256) {20 * math.log10(h0(1e3) / h0(carrier)):.1f} dB down")
    print(f"every corner (2^7 tolerance corners x the pad's 0-170 ohm): high -3 dB at least {worst['hi3db'] / 1e3:.1f} kHz "
          f"(limit 10), low corner at most {worst['lo3db']:.1f} Hz (limit 50), carrier at least {worst['carrier']:.1f} dB "
          "down (limit 40)")
    def att_at(share):
        v = list(nom)
        v[1] *= share * (1 - tol[1])
        v[3] *= share * (1 - tol[3])
        return 20 * math.log10(abs(audio_h(1e3, *v, 0.0)) / abs(audio_h(carrier, *v, 0.0)))

    keep = bisect(lambda s: att_at(s) - 40, 0.05, 1.0)
    print(f"C37 and C38 are X7R (0402B472K, 50 V): DC bias at the PWM's 1.65 V mean lowers them (VER-30); the carrier "
          f"stays 40 dB down while they keep {keep * 100:.0f}% of their -10% value")
    print("not in this check: the FM8002A's own bandwidth and PSRR (fm8002a.pdf prints neither)")
    return dn.result(worst["hi3db"] >= 10e3 and worst["lo3db"] <= 50 and worst["carrier"] >= 40)


# ---- VER-204 (39cc07f; VER-197 read the RP2350's copied core) --------------------------------------------------
SEED_MODULE = {"C11": "1u"}


def module(seed=None):
    """The ESP32-S3-WROOM-1U's supply and reset against Espressif's own figures: a 22 uF and a 0.1 uF on 3V3 at pin 2
    (the module's peripheral schematic, esp32-s3-wroom-1.txt:2875), each within CLOSE_MM of the pin; EN's RC (10k, 1 uF,
    esp32-s3-hw-design-guidelines.txt:338) and its rise; a 10 uF at the power entrance (218); the EN trace's length."""
    parts, _ = load(seed)
    pcb, b = dn.board()
    mm = pcb.ToMM
    ok = True

    def row(good, text):
        nonlocal ok
        ok &= good
        print(("  ok    " if good else "  FAIL  ") + text)

    if not parts["U4"]["value"].startswith("ESP32-S3"):
        sys.exit("paper_checks module: U4 is " + parts["U4"]["value"])
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    u4 = {p.GetNumber(): p for p in fps["U4"].Pads()}
    vdd = next(k for k, v in parts["U4"]["pins"].items() if v == "+3V3")
    en = next(k for k, v in parts["U4"]["pins"].items() if v == "EN")

    def pos(p):
        return mm(p.GetPosition().x), mm(p.GetPosition().y)

    def near(pin, net, cmin, cmax=None):
        out = []
        for ref, f in fps.items():
            if ref not in parts or not parts[ref]["bom"] or parts[ref]["lib"] != "Device:C":
                continue
            if net not in parts[ref]["pins"].values() or "GND" not in parts[ref]["pins"].values():
                continue
            cv = c(parts, ref)
            if cv < cmin * 0.99 or (cmax and cv > cmax * 1.01):
                continue
            pad = next(q for q in f.Pads() if q.GetNetname().split("/")[-1] == net)
            out.append((math.hypot(pos(pad)[0] - pos(u4[pin])[0], pos(pad)[1] - pos(u4[pin])[1]), ref))
        return sorted(out)

    print(f"U4 {parts['U4']['mpn']}: 3V3 on pin {vdd}, EN on pin {en}; 'close to the pin' read as a capacitor pad within "
          f"{CLOSE_MM:g} mm of the pin's pad, centre to centre")
    for want, label in ((ESP_3V3_C[0], "22 uF"), (ESP_3V3_C[1], "0.1 uF")):
        hits = near(vdd, "+3V3", want, want)
        d, ref = hits[0] if hits else (1e9, "none")
        row(d <= CLOSE_MM, f"{label} on 3V3 (esp32-s3-wroom-1.txt:2875): the nearest is {ref} "
            + (f"{parts[ref]['value']} at {d:.1f} mm" if hits else "(none on +3V3)"))
    big = near(vdd, "+3V3", 10e-6)
    print("  every capacitor of 10 uF or more on +3V3 and 3V3_LDO by distance from pin 2: "
          + ", ".join(f"{ref} {parts[ref]['value']} {d:.1f} mm" for d, ref in big))
    ent = [ref for ref in ("C6", "C7") if ref in parts and c(parts, ref) >= 10e-6 * 0.99]
    row(bool(ent), f"10 uF or more at the main power entrance (esp32-s3-hw-design-guidelines.txt:218, 300-303): "
        f"{', '.join(ent)} at U3's output")
    en_r = [ref for ref, p in parts.items() if p["lib"] == "Device:R" and set(p["pins"].values()) == {"EN", "+3V3"}]
    en_c = [ref for ref, p in parts.items() if p["lib"] == "Device:C" and set(p["pins"].values()) == {"EN", "GND"}]
    rv = r(parts, en_r[0]) if en_r else 0.0
    cv = c(parts, en_c[0]) if en_c else 0.0
    row(abs(rv - ESP_EN_RC[0]) < 1 and abs(cv - ESP_EN_RC[1]) < 1e-9,
        f"EN's RC: {en_r[0] if en_r else 'none'} {parts[en_r[0]]['value'] if en_r else ''} and {en_c[0] if en_c else 'none'} "
        f"{parts[en_c[0]]['value'] if en_c else ''} against 'R = 10 kOhm and C = 1 uF' (esp32-s3-hw-design-guidelines.txt:338)")
    if rv and cv:
        t = -rv * cv * math.log(1 - 0.75)
        row(t >= 50e-6, f"EN reaches VIH_nRST (0.75 x VDD, esp32-s3.txt:3761) {t * 1e3:.1f} ms after 3V3, past tSTBL's 50 us "
            "(esp32-s3-wroom-1.txt:931)")
        for ref in en_r + en_c:
            pad = next(q for q in fps[ref].Pads() if q.GetNetname().split("/")[-1] == "EN")
            d = math.hypot(pos(pad)[0] - pos(u4[en])[0], pos(pad)[1] - pos(u4[en])[1])
            print(f"  {ref}'s EN pad {d:.1f} mm from pin {en}")
    l_en = sum(mm(t.GetLength()) for t in b.GetTracks() if t.GetClass() == "PCB_TRACK"
               and t.GetNetname().split("/")[-1] == "EN")
    print(f"  the EN trace is {l_en:.1f} mm long (to J7's pogo pad on the back): 'make the CHIP_PU trace as short as possible' "
          "(esp32-s3-hw-design-guidelines.txt:316) gives no number; a long EN picks up a reset from a nearby edge or ESD "
          "(DFMEA, VER-95's screen)")
    # 3V3 at the module during a Wi-Fi TX burst with the cell at the 3.50 V cutoff (VBAT read at U2's BAT, R17's end)
    import circuit_checks as cc
    i3 = sum(i for _, i, _ in cc.loads_3v3(parts, "peak"))
    i_sys = i3 + 0.1177 + I_LOADS["U7"][1]
    d_vsys, _ = path_drop(pcb, b, "VSYS", [("U2", "10"), ("U2", "11")], ("U3", "1"), i3)
    d_vsys = d_vsys or 0.0
    vd = dn.U3_VDO_1A * i3 / 1.0
    v3 = 3.50 - U2_BAT_OUT_R * i_sys - d_vsys - vd
    print(f"  3V3 during a TX burst at the 3.50 V cutoff: VBAT 3.50 V less U2's battery FET ({U2_BAT_OUT_R} ohm, bq24074.txt:724) "
          f"at the system's {i_sys * 1e3:.0f} mA ({U2_BAT_OUT_R * i_sys * 1e3:.0f} mV), less VSYS's copper from U2's OUT to U3's "
          f"IN at {i3 * 1e3:.0f} mA ({d_vsys * 1e3:.0f} mV, VER-13's solver on the routed board), less U3's dropout "
          f"{vd * 1e3:.0f} mV ({dn.U3_VDO_1A * 1e3:.0f} mV at 1 A, tlv757p.txt:298, scaled with the current) = {v3:.3f} V")
    row(v3 >= ESP_VDD_MIN, f"3V3 {v3:.3f} V at the module in that burst against VDD33's 3.0 V minimum (esp32-s3-wroom-1.txt:1596)")
    return dn.result(ok)


# ---- VER-12 --------------------------------------------------------------------------------------------------
SEED_HOTPLUG = {"R41": "0"}


def hotplug_peak(vs, rc, lc, r41, c1, dt=2e-9, t_end=60e-6):
    """Series cable (rc, lc) from a 5.25 V step into R41 + C1 (U2's IN is the node after the cable). RK4."""
    def f(s):
        i, vc = s
        vin = vc + i * r41
        return ((vs - rc * i - vin) / lc, i / c1)

    s = (0.0, 0.0)
    peak = 0.0
    for _ in range(int(t_end / dt)):
        k1 = f(s)
        k2 = f((s[0] + dt / 2 * k1[0], s[1] + dt / 2 * k1[1]))
        k3 = f((s[0] + dt / 2 * k2[0], s[1] + dt / 2 * k2[1]))
        k4 = f((s[0] + dt * k3[0], s[1] + dt * k3[1]))
        s = (s[0] + dt / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]), s[1] + dt / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]))
        peak = max(peak, s[1] + s[0] * r41)
    return peak


def hotplug(seed=None):
    parts, _ = load(seed)
    r41 = dn.ohms(parts["R41"]["value"]) if parts["R41"]["value"] != "0" else 0.0
    c1 = c(parts, "C1")
    limit = U2_OVP_MIN - 0.2
    worst = (0, None)
    for rc, lc, share in itertools.product(STAND_CABLE["r"], STAND_CABLE["l"], STAND_C1_EFF):
        for rr in (r41 * (1 - R_TOL), r41 * (1 + R_TOL)):
            pk = hotplug_peak(5.25, rc, lc, rr, c1 * share)
            if pk > worst[0]:
                worst = (pk, (rc, lc, share, rr))
    pk, (rc, lc, share, rr) = worst
    ok = pk <= limit
    print(f"a 5.25 V source stepped onto a 2 m cable (loop {STAND_CABLE['r'][0]}-{STAND_CABLE['r'][1]} ohm, "
          f"{STAND_CABLE['l'][0] * 1e6:.0f}-{STAND_CABLE['l'][1] * 1e6:.0f} uH: stand-ins) into R41 {parts['R41']['value']} ohm "
          f"and C1 {parts['C1']['value']} at {STAND_C1_EFF[0] * 100:.0f}-{STAND_C1_EFF[1] * 100:.0f}% of nominal (DC bias, VER-30)")
    print(f"worst VBUS at U2's IN {pk:.2f} V (cable {rc} ohm, {lc * 1e6:.0f} uH, C1 {share * c1 * 1e6:.1f} uF, R41 {rr:.2f} ohm) "
          f"against OVP's minimum {U2_OVP_MIN} V less 0.2 V = {limit:.1f} V (bq24074.txt:699)")
    r43 = r(parts, "R43") * (1 - R_TOL)
    en1 = pk * U2_EN_PD / (U2_EN_PD + r43)
    ok &= en1 <= U2_EN_ABS - 0.3
    print(f"EN1 at that peak {en1:.2f} V through R43 {parts['R43']['value']} and the 285k pull-down (bq24074.txt:512), "
          f"limit {U2_EN_ABS - 0.3:.1f} V (7 V absolute, bq24074.txt:598)")
    r14, r15 = r(parts, "R14") * (1 - R_TOL), r(parts, "R15") * (1 + R_TOL)
    pwr_en = pk * r15 / (r14 + r15)
    ok &= pwr_en <= 5.5
    print(f"PWR_EN at most {pwr_en:.2f} V (R14/R15 with D3's drop taken as 0, C47 {parts['C47']['value']} only slows it), "
          f"limit 5.5 V (U3's EN recommended maximum, tlv757p.txt:208; 6.0 V absolute, 179)")
    return dn.result(ok)


# ---- VER-13 ---------------------------------------------------------------------------------------------------
CU_T = 0.035                     # mm: 1 oz outer copper, JLC's default 2-layer order (no weight is named in assembly.md:10)
CU_RHO = 1.72e-5                 # ohm.mm: copper at 20 C
VIA_PLATE = 0.018                # mm: via barrel plating, a thin figure (to confirm on JLC's capability page, VER-17)
I_PATH = 0.75                    # A: the row's current through the input paths (U2's 475 mA USB500 limit, bq24074.txt:733,
                                 # sits under it)
I_LOADS = {"U3": ("1", 0.62), "J3": (None, 0.12), "U7": ("6", 0.5)}   # A: 3V3's peak in a Wi-Fi burst (VER-31), the panel's 120 mA
                                 # backlight (pin_review/panel.json), the amplifier's peak (stand-in: 0.35 W into 8 ohm
                                 # bridged at 4 V needs about 0.3 A, plus margin)
SEED_COPPER = {"CELL_P": 0.05}   # a 0.05 mm track for the cell (applied to the board in memory)


def ipc2221_amps(width_mm, dt=10.0):
    """IPC-2221's external-layer chart as its formula: I = 0.048 dT^0.44 A^0.725, A in square mils (from memory:
    IPC-2152 allows more on most boards, so this is the stricter reading)."""
    area = (width_mm / 0.0254) * (CU_T / 0.0254)
    return 0.048 * dt ** 0.44 * area ** 0.725


def net_graph(pcb, b, net, narrow=None):
    """Nodes and resistive edges of one net's tracks and vias; pads join every node inside them."""
    mm = pcb.ToMM
    key = lambda x, y, layer: (round(x, 3), round(y, 3), layer)   # noqa: E731
    edges, segs = [], []
    for t in b.GetTracks():
        if t.GetNetname().split("/")[-1] != net:
            continue
        if t.GetClass() == "PCB_VIA":
            x, y = mm(t.GetPosition().x), mm(t.GetPosition().y)
            d = mm(t.GetDrillValue())
            rv = CU_RHO * 1.6 / (math.pi * d * VIA_PLATE)
            edges.append((key(x, y, "F"), key(x, y, "B"), rv, None))
        else:
            w = narrow if narrow else mm(t.GetWidth())
            lay = "F" if t.GetLayer() == pcb.F_Cu else "B"
            a = (mm(t.GetStart().x), mm(t.GetStart().y))
            z = (mm(t.GetEnd().x), mm(t.GetEnd().y))
            segs.append((a, z, w, lay))
    # split a segment where another's end lands on it (T-junctions)
    ends = [(p, s[3]) for s in segs for p in (s[0], s[1])]
    for a, z, w, lay in segs:
        pts = [a, z]
        for p, l2 in ends:
            if l2 != lay or p in (a, z):
                continue
            dx, dy = z[0] - a[0], z[1] - a[1]
            L2 = dx * dx + dy * dy
            if L2 == 0:
                continue
            u = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L2
            if 0 < u < 1 and math.hypot(a[0] + u * dx - p[0], a[1] + u * dy - p[1]) <= w / 2:
                pts.append(p)
        pts.sort(key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
        for p, q in zip(pts, pts[1:]):
            length = math.hypot(q[0] - p[0], q[1] - p[1])
            edges.append((key(*p, lay), key(*q, lay), max(CU_RHO * length / (w * CU_T), 1e-7), w))
    nodes = {n for e in edges for n in e[:2]}
    # merge coincident ends of different widths (within the narrower half-width)
    near = []
    nl = sorted(nodes)
    for i, n in enumerate(nl):
        for m in nl[i + 1:i + 40]:
            if m[2] == n[2] and n != m and math.hypot(n[0] - m[0], n[1] - m[1]) < 0.05:
                near.append((n, m, 1e-7, None))
    edges += near
    pads = {}
    for f in b.GetFootprints():
        for pad in f.Pads():
            if pad.GetNetname().split("/")[-1] != net:
                continue
            pk = ("pad", f.GetReference(), pad.GetNumber())
            pads.setdefault((f.GetReference(), pad.GetNumber()), []).append(pk)
            for n in nodes:
                lay = pcb.F_Cu if n[2] == "F" else pcb.B_Cu
                if pad.IsOnLayer(lay) and pad.HitTest(pcb.VECTOR2I(pcb.FromMM(n[0]), pcb.FromMM(n[1]))):
                    edges.append((pk, n, 1e-6, None))
    return edges, pads


def solve(edges, src, sink, amps):
    """Node voltages with 'amps' injected at the src node and the sink node held at 0 V (Gaussian elimination)."""
    nodes = sorted({n for e in edges for n in e[:2]}, key=str)
    idx = {n: i for i, n in enumerate(nodes)}
    if src not in idx or sink not in idx:
        return None
    n = len(nodes)
    g = [[0.0] * n for _ in range(n)]
    for a, z, res, _ in edges:
        i, j = idx[a], idx[z]
        if i == j:
            continue
        y = 1 / res
        g[i][i] += y
        g[j][j] += y
        g[i][j] -= y
        g[j][i] -= y
    rhs = [0.0] * n
    rhs[idx[src]] = amps
    k = idx[sink]
    keep = [i for i in range(n) if i != k]
    a = [[g[i][j] for j in keep] + [rhs[i]] for i in keep]
    m = len(keep)
    for col in range(m):
        piv = max(range(col, m), key=lambda r_: abs(a[r_][col]))
        if abs(a[piv][col]) < 1e-12:
            return None                                  # not connected through these tracks
        a[col], a[piv] = a[piv], a[col]
        for r_ in range(col + 1, m):
            fct = a[r_][col] / a[col][col]
            if fct:
                for c_ in range(col, m + 1):
                    a[r_][c_] -= fct * a[col][c_]
    x = [0.0] * m
    for r_ in range(m - 1, -1, -1):
        x[r_] = (a[r_][m] - sum(a[r_][c_] * x[c_] for c_ in range(r_ + 1, m))) / a[r_][r_]
    v = {}
    for pos_, i in enumerate(keep):
        v[nodes[i]] = x[pos_]
    v[sink] = 0.0
    return v


def path_drop(pcb, b, net, src_pads, sink_pad, amps, narrow=None):
    edges, pads = net_graph(pcb, b, net, narrow)
    src = ("src", net)
    for p in src_pads:
        for pk in pads.get(p, []):
            edges.append((src, pk, 1e-7, None))
    sink = pads.get(sink_pad, [None])[0]
    v = solve(edges, src, sink, amps)
    if v is None:
        return None, None
    worst = []
    for a, z, res, w in edges:
        if w:
            i = abs(v.get(a, 0) - v.get(z, 0)) / res
            worst.append((i / ipc2221_amps(w), i, w))
    return v[src], max(worst) if worst else (0, 0, 0)


def copper(seed=None):
    parts, _ = load(None)
    pcb, b = dn.board()
    narrow_net = list(seed)[0] if seed else None
    ok = True
    print(f"copper {CU_T * 1e3:.0f} um (1 oz, JLC's default; stand-in until the order options name it), rho {CU_RHO} ohm.mm, "
          f"vias {VIA_PLATE * 1e3:.0f} um plating over 1.6 mm; IPC-2221's external chart for a 10 C rise (formula, from memory)")

    def leg(net, src, sink, amps, label):
        nonlocal ok
        drop, (ratio, i, w) = path_drop(pcb, b, net, src, sink, amps, seed[net] if seed and net == narrow_net else None)
        if drop is None:
            print(f"  {label}: {net} not connected by tracks between {src} and {sink} (a pour or a jumper carries it)")
            ok = False
            return 0.0
        good = ratio <= 1.0
        ok &= good
        print(f"  {label}: {net} {drop * 1e3:.1f} mV at {amps} A ({drop / amps * 1e3:.1f} mohm); its most loaded track "
              f"{w:.2f} mm carries {i:.2f} A, {ratio * 100:.0f}% of IPC-2221's {ipc2221_amps(w):.2f} A for 10 C"
              + ("" if good else ": OVER"))
        return drop

    u2_out = [("U2", "10"), ("U2", "11")]
    print(f"VBUS to VSYS with {I_PATH} A in (the row's current):")
    d_in = leg("VBUS", [("J1", p) for p in ("A4", "A9", "B4", "B9")], ("U2", "13"), I_PATH, "J1 to U2 IN")
    print(f"VBAT to VSYS with {I_PATH} A out of the cell:")
    d_cell = leg("CELL_P", [("J2", "1")], ("JP5", "1"), I_PATH, "J2 to JP5")
    d_bat = leg("VBAT", [("JP5", "2")], ("U2", "2"), I_PATH, "JP5 to U2 BAT")
    print("VSYS from U2's OUT to each load at its own current:")
    d_sys = []
    for ref, (pin, amps) in I_LOADS.items():
        if ref == "J3":
            d_sys.append(leg("VSYS", u2_out, ("JP2", "1"), amps, "U2 OUT to JP2 (LED_A)"))
            leg("LED_A", [("JP2", "2")], next(("J3", p) for p, n in parts["J3"]["pins"].items() if n == "LED_A"), amps,
                "JP2 to J3 (LED_A)")
        elif ref == "U7":
            d_sys.append(leg("VSYS", u2_out, ("JP3", "1"), amps, "U2 OUT to JP3 (AMP_VDD)"))
            leg("AMP_VDD", [("JP3", "2")], ("U7", pin), amps, "JP3 to U7 VDD")
        else:
            d_sys.append(leg("VSYS", u2_out, (ref, pin), amps, f"U2 OUT to {ref} pin {pin}"))
    worst_sys = max(d_sys)
    a = d_in + worst_sys
    bt = d_cell + d_bat + worst_sys
    print(f"VBUS to VSYS copper: {a * 1e3:.1f} mV (limit 50); VBAT to VSYS copper: {bt * 1e3:.1f} mV (limit 30), each "
          "with the worst VSYS branch; U2's own switches are not copper and are not counted")
    ok &= a <= 0.050 and bt <= 0.030
    return dn.result(ok)


# ---- VER-140 (blocked on the PCM) -----------------------------------------------------------------------------
def mating(seed=None):
    parts, _ = load(seed)
    vbat_c = sum(c(parts, p) for p in ("C3",))
    vsys = [p for p, q in parts.items() if q["lib"] == "Device:C" and q["bom"] and
            set(q["pins"].values()) & {"VSYS", "AMP_VDD"}]
    vsys_c = sum(c(parts, p) for p in vsys)
    print(f"plugging the cell (no USB): C3 {vbat_c * 1e6:.1f} uF on VBAT at once, and {', '.join(sorted(vsys))} "
          f"({vsys_c * 1e6:.1f} uF) on VSYS through U2's battery FET (at most {U2_BAT_OUT_R} ohm: 100 mV at 1 A, "
          "bq24074.txt:724)")
    for rs in (0.05, 0.15, 0.35):
        ipk = 4.2 / rs
        tau = (rs + U2_BAT_OUT_R) * vsys_c
        print(f"  series {rs} ohm (cell, PCM, leads; stand-in): peak about {ipk:.0f} A into C3, VSYS charged with tau "
              f"{tau * 1e6:.1f} us; {(vbat_c + vsys_c) * 4.2 * 1e6:.0f} uC in all")
    print("PWR_EN through the edge: no capacitor or diode joins VSYS to PWR_EN with SW24 open (D2 and D3 are "
          "reverse-biased or at 0 V; R13 holds PWR_BTN_RAW at 0 V), so PWR_EN moves only by the open switch's "
          "capacitance into Q1's gate: well under U3's 0.3 V VIL")
    print("BLOCKED: the PCM's short-circuit threshold and delay come from the cell's spec sheet (VER-37); with them, "
          "compare the peak and tau above. No result until then.")
    print("RESULT: BLOCKED")
    return 2


# ---- VER-30: each MLCC's capacitance at its tolerance and its DC-bias curve ----------------------------------------------
def bias_curve(mpn):
    """[(vdc, delta %)] from hardware/ref_design/datasheets/samsung_<mpn>_dcbias.csv (Samsung's own curve), or None."""
    import csv as _csv
    import glob as _glob
    for path in _glob.glob(os.path.join(dn.REF, "datasheets", "samsung_*_dcbias.csv")):
        key = os.path.basename(path)[len("samsung_"):-len("_dcbias.csv")].upper()
        if (mpn or "").upper().startswith(key[:13]):
            rows = [r for r in _csv.reader(ln for ln in open(path, encoding="utf-8") if not ln.startswith("#"))]
            return [(float(a), float(b)) for a, b in rows[1:]], os.path.basename(path)
    return None


def at_bias(curve, v):
    pts = curve[0]
    for (v0, d0), (v1, d1) in zip(pts, pts[1:]):
        if v0 <= v <= v1:
            return d0 + (d1 - d0) * (v - v0) / (v1 - v0)
    return pts[-1][1]


def mlccbias(seed=None):
    parts, _ = load(seed)
    rail = {"VBUS_C1": 5.25, "VSYS": 4.5, "VBAT": 4.2, "3V3_LDO": 3.35, "+3V3": 3.35, "SD_VDD": 3.35, "AMP_VDD": 4.5}
    need = [(("C1",), 1e-6, "U2 IN 1-10 uF (bq24074.txt:520)"),
            (("C2", "C5", "C34"), 4.7e-6, "U2 OUT 4.7-47 uF (bq24074.txt:533), C5 and C34 on VSYS too"),
            (("C3",), 4.7e-6, "U2 BAT 4.7-47 uF (bq24074.txt:501)"),
            (("C5",), 1e-6, "U3 CIN 1 uF or more (tlv757p.txt:210)"),
            (("C6", "C7"), 1e-6, "U3 COUT 1 uF or more (tlv757p.txt:211)"),
            (("C10",), 10e-6, "the module's bulk: 'highly recommended to add a 10 uF capacitor' for the TX bursts "
                              "(esp32-s3-hw-design-guidelines.txt:299-301; the peripheral schematic's 22 uF nominal, "
                              "esp32-s3-wroom-1.txt:2875)"),
            (("C34",), 1e-6, "FM8002A's VDD 1 uF (u7.json p6)")]
    short, blocked = [], []
    for refs, cmin, why in need:
        have, notes, missing = 0.0, [], []
        for ref in refs:
            v = max(rail.get(n, 0) for n in parts[ref]["pins"].values())
            low = c(parts, ref) * (1 - ctol(parts, ref))
            cur = bias_curve(parts[ref]["mpn"])
            if cur is None:
                missing.append(ref)
                have += low
                notes.append(f"{ref} {parts[ref]['mpn']} {low * 1e6:.2f} uF at -tol, no curve")
                continue
            d = at_bias(cur, v)
            eff = low * (1 + d / 100)
            have += eff
            notes.append(f"{ref} {parts[ref]['mpn']} at {v:.2f} V: {low * 1e6:.2f} uF at -tol, {d:+.1f}% "
                         f"({cur[1]}) = {eff * 1e6:.2f} uF")
        good = have >= cmin * 0.999
        state = "ok" if good and not missing else ("SHORT" if not good else "BLOCKED")
        if not good:
            short.append("+".join(refs))
        elif missing:
            blocked.append(f"{'+'.join(refs)} ({', '.join(parts[x]['mpn'] for x in missing)})")
        print(f"  {state:7s} {'+'.join(refs)}: {have * 1e6:.2f} uF against {cmin * 1e6:.2f} uF ({why}); " + "; ".join(notes))
    if short:
        print(f"short at the part's tolerance and DC bias: {', '.join(short)}")
        return dn.result(False)
    if blocked:
        print("BLOCKED: no DC-bias curve on disk for " + "; ".join(blocked) + " (Samsung's component library, "
              "https://weblib.samsungsem.com/mlcc/mlcc-ec-data-sheet.do?partNumber=CL21A226MAQNNNE): its tolerance alone leaves "
              "it over the minimum, the curve decides")
        print("RESULT: BLOCKED")
        return 2
    return dn.result(True)


# ---- VER-206 (39cc07f): a microSD card plugged in while the board runs --------------------------------------------------
STAND_CARD_C = (1e-6, 10e-6)     # F: a card's own VDD capacitance (the SD Physical Layer spec is not on disk: stand-ins)
SEED_SD = {"C60": "100n", "C6": "100n", "C7": "100n"}     # the rail's bulk taken away in memory: the dip must fail


def sdinsert(seed=None):
    """J8's VDD is +3V3 through JP6 (a bridged jumper, no load switch): a card's own capacitance charges from the rail's
    MLCCs at once (charge sharing), then U3 refills it at its current limit. 3V3 must stay at the module's 3.0 V minimum."""
    parts, _ = load(seed)
    rails = {"+3V3", "3V3_LDO", "SD_VDD", "LCD_3V3"}
    tot, lines, nocurve = 0.0, [], []
    for ref, p in sorted(parts.items()):
        if p["lib"] != "Device:C" or not p["bom"] or not (set(p["pins"].values()) & rails) or "GND" not in p["pins"].values():
            continue
        low = c(parts, ref) * (1 - ctol(parts, ref))
        cur = bias_curve(p["mpn"])
        eff = low * (1 + at_bias(cur, 3.35) / 100) if cur else low * 0.5
        if not cur:
            nocurve.append(ref)
        tot += eff
        lines.append(f"{ref} {p['value']} {eff * 1e6:.2f} uF" + ("" if cur else " (no curve: half of -tol, stand-in)"))
    print("the 3V3 rail's capacitance at 3.35 V, each part at its -tolerance and its Samsung DC-bias curve: " + ", ".join(lines)
          + f"; {tot * 1e6:.1f} uF in all")
    v0 = 3.3 * (1 - dn.VREG_TOL)
    icl = 1.2                                   # A: U3's current limit, minimum (tlv757p.txt:267)
    worst = None
    for cc in STAND_CARD_C:
        dip = v0 * cc / (tot + cc)
        t = cc * (v0 - dip) / max(icl - 0.62, 1e-3)       # refilled at the current limit less VER-31's 0.62 A peak load
        print(f"  a card with {cc * 1e6:g} uF (stand-in): 3V3 from {v0:.3f} V falls {dip:.2f} V to {v0 - dip:.2f} V at once, then U3 "
              f"refills it in about {t * 1e6:.0f} us at its 1.2 A limit (tlv757p.txt:267) less the 0.62 A peak load")
        worst = v0 - dip if worst is None else min(worst, v0 - dip)
    cmax = tot * (v0 - ESP_VDD_MIN) / ESP_VDD_MIN
    print(f"  a card's capacitance that takes 3V3 under the module's {ESP_VDD_MIN} V (esp32-s3-wroom-1.txt:1596): {cmax * 1e6:.1f} uF")
    ok = worst >= ESP_VDD_MIN
    print(("  ok    " if ok else "  FAIL  ") + f"3V3 at its lowest {worst:.2f} V for a {STAND_CARD_C[1] * 1e6:g} uF card against {ESP_VDD_MIN} V; "
          "the card's real capacitance and the dip's length on the scope are VER-209's (unit 1)")
    if nocurve:
        print(f"  no DC-bias curve on disk for {', '.join(nocurve)} (CL21A226MAQNNNE: Samsung's library)")
    return dn.result(ok)


# ---- VER-05: the backlight's two current sinks on paper (2026-10-28; the ngspice run in board/sim was DEF-45's) ------
# Each sink: U6 (MCP6002) holds BL_Sk at BL_REF through R58/R59 (10k) with C48/C49 (1n) from its output to its - input;
# R54/R55 (330) into the AO3400A's gate, R56/R57 (100k) to GND, R52/R53 (2.2) from the source to GND, three LEDs from
# LED_A (VSYS through JP2) to the drain. The small-signal model below is a nodal one, solved for the loop gain (broken
# at U6's - input) and stepped in time for a plug-in; every number is design.py's, a datasheet line read by ds.quote(),
# or a stand-in named STAND_BL_* and printed as one.
STAND_BL_GBW = (0.5, 1.5)        # x U6's typical GBW: the datasheet gives a typical only
STAND_BL_P2 = (3.0, 10.0, 30.0)  # U6's second pole, x its GBW (not published)
STAND_BL_RO = (100.0, 300.0, 1000.0)   # ohm: U6's open-loop output resistance (not published)
STAND_BL_PM1 = 70.0              # deg: a (GBW, p2, ro) corner is kept only if it gives U6 this much phase margin at G = +1 with
                                 # the datasheet's 60 pF and 10 kohm load: 20 deg under the 90 deg typical (mcp6001.txt:173, 177)
STAND_BL_CMUL = ((1.0, 1.0), (2.0, 3.0))   # Cgs and Crss multipliers at the sink's VDS of a volt or two (the table is at 15 V)
STAND_BL_RLED = (2.0, 50.0)      # ohm: three parallel LEDs' small-signal resistance at 18 mA each (no I-V slope published)
STAND_BL_N = 1.3                 # weak-inversion slope factor: gm <= ID / (n kT/q)
STAND_BL_LAMBDA = 0.05           # 1/V: the FET's channel-length modulation (gds = lambda x ID)
STAND_BL_PATH_R = 0.1            # ohm: JP2 and the LED_A and LED_K tracks, each (VER-13 does not route the backlight)
STAND_BL_EDGE_X = (1.0, 4.0)     # x the fastest VSYS rise U2's input limit allows (and a slower one)
U2_IIN_MAX = 1.5                 # A: U2's programmable input current limit, its highest (bq24074.txt:738)
STAND_BL_CRSS_RATIO = 0.15       # Crss / Ciss averaged over a 0-4.4 V drain swing (the 15 V table gives 0.079)
STAND_BL_SUBVT = 0.090           # V a decade: the FET's subthreshold slope below VGS(th)
KT_Q = 0.02585                   # V at 300 K
SEED_BL = {"C48": "10p", "C49": "10p"}   # the local loop's capacitor nearly gone: the gate pole sets the phase


def _solve(a, b):
    """Gaussian elimination with partial pivoting, complex or real."""
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for k in range(n):
        p = max(range(k, n), key=lambda i: abs(m[i][k]))
        m[k], m[p] = m[p], m[k]
        for i in range(k + 1, n):
            f = m[i][k] / m[k][k]
            if f:
                for j in range(k, n + 1):
                    m[i][j] -= f * m[k][j]
    x = [0] * n
    for i in range(n - 1, -1, -1):
        x[i] = (m[i][n] - sum(m[i][j] * x[j] for j in range(i + 1, n))) / m[i][i]
    return x


class Sink:
    """One sink's small-signal network. Nodes: FB (U6's - input side), X and Y (U6's two poles), OUT (U6's pin), G, S, D.
    U6 is gm1 (into X: R1, C1 give AOL and the GBW), a unit stage into Y (the second pole), then Y through ro to OUT.
    The FET is gm (G-S) from D to S, with gds, Cgs, Cgd and Cds. The LEDs are rled from D to VSYS."""
    N = ("FB", "X", "Y", "OUT", "G", "S", "D")

    def __init__(self, k):
        self.k = k
        i = {n: j for j, n in enumerate(self.N)}
        n = len(self.N)
        self.i = i
        self.G = [[0.0] * n for _ in range(n)]
        self.C = [[0.0] * n for _ in range(n)]
        self.gm1 = 1e-3                           # U6's input stage reads (REF - FB): REF is 0 small-signal
        G, C = self.G, self.C

        def y(m, a, b, v):
            ia, ib = i.get(a), i.get(b)
            if ia is not None:
                m[ia][ia] += v
            if ib is not None:
                m[ib][ib] += v
            if ia is not None and ib is not None:
                m[ia][ib] -= v
                m[ib][ia] -= v

        def vccs(out_p, out_n, cp, cn, g):        # current g (V(cp) - V(cn)) from out_n into out_p
            for o, so in ((out_p, 1), (out_n, -1)):
                if o not in i:
                    continue
                for cc, sc in ((cp, 1), (cn, -1)):
                    if cc in i:
                        G[i[o]][i[cc]] -= so * sc * g

        r1 = k["aol"] / self.gm1
        y(G, "X", None, 1 / r1)
        y(C, "X", None, self.gm1 / (2 * math.pi * k["gbw"]))
        y(G, "Y", None, 1.0)
        y(C, "Y", None, 1 / (2 * math.pi * k["p2"]))
        vccs("Y", None, "X", None, 1.0)
        y(G, "OUT", None, 1 / k["ro"])
        vccs("OUT", None, "Y", None, 1 / k["ro"])
        y(C, "FB", None, U6_CIN)
        y(G, "FB", "S", 1 / k["rf"])
        y(C, "FB", "OUT", k["cf"])
        y(G, "OUT", "G", 1 / (k["rg"] + Q_RG_INT))
        y(G, "G", None, 1 / k["rpd"])
        y(C, "G", "S", k["cgs"])
        y(C, "G", "D", k["cgd"])
        y(C, "D", "S", k["cds"])
        y(G, "S", None, 1 / k["rs"])
        vccs("S", "D", "G", "S", k["gm"])
        y(G, "D", "S", k["gds"])
        y(G, "D", None, 1 / k["rled"])           # to VSYS: a source node, driven through b in step()

    def loop(self, f):
        """The loop gain at U6's - input: the input stage reads a test source VT instead of FB; T = -V(FB) / VT."""
        n, i, w = len(self.N), self.i, 2j * math.pi * f
        a = [[self.G[r][c] + w * self.C[r][c] for c in range(n)] for r in range(n)]
        b = [0j] * n
        b[i["X"]] = -self.gm1 * 1.0               # gm1 x (0 - VT), VT = 1
        v = _solve(a, b)
        return -v[i["FB"]]

    def step(self, dv, edge, t_end, h=10e-9):
        """VSYS ramps by dv over edge seconds (into D through rled); the source's current deviation over time
        (trapezoidal integration of G v + C v' = b)."""
        n, i = len(self.N), self.i
        a = [row[:] for row in self.G]
        a[i["X"]][i["FB"]] += self.gm1           # the loop closed: gm1 x (0 - FB)
        lhs = [[self.C[r][c] / h + a[r][c] / 2 for c in range(n)] for r in range(n)]
        rhs_m = [[self.C[r][c] / h - a[r][c] / 2 for c in range(n)] for r in range(n)]
        v, t, out = [0.0] * n, 0.0, []
        gl = 1 / self.k["rled"]

        def bvec(t):
            b = [0.0] * n
            b[i["D"]] = gl * dv * min(1.0, t / edge)
            return b
        b0 = bvec(0.0)
        while t < t_end:
            b1 = bvec(t + h)
            rhs = [sum(rhs_m[r][c] * v[c] for c in range(n)) + (b0[r] + b1[r]) / 2 for r in range(n)]
            v = _solve(lhs, rhs)
            t += h
            b0 = b1
            out.append((t, v[i["S"]] / self.k["rs"]))
        return out


def follower_pm(gbw, p2, ro, cl=60e-12, rl=10e3):
    """U6 alone as a unity-gain follower into the datasheet's AC test load (mcp6001.txt:173): its phase margin, deg."""
    w1 = 2 * math.pi * gbw / U6_AOL

    def t(f):
        s_ = 2j * math.pi * f
        zl = 1 / (1 / rl + s_ * cl)
        return U6_AOL / ((1 + s_ / w1) * (1 + s_ / (2 * math.pi * p2))) * zl / (ro + zl)
    lo, hi = 1.0, 1e9
    for _ in range(80):
        mid = math.sqrt(lo * hi)
        if abs(t(mid)) >= 1:
            lo = mid
        else:
            hi = mid
    return 180 + math.degrees(cmath.phase(t(lo)))


def phase_margin(sink, f_lo=1e3, f_hi=50e6, pts=60):
    """(crossover Hz, phase margin deg) of the loop gain, or (None, None) when |T| never crosses 1."""
    fs = [f_lo * (f_hi / f_lo) ** (j / pts) for j in range(pts + 1)]
    ph_prev, last = None, None
    for f in fs:
        t = sink.loop(f)
        ph = math.degrees(cmath.phase(t))
        if ph_prev is not None:
            while ph - ph_prev > 180:
                ph -= 360
            while ph - ph_prev < -180:
                ph += 360
        if last and abs(last[1]) >= 1 > abs(t):
            lo, hi, plo = last[0], f, last[2]
            for _ in range(30):
                mid = math.sqrt(lo * hi)
                tm = sink.loop(mid)
                if abs(tm) >= 1:
                    lo = mid
                else:
                    hi = mid
            pm = math.degrees(cmath.phase(sink.loop(lo)))
            while pm - plo > 180:
                pm -= 360
            while pm - plo < -180:
                pm += 360
            return lo, 180 + pm
        last, ph_prev = (f, t, ph), ph
    return None, None


def backlight(seed=None):
    """VER-05's pass limit on paper: phase margin at every corner, DC at the highest duty, headroom at VSYS 3.5 V and
    Vf 3.2 V, the off states (BL_PWM low, IO9 at reset, 3V3 off at a VSYS edge) and a USB plug-in step while lit."""
    import ds
    parts, _ = load(seed)
    ok = True

    def row(good, text):
        nonlocal ok
        ok &= good
        print(("  ok    " if good else "  FAIL  ") + text)

    for ref, want in (("U6", "MCP6002"), ("Q5", "AO3400A"), ("Q6", "AO3400A")):
        if parts[ref]["value"] != want:
            sys.exit(f"paper_checks backlight: {ref} is {parts[ref]['value']}, not {want}: re-read its datasheet first")
    print("datasheet lines:")
    for q in (ds.quote("mcp6001", 176, "Gain Bandwidth Product", "1.0", "MHz"),
              ds.quote("mcp6001", 177, "Phase Margin", "90", "G = +1"),
              ds.quote("mcp6001", 151, "DC Open-Loop Gain", "88", "112"),
              ds.quote("mcp6001", 135, "Input Offset Voltage", "4.5", "VCM = VSS"),
              ds.quote("mcp6001", 154, "Maximum Output Voltage Swing", "25"),
              ds.quote("mcp6001", 145, "Common-Mode Input Impedance", "6"),
              ds.quote("mcp6001", 146, "Differential Input Impedance", "3"),
              ds.quote("mcp6001", 159, "Supply Voltage", "1.8", "6.0"),
              ds.quote("ao3400a", 85, "Ciss", "630"), ds.quote("ao3400a", 86, "Coss", "75", "VDS=15V"),
              ds.quote("ao3400a", 87, "Crss", "50"), ds.quote("ao3400a", 88, "Gate resistance", "4.5"),
              ds.quote("ao3400a", 73, "Gate Threshold Voltage", "0.65", "1.45"),
              ds.quote("ao3400a", 80, "VGS=2.5V", "48"), ds.quote("ao3400a", 69, "VDS=30V", "1"),
              ds.quote("er-tft035ips-6", 758, "Vf", "120mA", "3.2"),
              ds.quote("hirose_fh12", 56, "Contact resistance", "50m ohms maximum"),
              ds.quote("esp32-s3", 852, "GPIO9", "IE"), ds.quote("esp32-s3", 969, "GPIO9", "Low-level glitch", "60"),
              ds.quote("esp32-s3", 3748, "IIH"), ds.quote("bq24074", 729, "4.3", "4.5")):
        print("  " + q[:150])
    rs = min(r(parts, "R52"), r(parts, "R53")) * (1 - R_TOL)
    r60, r61 = r(parts, "R60"), r(parts, "R61")
    v3 = 3.3
    ref_hi = v3 * (1 + dn.VREG_TOL) * r61 * (1 + R_TOL) / (r60 * (1 - R_TOL) + r61 * (1 + R_TOL))
    dmax, dsrc = dn.max_duty()
    dmin = dn.min_duty()
    i_hi = (ref_hi * dmax + U6_VOS) / rs
    i_nom = v3 * r61 / (r60 + r61) / r(parts, "R52")

    # (a) the loop at every corner
    print(f"(a) loop gain broken at U6's - input; R58/R59 {parts['R58']['value']}, C48/C49 {parts['C48']['value']}, "
          f"R54/R55 {parts['R54']['value']}, R56/R57 {parts['R56']['value']}, R52/R53 {parts['R52']['value']}")
    worst, nocross, n = (999.0, None), [], 0
    amps = [(g, p, ro) for g, p, ro in itertools.product(STAND_BL_GBW, STAND_BL_P2, STAND_BL_RO)
            if follower_pm(U6_GBW * g, U6_GBW * g * p, ro) >= STAND_BL_PM1]
    print(f"  U6's unpublished second pole and output resistance: {len(amps)} of "
          f"{len(STAND_BL_GBW) * len(STAND_BL_P2) * len(STAND_BL_RO)} (GBW, p2, ro) corners give {STAND_BL_PM1:g} deg or "
          "more as a follower into 60 pF and 10 kohm (the datasheet's 90 deg typical less 20); the rest are dropped: "
          + ", ".join(f"{g}x/{p}x/{ro:g} ohm {follower_pm(U6_GBW * g, U6_GBW * g * p, ro):.0f} deg"
                      for g, p, ro in itertools.product(STAND_BL_GBW, STAND_BL_P2, STAND_BL_RO) if (g, p, ro) not in amps))
    for duty in (0.05, dmin, 0.5, dmax):
        idd = max(i_nom * duty, 1e-4)
        gm_hi = idd / (STAND_BL_N * KT_Q)          # the weak-inversion bound; a quarter of it the low corner
        for (gbw, p2, ro), (mg, md), rled, gm, cft in itertools.product(
                amps, STAND_BL_CMUL, STAND_BL_RLED, (gm_hi, gm_hi / 4),
                (1 - ctol(parts, "C48"), 1 + ctol(parts, "C48"))):
            k = dict(aol=U6_AOL, gbw=U6_GBW * gbw, p2=U6_GBW * gbw * p2, ro=ro, rf=r(parts, "R58"),
                     cf=c(parts, "C48") * cft, rg=r(parts, "R54"), rpd=r(parts, "R56"), rs=r(parts, "R52"),
                     cgs=(Q_CISS - Q_CRSS) * mg, cgd=Q_CRSS * md, cds=Q_COSS - Q_CRSS, gm=gm,
                     gds=STAND_BL_LAMBDA * idd, rled=rled)
            fc, pm = phase_margin(Sink(k))
            n += 1
            if fc is None:
                nocross.append((duty, gbw, ro))
                continue
            if pm < worst[0]:
                worst = (pm, dict(duty=duty, gbw_MHz=k["gbw"] / 1e6, p2_x=p2, ro=ro, cgs_x=mg, crss_x=md, rled=rled,
                                  gm=round(gm, 4), cf_nF=round(k["cf"] * 1e9, 3), fc_kHz=round(fc / 1e3, 1)))
    print(f"  {n} corners (duty, U6's GBW, second pole and output resistance, the FET's capacitances and gm, the LEDs' "
          "resistance, C48's tolerance); stand-ins: " + ", ".join(
              f"{a} {b}" for a, b in (("GBW x", STAND_BL_GBW), ("p2 x GBW", STAND_BL_P2), ("ro ohm", STAND_BL_RO),
                                      ("Cgs/Crss x", STAND_BL_CMUL), ("rled ohm", STAND_BL_RLED))))
    if nocross:
        print(f"  no crossover at {len(nocross)} corners (|T| < 1 everywhere: no loop to ring)")
    row(worst[0] >= 45, f"worst phase margin {worst[0]:.1f} deg at {worst[1]} (limit 45 deg)")

    # (b) DC at the highest duty: VER-137's sum
    row(2 * i_hi * 1e3 <= dn.PANEL_MAX_MA,
        f"(b) DC at {dmax:.0%} duty ({dsrc}), 3V3 +{dn.VREG_TOL:.1%}, R61 +1%, R60 -1%, U6 +{U6_VOS * 1e3:.1f} mV, sense -1%: "
        f"{i_hi * 1e3:.1f} mA a sink, {2 * i_hi * 1e3:.1f} mA in all (limit {dn.PANEL_MAX_MA:g} mA, er-tft035ips-6.txt:760)")

    # (c) headroom at VSYS 3.5 V and the panel's maximum Vf
    vsys = 3.5
    vg_max = v3 * (1 - dn.VREG_TOL) - U6_SWING
    rds = AO_RDS_25                                 # at VGS 2.5 V; U6 can drive the gate to vg_max - Vs, above that
    for vf, lim in ((LED_VF_MAX, 0.05), (3.4, 0.10)):
        spare = vsys - vf - 2 * i_hi * (FH12_R + STAND_BL_PATH_R) - i_hi * (FH12_R + STAND_BL_PATH_R) - i_hi * rs - i_hi * rds
        if spare >= 0:
            drop = 0.0
        else:                                       # out of regulation: the LEDs' slope and the sense take the shortfall
            drop = min(1.0, -spare / (STAND_BL_RLED[0] + rs + rds) / i_hi)
        vgs_need = AO_VTH[1] + math.sqrt(2 * i_hi / (1 / (AO_RDS_25 * (2.5 - AO_VTH[1]))))
        txt = (f"(c) VSYS {vsys} V, Vf {vf} V{' (REQ-DSP-07: the newer datasheet; the one on disk says 3.2)' if vf > 3.2 else ''}: "
               f"{spare * 1e3:+.0f} mV to spare after the sense ({i_hi * rs * 1e3:.0f} mV), the FET at {rds * 1e3:.0f} mOhm, "
               f"FH12 {FH12_R * 1e3:.0f} mOhm a contact and {STAND_BL_PATH_R} ohm of path (stand-in); the gate needs "
               f"{i_hi * rs + vgs_need:.2f} V of U6's {vg_max:.2f} V; current drop {drop:.1%} (limit {lim:.0%})")
        if vf == LED_VF_MAX:
            row(drop <= lim, txt)
        else:
            print("  info  " + txt + "; not VER-05's limit, printed for REQ-DSP-07")

    # (d) the off states
    i_off = U6_VOS / rs
    row(2 * i_off * 1e3 < 5,
        f"(d) BL_PWM low, 3V3 on: BL_REF 0 V, U6's offset up to {U6_VOS * 1e3:.1f} mV at VCM = VSS holds "
        f"{i_off * 1e3:.2f} mA a sink, {2 * i_off * 1e3:.2f} mA in all (limit 5 mA); {i_off / dn.LEDS_PER_PIN * 1e3:.2f} mA an "
        "LED is a visible glow: 'BL_PWM low = off' (pinmap.md) holds only for a unit whose offset is negative")
    i_rst = (ESP_IIN * r61 + U6_VOS) / rs
    row(2 * i_rst * 1e3 < 5,
        f"(d) IO9 at reset (input, no pull: esp32-s3.txt:852): BL_REF = {ESP_IIN * 1e9:.0f} nA x R61 = "
        f"{ESP_IIN * r61 * 1e3:.2f} mV; {2 * i_rst * 1e3:.2f} mA in all with the offset; its 60 us low glitch at power-up "
        "(esp32-s3.txt:969) only pulls BL_REF lower")
    # 3V3 off: U6 unpowered, the gate held by R56/R57; a VSYS edge kicks the gate through Crss. Printed, not judged: the
    # result turns on Crss at a volt or two (a figure, not a table line) and on how far the drain moves, so VER-62 measures it
    tau = r(parts, "R56") * Q_CISS
    for dvd, why in ((4.4, "the drain taken to move the whole step"), (4.4 - 2.5, "the drain held about 2.5 V under VSYS "
                                                                               "by the unlit LEDs' knee (stand-in)")):
        vg = dvd * STAND_BL_CRSS_RATIO
        i_kick = 250e-6 * 10 ** ((vg - AO_VTH[0]) / STAND_BL_SUBVT)
        print(f"  info  (d) 3V3 off, a fast VSYS edge to 4.4 V (a cell mating), {why}: the gate kicked to {vg:.2f} V "
              f"(Crss/Ciss {STAND_BL_CRSS_RATIO}, stand-in) against VGS(th) {AO_VTH[0]} V minimum: about "
              f"{2 * i_kick * 1e3:.2g} mA in all at {STAND_BL_SUBVT * 1e3:.0f} mV a decade (stand-in), decaying through "
              f"R56/R57 with tau {tau * 1e6:.0f} us; REQ-DSP-09's 5 mA is VER-62's to measure (cell and USB plugged with "
              "3V3 off, JP2's current on the scope)")
    print(f"  info  3V3 rising: U6 is unspecified under its {U6_VDD_MIN} V minimum supply (mcp6001.txt:159), so the "
          "ramp itself is not on paper: VER-62 watches JP2's current on the scope at power-on with BL_PWM low")

    # (e) a USB plug-in while lit: VSYS from 3.5 V up to U2's 4.5 V at most, no faster than U2's input current limit
    # can charge the VSYS capacitors (each at its -tolerance and DC-bias curve at 4.4 V, half of it with no curve)
    dv = U2_VO_REG[1] - 3.5
    cv = 0.0
    for ref, p in parts.items():
        if p["lib"] == "Device:C" and p["bom"] and "VSYS" in p["pins"].values() and "GND" in p["pins"].values():
            cur = bias_curve(p["mpn"])
            cv += c(parts, ref) * (1 - ctol(parts, ref)) * ((1 + at_bias(cur, 4.4) / 100) if cur else 0.5)
    edge0 = dv * cv / U2_IIN_MAX
    print(f"  (e) VSYS's capacitance at 4.4 V: {cv * 1e6:.1f} uF at least; at U2's {U2_IIN_MAX} A highest input limit "
          f"(bq24074.txt:738) a {dv:.1f} V rise takes {edge0 * 1e6:.1f} us or more")
    idd = i_nom * dmax
    gm = idd / (STAND_BL_N * KT_Q)
    worst = None
    slow = [a for a in amps if a[0] == min(x[0] for x in amps)]
    for edge, (mg, md), rled, (g0, p2, ro), g in itertools.product([edge0 * x for x in STAND_BL_EDGE_X], STAND_BL_CMUL,
                                                                   STAND_BL_RLED, slow, (gm, gm / 4)):
        k = dict(aol=U6_AOL, gbw=U6_GBW * g0, p2=U6_GBW * g0 * p2, ro=ro,
                 rf=r(parts, "R58"), cf=c(parts, "C48"), rg=r(parts, "R54"), rpd=r(parts, "R56"), rs=r(parts, "R52"),
                 cgs=(Q_CISS - Q_CRSS) * mg, cgd=Q_CRSS * md, cds=Q_COSS - Q_CRSS, gm=g, gds=STAND_BL_LAMBDA * idd,
                 rled=rled)
        tr = Sink(k).step(dv, edge, edge + 6e-6, h=max(10e-9, edge / 400))
        hh = tr[1][0] - tr[0][0]
        peak = max(x for _, x in tr)
        at5 = max(x for t, x in tr if t >= 5e-6)
        over = sum(hh for _, x in tr if 2 * (i_hi + x) > dn.PANEL_MAX_MA / 1e3)
        cand = (over, 2 * peak, 2 * at5, dict(edge_us=round(edge * 1e6, 2), cgs_x=mg, crss_x=md, rled=rled, ro=ro,
                                              gm=round(g, 4)))
        if worst is None or cand[:2] > worst[:2]:
            worst = cand
    # REQ-DSP-06: over 120 mA in total for 5 us or less at any event (the time above it, wherever it falls in the ramp)
    row(worst[0] <= 5e-6,
        f"(e) USB plugged in while lit: VSYS +{dv:.1f} V (3.5 V to U2's 4.5 V, bq24074.txt:729) at U6's slowest corners: "
        f"both sinks {worst[1] * 1e3:+.2f} mA at the peak ({2 * i_hi * 1e3 + worst[1] * 1e3:.1f} mA in all on the worst-case "
        f"DC of {2 * i_hi * 1e3:.1f}), {worst[2] * 1e3:+.2f} mA 5 us after the ramp's start; over {dn.PANEL_MAX_MA:g} mA "
        f"for {worst[0] * 1e6:.2f} us in all (limit 5 us, REQ-DSP-06) at {worst[3]}; the deviation is C48's current "
        "through R58 while U6's output ramps to undo the FET's gds, and Crss's kick into the gate on a fast ramp")
    print("  the ngspice run (board/sim/backlight.py, about 40 min, vendor models) is not this check's evidence: it still "
          "hard-codes R61 3.9k (DEF-45); its last run gave 72.5 deg over 720 corners")
    return dn.result(ok)


CHECKS = {"latch": (latch, SEED_LATCH), "usb": (usb, SEED_USB), "audio": (audio, SEED_AUDIO), "module": (module, SEED_MODULE),
          "shelf": (shelf, SEED_SHELF), "ampstate": (ampstate, SEED_AMP), "hotplug": (hotplug, SEED_HOTPLUG),
          "copper": (copper, SEED_COPPER), "mating": (mating, None), "mlccbias": (mlccbias, None),
          "sdinsert": (sdinsert, SEED_SD), "backlight": (backlight, SEED_BL)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"paper_checks: {args[0]} has no seed")
        print(f"SEEDED: {seed}")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
