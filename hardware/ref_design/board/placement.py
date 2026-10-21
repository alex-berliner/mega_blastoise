"""Where every part goes on the board (KiCad frame, mm, degrees CCW).

Positions that the case owns (buttons, locating pins, edge connectors, pinholes,
the battery connector) are read from case/positions.json, which
tools/export_case.py writes from case/case.scad. The RP2350B core is not listed
here: tools/build_pcb.py copies its placement and routing from Raspberry Pi's
minimal design (CORE below), turned so its USB and flash side faces the USB-C.

Height rules (from the case, checked by case/check_fit.py; case/BOARD_OUTLINE.md): under the
glass <= 2.2 mm, in the side bezels and spines <= 3.45 mm, under the decks <= 1.6 mm, round the
clusters <= 0.5 mm, on the back <= 0.5 mm, nothing under the panel's folded FPC tail or a
silicone piece, nothing within 2.8 mm of a screw.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CASE = json.load(open(os.path.join(HERE, "..", "case", "positions.json")))

# ---- RP2350B core: copied from the reference board ------------------------------
CORE_AT = (100.0, 86.5)      # where the reference's U1 (at 100,100) lands: 2.5 mm below the speaker
CORE_Y = CORE_AT[1]          # plug's hole (case spk_slot), so the top pins fan out between the two
CORE_ROT = -90               # reference rotated clockwise: its USB/QSPI side faces +X
CORE = {                     # reference designator -> ours (same nets, same job); the RP2354B
    "U1": "U4", "X1": "X1", "L1": "L2",    # has its flash inside, so the reference's U3 and R10 stay behind
    "C2": "C10", "C12": "C11", "C13": "C12", "C14": "C13", "C15": "C14",
    "C16": "C15", "C17": "C16", "C18": "C17", "C20": "C18", "C21": "C19",
    "C8": "C20", "C10": "C21", "C11": "C22",
    "C5": "C23", "C6": "C24", "C7": "C25", "C9": "C26", "C3": "C27", "C4": "C28",
    "R2": "R20", "R3": "R19", "R7": "R21", "R8": "R22", "R1": "R23", "R6": "R25",
}
CORE_NETS = {                # reference net -> ours, for the copied tracks and zones
    "+1V1": "+1V1", "+3V3": "+3V3", "GND": "GND", "/QSPI_SS": "QSPI_SS",
    "/VREG_AVDD": "VREG_AVDD", "/VREG_LX": "VREG_LX",
    "/XIN": "/MCU/XIN", "/XOUT": "/MCU/XOUT", "Net-(C4-Pad1)": "/MCU/XO",
    "Net-(U1-USB_DM)": "/MCU/USB_DM_MCU", "Net-(U1-USB_DP)": "/MCU/USB_DP_MCU",
}

# ---- everything else ------------------------------------------------------------
b = CASE["buttons"]
s = CASE["pins"]
P = {}                       # ref: (x, y, rot, side)   side "F" or "B"


def put(ref, x, y, rot=0, side="F"):
    P[ref] = (round(x, 3), round(y, 3), rot, side)


for i in range(16):
    put(f"SW{i + 1}", *b[f"SW{i + 1}"])
for ref, xy in CASE["ss_buttons"].items():  # Start/Select, between each seat's clusters at the case's end
    put(ref, *xy)
for i, (x, y) in enumerate(s):
    put(f"H{i + 1}", x, y)
put("FID1", 67.3, 71.0)
put("FID2", 133.2, 141.6)
put("FID3", 66.8, 141.6)

# display: the panel's ZIF connector where its folded FPC reaches (case.scad conn_in), its mouth
# towards seat 1; the backlight driver beside the connector's LED end (pins 48-50)
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
put("R28", 97.6, 99.0, 90)                  # SPI source terminations, next to GPIO38/39
put("R29", 99.1, 99.0, 90)
put("C29", 91.4, 102.6, 0)                  # panel VCI/VDDI, behind connector pins 9-11
put("C30", 93.4, 102.6, 0)
# backlight current sinks: right of the core and above J3's LED end (pins 46-50), clear of the
# bar route.py lays over J3's grounded pins. Channel 2 (Q6, LED_K2) on the left, channel 1 (Q5,
# LED_K1) on the right, each FET beside its sense resistor; the op-amp's parts in two rows
# above them, the op-amp above those
put("R53", 108.6, 102.4, 180)               # sense resistors: BL_S end by the source
put("Q6", 112.2, 102.4, 0)                  # LED_K2 (J3 pins 46, 48)
put("R52", 115.8, 102.4, 180)
put("Q5", 119.4, 102.4, 0)                  # LED_K1 (J3 pins 47, 49)
put("R57", 108.4, 99.4, 0)                  # channel 2: gate pull-down, gate resistor
put("R55", 110.6, 99.4, 0)
put("R60", 112.8, 99.4, 0)                  # PWM into BL_REF
put("R56", 115.0, 99.4, 0)                  # channel 1: gate pull-down, gate resistor
put("R54", 117.2, 99.4, 0)
put("C49", 108.4, 97.8, 0)                  # channel 2: feedback C and R
put("R59", 110.6, 97.8, 0)
put("R61", 112.8, 97.8, 0)
put("C48", 115.0, 97.8, 0)                  # channel 1: feedback C and R
put("R58", 117.2, 97.8, 0)
put("U6", 116.6, 93.4, 0)                   # op-amp half A (left pins) runs LED_K2, half B LED_K1
put("C51", 119.8, 89.4, 0)                  # U6 supply, by pin 8
put("C50", 113.6, 89.4, 0)                  # BL_REF filter

# debug: left bezel
put("SW18", CASE["pinholes"][0][0], CASE["pinholes"][0][1], 90)
put("SW19", CASE["pinholes"][1][0], CASE["pinholes"][1][1], 90)
put("R26", 71.8, 91.0, 90)
put("J5", CASE["swd"][0], CASE["swd"][1], 180)
put("J6", CASE["uart"][0], CASE["uart"][1], 0)      # mouth towards seat 1: turned, it faced J5's back 5 mm away

# edge part on the right: USB-C
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, CASE["usb"]["y"], 90)

# USB and charger: right strip below the USB-C (the parts are 1.75 mm at most, so the
# glass's 2.2 mm allows them too). TP4056 input on its left, cell side on its right;
# then the load-sharing parts in the order the current flows
put("U1", 130.6, 107.6, 90)                 # ESD at the connector
put("C1", 128.0, 111.2, 90)
put("R41", 126.2, 109.8, 90)                # C1's series resistor (hot-plug ringing)
put("R38", 126.8, 73.4, 90)                 # VBUS bleeder, beside R14 where VBUS meets the power latch
put("R1", 126.0, 97.4, 0)
put("R2", 126.0, 102.6, 0)
put("R3", 124.0, 97.4, 0)
put("R4", 124.0, 102.6, 0)
put("D5", 131.6, 111.6, 0)                  # VBUS -> VSYS
put("U2", 130.0, 116.2, 0)                  # TP4056: its thermal pad spreads into the GND pours
put("R5", 125.6, 115.6, 90)                 # PROG
put("R9", 124.4, 115.6, 90)
put("R10", 123.2, 115.6, 90)
put("R39", 125.6, 112.6, 90)                # TEMP divider: VBUS side
put("R40", 124.4, 112.6, 90)
put("TH1", 84.6, 96.0, 90)                 # over the cell (under the board), left of the core and clear of the FPC; 50 mm from U2
# the cell side, round the screw at 131.4, 122
put("C3", 127.0, 120.9, 90)                 # VBAT
put("C2", 125.2, 120.9, 90)                 # VSYS
put("Q2", 131.2, 127.6, 0)                  # cell -> VSYS without USB
put("D7", 126.4, 126.0, 0)                  # Schottky across Q2: carries the unplug gap
put("D6", 131.6, 131.2, 0)                  # Q2's gate from VBUS
put("R6", 128.3, 129.4, 90)
put("C9", 125.8, 129.4, 0)                  # holds VSYS through the switch-over
put("J2", CASE["batt_conn"][0], CASE["batt_conn"][1], 180)     # PicoBlade: mouth towards the panel, contacts behind

# 3V3 LDO right of the chip, next to the core's 3V3 decoupling (routed to it as a track)
put("U3", 129.5, 88.0, 90)                 # right edge, off the core's copper (1.54 mm tall)
put("C5", 132.5, 86.25, 90)                 # VSYS in, beside U3
put("C6", 129.5, 92.0, 90)                  # 3V3 out
put("C7", 131.5, 92.0, 90)

# power switch latch: right strip above the USB-C (the button is seat 1's Y contact)
put("R13", 128.4, 70.0, 90)
put("D2", 131.6, 70.2, 0)
put("R14", 128.4, 73.4, 90)
put("D3", 131.8, 73.4, 180)
put("R15", 134.0, 70.2, 90)
put("Q1", 127.6, 82.6, 0)                  # clear of the screw at 131.4, 78
put("R16", 132.4, 83.8, 0)

# battery sense: next to the ADC pins
put("R17", 104.0, 99.0, 90)
put("R18", 105.4, 99.0, 90)
put("C8", 106.8, 99.0, 90)

# audio: in the left bezel's top corner, over the speaker (which lies under the board's back),
# clear of the speaker plug's hole: that hole sits over the core's top pins, which need the room
# beside it to fan out. J4 (the speaker's plug-in header) stays under seat 2's spine, its mated
# plug and the hole kept clear. The PWM comes in through the filter (R32 C37 R33 C38), Ci and Ri,
# in two rows above U7 under the deck (clear of the cluster's retainer ring); Rf under the chip
put("U7", 74.6, 71.5, 0)                    # 1.75 mm tall: the bezel and frame allow it. FM8002A: 1 SHUTDOWN .. 4 -IN on the left, 5 VO1 .. 8 VO2 right
put("R32", 70.6, 67.6, 0)
put("C37", 70.6, 66.0, 0)
put("R33", 73.2, 67.6, 0)
put("C38", 73.2, 66.0, 0)
put("C39", 75.8, 67.6, 0)                   # Ci
put("R34", 78.4, 67.6, 0)                   # Ri to -IN
put("R35", 74.6, 75.4, 0)                   # Rf, VO1 to -IN
put("C40", 75.8, 66.0, 0)                   # BYPASS
put("R36", 78.4, 66.0, 0)                   # SHUTDOWN pull-up
put("Q3", 67.5, 66.6, 0)                    # SHUTDOWN switch
put("R37", 67.5, 63.8, 0)
put("C34", 80.0, 72.4, 90)                  # VDD
put("C35", 80.0, 69.6, 90)                  # by VDD (pin 6)
put("J4", *CASE["spk_conn"], 0)             # PicoBlade like J2, turned round: mouth towards the panel, the plug's hole past it

# test points on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")

# 2026-10-05, from the decisions sheet: firmware charge control, PWR_HOLD pull-down, D5's
# damping resistor, bring-up jumpers and test pads, hand-solderable spare footprints
put("R42", 127.10, 70.20, 90)
put("R43", 125.15, 117.90, 90)
put("Q4", 121.90, 123.40, 0)                # 10.4 mm from U2
put("R44", 123.40, 119.90, 90)
put("R47", 126.10, 106.85, 0)               # 7.3 mm from D5
put("JP1", 125.75, 88.00, 0)                # 3.8 mm from U3
put("JP2", 117.75, 122.60, 0)               # VSYS to LED_A (J3 pin 50)
put("TP6", 134.00, 82.00, 0)                # PWR_EN, clear of the screw at 131.4, 78
put("TP7", 128.40, 132.00, 0)               # 2.6 mm from R6
put("TP8", 81.60, 67.00, 0)                 # SHUTDOWN, by R36
put("C41", 119.20, 114.00, 90)              # 5.2 mm from R40, beside the FPC
put("C42", 125.60, 100.00, 0)              # between R1/R3 and R2/R4
put("C43", 125.60, 104.40, 0)              # below R2/R4
put("R48", 91.60, 80.60, 0)
put("R49", 91.35, 79.25, 0)
put("R50", 92.60, 76.80, 0)
put("R51", 89.10, 80.20, 0)
put("C47", 125.40, 76.20, 90)              # PWR_EN's slow-rise cap, clear of the screw at 131.4, 78

# 2026-10-19, debug readiness (Alex's review): SWD pads, probe pads and isolation jumpers
put("J7", 76.0, 95.0, 90, "B")              # SWD pogo pads on the back, behind J5
put("TP9", 92.78, CORE_Y + 0.8, 0, "B")             # +1V1: on the core's own +1V1 via (the rail never leaves the core)
put("TP10", 121.00, 99.40, 0)               # BL_REF, by U6 and R61
for i, ref in enumerate(("TP11", "TP12", "TP13", "TP14")):    # LCD SCK, MOSI, CS, DC on the back, under J3's lead-in
    put(ref, 92.2 + 2.54 * i, 110.0, 0, "B")
put("TP15", 121.0, 128.0, 0)                # GND for a scope clip, by the charger
put("TP16", 82.0, 74.4, 0)                  # GND for a scope clip, by the amplifier
put("JP3", 82.6, 70.6, 0)                  # VSYS -> amplifier, by C34/C35
put("JP4", 88.4, 102.6, 0)                 # 3V3 -> panel, beside C29/C30
put("JP5", 107.0, 139.5, 0)                 # cell -> VBAT, beside J2, clear of the cluster
