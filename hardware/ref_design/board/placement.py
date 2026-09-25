"""Where every part goes on the board (KiCad frame, mm, degrees CCW).

Positions that the case owns (buttons, locating pins, edge connectors, pinholes,
the battery connector) are read from case/positions.json, which
tools/export_case.py writes from case/case.scad. The RP2350B core is not listed
here: tools/build_pcb.py copies its placement and routing from Raspberry Pi's
minimal design (CORE below), turned so its USB and flash side faces the USB-C.

Height rules (from the case, checked by case/check_fit.py): under the glass <= 2.2 mm,
in the bezels <= 4.6 mm, nothing under the panel's folded FPC tail or a silicone
piece, nothing over 0.6 mm under the lid's snap hooks.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CASE = json.load(open(os.path.join(HERE, "..", "case", "positions.json")))

# ---- RP2350B core: copied from the reference board ------------------------------
CORE_AT = (100.0, 84.0)      # where the reference's U1 (at 100,100) lands
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
for i, (x, y) in enumerate(s):
    put(f"H{i + 1}", x, y)
put("FID1", 67.3, 71.0)
put("FID2", 133.2, 141.6)
put("FID3", 66.8, 141.6)

# display: the panel's ZIF connector where its folded FPC reaches (case.scad conn_in), its mouth
# towards seat 1; the backlight driver beside the connector's LED end (pins 48-50)
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
put("R28", 97.6, 96.6, 90)                  # SPI source terminations, next to GPIO38/39
put("R29", 99.1, 96.6, 90)
put("C29", 92.3, 112.4, 0)                  # panel VCI/VDDI, behind connector pins 9-11
put("C30", 94.3, 112.4, 0)
put("U6", 120.0, 118.6, 180)                # clear of J3's right mounting tab
put("C31", 116.4, 118.6, 90)                # VIN
put("C32", 123.4, 118.2, 90)                # VOUT (LED anodes)
put("C33", 120.0, 115.4, 180)               # flying C1P-C1N
put("C36", 120.0, 113.9, 180)               # flying C2P-C2N
put("R31", 123.4, 120.4, 90)                # SDI pull-down

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
put("R1", 126.0, 97.4, 0)
put("R2", 126.0, 102.6, 0)
put("R3", 124.0, 97.4, 0)
put("R4", 124.0, 102.6, 0)
put("D5", 131.6, 111.6, 0)                  # VBUS -> VSYS
put("U2", 130.4, 117.4, 0)                  # TP4056: its thermal pad spreads into the GND pours
put("R5", 125.6, 115.6, 90)                 # PROG
put("R9", 124.4, 115.6, 90)
put("R10", 123.2, 115.6, 90)
put("C3", 133.2, 121.6, 0)                  # VBAT
put("C2", 129.0, 121.6, 0)                  # VSYS
put("Q2", 131.2, 125.0, 0)                  # cell -> VSYS without USB
put("D6", 131.2, 128.6, 0)                  # Q2's gate from VBUS
put("R6", 128.4, 127.0, 90)
put("C9", 131.2, 131.4, 0)                  # holds VSYS through the switch-over
put("J2", CASE["batt_conn"][0], CASE["batt_conn"][1] + 0.6, 180)

# 3V3 LDO right of the chip, next to the core's 3V3 decoupling (routed to it as a track)
put("U3", 116.0, 88.0, 90)
put("C5", 113.0, 88.0, 90)                  # VSYS in
put("C6", 119.0, 86.4, 90)                  # 3V3 out
put("C7", 119.0, 89.8, 90)

# power switch latch: right strip above the USB-C (the button is seat 1's Y contact)
put("R13", 128.4, 70.0, 90)
put("D2", 131.6, 70.2, 0)
put("R14", 128.4, 73.4, 90)
put("D3", 131.8, 73.4, 180)
put("R15", 134.0, 70.2, 90)
put("Q1", 129.6, 81.8, 0)
put("R16", 132.4, 83.8, 0)

# battery sense: next to the ADC pins
put("R17", 104.0, 96.6, 90)
put("R18", 105.4, 96.6, 90)
put("C8", 106.8, 96.6, 90)

# audio: above the core, speaker pads on the back over the speaker. The PWM comes in on
# the left through the filter (R32 C37 R33 C38), Ci and Ri; Rf under the chip
put("U7", 100.0, 64.0, 0)                   # FM8002A: 1 SHUTDOWN .. 4 -IN on the left, 5 VO1 .. 8 VO2 right
put("R32", 88.2, 68.6, 0)
put("C37", 88.2, 70.2, 0)
put("R33", 91.0, 68.6, 0)
put("C38", 91.0, 70.2, 0)
put("C39", 93.8, 67.0, 0)                   # Ci
put("R34", 94.6, 65.4, 0)                   # Ri to -IN
put("R35", 100.0, 67.6, 0)                  # Rf, VO1 to -IN
put("C40", 94.6, 63.6, 0)                   # BYPASS
put("R36", 94.6, 61.8, 0)                   # SHUTDOWN pull-up
put("Q3", 91.4, 61.6, 0)
put("R37", 91.4, 59.0, 0)
put("C34", 105.4, 63.0, 90)                 # VDD
put("C35", 104.0, 60.4, 0)
put("J4", 100.0, 57.5, 0, "B")          # leads run up from the speaker below

# test points on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")
