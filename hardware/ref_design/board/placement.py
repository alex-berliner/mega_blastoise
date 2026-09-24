"""Where every part goes on the board (KiCad frame, mm, degrees CCW).

Positions that the case owns (buttons, screws, edge connectors, pinholes,
light pipes, the battery connector) are read from case/positions.json, which
tools/export_case.py writes from case/case.scad. The RP2350B core is not listed
here: tools/build_pcb.py copies its placement and routing from Raspberry Pi's
minimal design (CORE below), turned so its USB and flash side faces the USB-C.

Height rules (from the case): under the glass <= 1.2 mm, in the bezels <= 4.6 mm.
Every part taller than a 0603 (SOT-23-6, inductors, switches, connectors) is
therefore in a bezel or between a seat's clusters.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CASE = json.load(open(os.path.join(HERE, "..", "case", "positions.json")))

# ---- RP2350B core: copied from the reference board ------------------------------
CORE_AT = (100.0, 84.0)      # where the reference's U1 (at 100,100) lands
CORE_ROT = -90               # reference rotated clockwise: its USB/QSPI side faces +X
CORE = {                     # reference designator -> ours (same nets, same job)
    "U1": "U4", "U3": "U5", "X1": "X1", "L1": "L2",
    "C2": "C10", "C12": "C11", "C13": "C12", "C14": "C13", "C15": "C14",
    "C16": "C15", "C17": "C16", "C18": "C17", "C20": "C18", "C21": "C19",
    "C8": "C20", "C10": "C21", "C11": "C22",
    "C5": "C23", "C6": "C24", "C7": "C25", "C9": "C26", "C3": "C27", "C4": "C28",
    "R2": "R20", "R3": "R19", "R7": "R21", "R8": "R22", "R1": "R23", "R10": "R24", "R6": "R25",
}
CORE_NETS = {                # reference net -> ours, for the copied tracks and zones
    "+1V1": "+1V1", "+3V3": "+3V3", "GND": "GND",
    "/QSPI_SCLK": "/MCU/QSPI_SCLK", "/QSPI_SD0": "/MCU/QSPI_SD0", "/QSPI_SD1": "/MCU/QSPI_SD1",
    "/QSPI_SD2": "/MCU/QSPI_SD2", "/QSPI_SD3": "/MCU/QSPI_SD3", "/QSPI_SS": "QSPI_SS",
    "/FLASH_SS": "/MCU/FLASH_SS", "/VREG_AVDD": "VREG_AVDD", "/VREG_LX": "VREG_LX",
    "/XIN": "/MCU/XIN", "/XOUT": "/MCU/XOUT", "Net-(C4-Pad1)": "/MCU/XO",
    "Net-(U1-USB_DM)": "/MCU/USB_DM_MCU", "Net-(U1-USB_DP)": "/MCU/USB_DP_MCU",
}

# ---- everything else ------------------------------------------------------------
b = CASE["buttons"]
s = CASE["screws"]
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

# display: the panel's solder-FPC landing where its folded tail reaches (case.scad fpc_front),
# contact 1 at -X; its backlight driver just above the LED end of the row, D1..D6 over
# the six cathode pads in order
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["front_y"], 0)
put("R28", 97.6, 96.6, 90)                  # SPI source terminations, next to GPIO38/39
put("R29", 99.1, 96.6, 90)
put("C29", 84.8, 121.0, 0)                  # panel VDD/IOVCC, at pads 5 and 6
put("C30", 86.8, 121.0, 0)
put("U6", 116.8, 118.6, 180)
put("C31", 113.4, 118.3, 90)                # VIN
put("C32", 120.2, 117.6, 90)                # VOUT (LED anodes)
put("C33", 116.8, 115.4, 180)               # flying C1P-C1N
put("C36", 116.8, 113.9, 180)               # flying C2P-C2N
put("R31", 120.2, 120.4, 90)                # SDI pull-down

# debug: left bezel
put("SW18", CASE["pinholes"][0][0], CASE["pinholes"][0][1], 90)
put("SW19", CASE["pinholes"][1][0], CASE["pinholes"][1][1], 90)
put("R26", 71.8, 91.0, 90)
put("J5", CASE["swd"][0], CASE["swd"][1], 180)
put("J6", CASE["uart"][0], CASE["uart"][1], 180)

# edge parts on the right: USB-C, power button, LEDs
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, CASE["usb"]["y"], 90)
put("SW17", CASE["pwr"]["plunger_x"] - 2.04, CASE["pwr"]["y"], 90)
put("D1", CASE["leds"][0][0], CASE["leds"][0][1], 0)
put("D4", CASE["leds"][1][0], CASE["leds"][1][1], 0)
put("R11", 128.6, 88.0, 90)
put("R27", 128.6, 92.0, 90)

# USB and charger: right bezel below the USB-C
put("U1", 130.6, 107.6, 90)                 # SOT-23-6 is 1.45 mm tall: stays out from under the glass
put("C1", 128.0, 111.2, 90)
put("R1", 126.0, 97.4, 0)
put("R2", 126.0, 102.6, 0)
put("R3", 124.0, 97.4, 0)
put("R4", 124.0, 102.6, 0)
put("U2", 131.4, 114.6, 0)
put("R5", 128.2, 113.9, 90)
put("R6", 128.2, 116.2, 90)
put("C2", 134.0, 118.0, 90)
put("C3", 131.4, 118.4, 0)
put("R7", 124.2, 112.0, 90)
put("R8", 125.4, 112.0, 90)
put("R9", 126.6, 112.0, 90)
put("R10", 124.2, 109.6, 90)
put("J2", CASE["batt_conn"][0], CASE["batt_conn"][1] + 0.6, 180)
put("RT1", 106.0, 114.0, 0)                 # over the cell, clear of the panel's FPC tail and the charger

# 3V3 buck-boost: right bezel. Power pins face down onto the inductor, VINA/EN/PS/FB up;
# input cap on the VIN side, output caps on the VOUT side (route.py draws the loop)
put("U3", 131.2, 123.8, 90)
put("L1", 131.2, 127.3, 180)
put("C5", 133.9, 126.2, 270)
put("C6", 128.3, 125.0, 270)
put("C7", 128.3, 128.0, 270)
put("R12", 130.0, 121.2, 0)
put("C4", 132.4, 121.2, 0)

# power switch latch: right bezel above the button
put("SW17", CASE["pwr"]["plunger_x"] - 2.04, CASE["pwr"]["y"], 90)
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

# audio: above the core, speaker pads on the back over the speaker
put("U7", 100.0, 63.0, 0)
put("R32", 95.4, 63.4, 90)
put("C34", 97.0, 59.0, 0)
put("C35", 100.2, 59.0, 0)
put("FB1", 104.4, 62.8, 90)
put("FB2", 106.2, 62.8, 90)
put("J4", 100.0, 57.5, 0, "B")          # leads run up from the speaker below

# test points on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")
