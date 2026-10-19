"""The reference design as data: every part, its value, footprint, LCSC number
and which net each pin lands on.

This file is the source for the schematic (tools/build_sch.py writes it through
Konnect) and the check that the schematic says what was meant
(tools/check_netlist.py compares KiCad's exported netlist against it).

Net naming:
  POWER nets get KiCad power symbols (GND, +3V3, +1V1, VBUS; VSYS and VBAT use
  power:VCC renamed through its value, which KiCad takes as the net name).
  Any other net used on more than one sheet becomes a global label.
  Nets used on one sheet only become local labels (KiCad prefixes /sheet/).
  None means "no connect": the pin gets an X.

Footprints in battler: are KiCad's own where possible, copied into the
project library only to carry a 3D model this KiCad install lacks (the
connectors and switches) or a land-pattern change noted in its description.
The RP2350B core's 0402 passives use Raspberry Pi's own pads
(tools/lib_from_ref.py): the core's copied tracks and zones were drawn around them.

Symbols are always root symbols. Konnect writes a derived symbol (one that
"extends" another in KiCad's library) without flattening it, and KiCad then
reads it with no pins. So the RP2354B, 2N7002 and AO3401A are drawn with the
symbols they derive from (same pins); the value is the real part.

This is the 2-layer build: TP4056 charger with USB load sharing, ME6211 3V3,
RP2354B (flash inside), PWM audio into an FM8002A, a face button as the power
button, a board thermistor over the cell (no LEDs), every hole >= 0.3 mm, and an ER-TFT035IPS-6 panel
in a ZIF connector.
"""

POWER = {"GND", "+3V3", "+1V1", "VBUS", "VSYS", "VBAT"}

SHEETS = [
    # file, sheet name (also the prefix KiCad puts on local net names)
    ("usb_charger.kicad_sch", "USB_Charger"),
    ("power.kicad_sch", "Power"),
    ("mcu.kicad_sch", "MCU"),
    ("mcu_power.kicad_sch", "MCU_Power"),
    ("debug.kicad_sch", "Debug"),
    ("display.kicad_sch", "Display"),
    ("audio.kicad_sch", "Audio"),
    ("buttons.kicad_sch", "Buttons"),
]

PARTS = []


def part(ref, sheet, lib, value, fp, pins, lcsc=None, mpn=None, mfr=None, rot=0, dnp=False, bom=True):
    PARTS.append(dict(ref=ref, sheet=sheet, lib=lib, value=value, fp=fp, pins=pins, lcsc=lcsc,
                      mpn=mpn, mfr=mfr, rot=rot, dnp=dnp, bom=bom))


# ---- common passives -------------------------------------------------------------
R0402 = "Resistor_SMD:R_0402_1005Metric"
R0603 = "Resistor_SMD:R_0603_1608Metric"
C0402 = "Capacitor_SMD:C_0402_1005Metric"
C0603 = "Capacitor_SMD:C_0603_1608Metric"
C0805 = "Capacitor_SMD:C_0805_2012Metric"
C0402_RPI = "battler:C_0402_1005Metric_RPi"          # the copied core's passives
C0402_RPI_WIDE = "battler:C_0402_1005Metric_RPi_Wide"  # VREG_LX copper runs between these pads
R0402_RPI = "battler:R_0402_1005Metric_RPi"

RES = {  # value: (lcsc, mpn)  all UNI-ROYAL 0402 1% unless noted
    "0": ("C17168", "0402WGF0000TCE"),
    "27": ("C25100", "0402WGF270JTCE"),
    "33": ("C25105", "0402WGF330JTCE"),
    "100": ("C25076", "0402WGF1000TCE"),
    "330": ("C25104", "0402WGF3300TCE"),
    "1k": ("C11702", "0402WGF1001TCE"),
    "1k 0603": ("C21190", "0603WAF1001T5E"),
    "0.1 1206": ("C25334", "1206W4F100LT5E"),
    "2.2 0603": ("C22939", "0603WAF220KT5E"),
    "1.2k": ("C25862", "0402WGF1201TCE"),
    "2.4k": ("C25882", "0402WGF2401TCE"),
    "3.3k": ("C25890", "0402WGF3301TCE"),
    "3.9k": ("C51721", "0402WGF3901TCE"),      # JLC Preferred
    "4.7k": ("C25900", "0402WGF4701TCE"),
    "5.1k": ("C25905", "0402WGF5101TCE"),
    "5.6k": ("C25908", "0402WGF5601TCE"),      # JLC Preferred: no Extended fee
    "10k": ("C25744", "0402WGF1002TCE"),
    "22": ("C25092", "0402WGF220JTCE"),
    "20k": ("C25765", "0402WGF2002TCE"),
    "33k": ("C25779", "0402WGF3302TCE"),
    "47k": ("C25792", "0402WGF4702TCE"),
    "100k": ("C25741", "0402WGF1003TCE"),
    "150k": ("C25755", "0402WGF1503TCE"),      # JLC Preferred
    "470k": ("C25790", "0402WGF4703TCE"),
}
CAP = {  # value: (lcsc, mpn, footprint)
    "15p": ("C1548", "0402CG150J500NT", C0402),
    "1n": ("C1523", "0402B102K500NT", C0402),
    "10n": ("C15195", "CL05B103KB5NNNC", C0402),
    "4.7n": ("C1538", "0402B472K500NT", C0402),
    "100n": ("C1525", "CL05B104KO5NNNC", C0402),
    "220n": ("C16772", "CL05B224KO5NNNC", C0402),
    "1u": ("C52923", "CL05A105KA5NQNC", C0402),
    "4.7u 0402": ("C23733", "CL05A475MP5NRNC", C0402),
    "4.7u": ("C19666", "CL10A475KO8NNNC", C0603),
    "10u": ("C19702", "CL10A106KP8NNNC", C0603),
    "10u 25V": ("C96446", "CL10A106MA8NRNC", C0603),
    "1u 50V": ("C15849", "CL10A105KB8NNNC", C0603),
    "22u 25V": ("C45783", "CL21A226MAQNNNE", C0805),
}


R1206 = "Resistor_SMD:R_1206_3216Metric"
C1206 = "Capacitor_SMD:C_1206_3216Metric"
JUMPER = "Jumper:SolderJumper-2_P1.3mm_Bridged_Pad1.0x1.5mm"


def DNP(ref, sheet, kind, a, b, rot=0):
    """An empty 1206 footprint (big enough to hand-solder) for a part fitted only if
    bring-up asks for it: not in JLC's BOM or placement file."""
    lib, fp = ("Device:R", R1206) if kind == "R" else ("Device:C", C1206)
    part(ref, sheet, lib, "DNP", fp, {"1": a, "2": b}, rot=rot, bom=False)


def JP(ref, sheet, a, b):
    """A solder jumper made with its two pads joined by copper: cut the bridge with a
    knife to open it (to measure a current, or power one part at a time), bridge it
    again with solder."""
    part(ref, sheet, "Jumper:SolderJumper_2_Bridged", "Bridged", JUMPER, {"1": a, "2": b}, bom=False)


def R(ref, sheet, value, a, b, rot=0, fp=R0402):
    lcsc, mpn = RES[value]
    part(ref, sheet, "Device:R", value.split()[0], fp, {"1": a, "2": b}, lcsc, mpn, "UNI-ROYAL", rot)


def C(ref, sheet, value, a, b="GND", rot=0, fp=None):
    lcsc, mpn, size = CAP[value]
    part(ref, sheet, "Device:C", value.split()[0], fp or size, {"1": a, "2": b}, lcsc, mpn,
         "FH" if mpn.startswith("0402CG") else "Samsung", rot)


# =================================================================================
# USB-C and charger
# =================================================================================
S = "usb_charger.kicad_sch"
part("J1", S, "Connector:USB_C_Receptacle_USB2.0_16P", "TYPE-C-31-M-12",
     "battler:USB_C_Receptacle_HRO_TYPE-C-31-M-12",
     {"A1": "GND", "A12": "GND", "B1": "GND", "B12": "GND", "SH": "GND",
      "A4": "VBUS", "A9": "VBUS", "B4": "VBUS", "B9": "VBUS",
      "A5": "CC1", "B5": "CC2", "A6": "USB_DP", "B6": "USB_DP", "A7": "USB_DM", "B7": "USB_DM",
      "A8": None, "B8": None},
     "C165948", "TYPE-C-31-M-12", "Korean Hroparts")
R("R1", S, "5.1k", "CC1", "GND")               # Rd: tells the source we are a sink
R("R2", S, "5.1k", "CC2", "GND")
R("R3", S, "10k", "CC1", "CC1_SENSE")          # ADC taps read the source's current offer;
R("R4", S, "10k", "CC2", "CC2_SENSE")          # 10k keeps a CC short to VBUS off the pin
# REF on 3V3, not VBUS: on VBUS, with no cable in, the MCU's D+ pull-up would feed VBUS
# through the D+ steering diode (to about 2.7 V), hold PWR_EN up and half-close Q2
part("U1", S, "Power_Protection:SRV05-4", "SRV05-4", "Package_TO_SOT_SMD:SOT-23-6",
     {"1": "USB_DP", "3": "USB_DM", "4": None, "6": None, "5": "+3V3", "2": "GND"},
     "C13612", "SRV05-4.TCT", "Semtech")      # ProTek's C85364 ran out at JLC (2026-10-07)
# C1 through R41: the TP4056 datasheet's fix for a ceramic input cap ringing when a live
# source is plugged in (it asks 1.5 ohm; 2.2 is the nearest Basic part). A 2 m cable into a
# board without a cell: VBUS peaks 6.7 V instead of 7.1 (U2's maximum is 8 V), VSYS 6.2 V
# instead of 6.7 (the ME6211's maximum is 6.5 V)
R("R41", S, "2.2 0603", "VBUS", "VBUS_C1", fp=R0603)
C("C1", S, "10u 25V", "VBUS_C1")
# VBUS bleeder. At an unplug C1 holds VBUS up, and Q2's gate (fed from VBUS through D6)
# can only fall as fast as VBUS does: 1k lets Q2 take over from its body diode in about
# 2-3 ms instead of tens of ms. It also keeps D5's reverse leakage (the cell on VSYS)
# from lifting the ME6211's CE through R14/D3: that now takes about 1.3 mA.
# 5 mA, 25-28 mW with USB in: an 0603 (100 mW), 25-30% loaded, in a warm sealed case
R("R38", S, "1k 0603", "VBUS", "GND", fp=R0603)
part("U2", S, "Battery_Management:TP4056-42-ESOP8", "TP4056-42-ESOP8",
     "battler:SOIC-8-1EP_3.9x4.9mm_P1.27mm_EP2.41x3.3mm_ThermalVias_0.3mm",
     {"4": "VBUS", "8": "CHG_CE", "5": "VBAT", "2": "CHG_PROG", "1": "CHG_TEMP",
      "7": "CHG_STAT_N", "6": "CHG_DONE_N", "3": "GND", "9": "GND"},
     "C16581", "TP4056-42-ESOP8", "TOPPOWER")
# cell temperature: the TP4056 charges only while TEMP sits between 45% and 80% of VBUS.
# The 403450 has no NTC wire, so TH1 sits on the board over the cell (which lies under the
# board, 0.5 mm from its back: TH1 reads it through the board and lags it), about 45 mm from U2 (whose 0.65 W while charging would trip it). TH1 is in parallel with R40:
# the window is 1 to 46 C (-3 to 49 C across the TP4056's threshold spread). Fed from VBUS,
# so it draws nothing from the cell. Out of range, CHRG and STDBY both go high (as with no USB)
R("R39", S, "5.6k", "VBUS", "CHG_TEMP")
R("R40", S, "150k", "CHG_TEMP", "GND")
part("TH1", S, "Device:Thermistor_NTC", "10k B3380", R0402, {"1": "CHG_TEMP", "2": "GND"},
     "C77131", "NCP15XH103F03RC", "Murata")
R("R5", S, "3.3k", "CHG_PROG", "GND")          # charge current 1100-1200 V / 3.3k = 333-364 mA (by datasheet
                                               # revision): about 0.5 C for the 403450's ~700 mAh
# CE: firmware can pause charging (a weak USB port, a warm case). R43 pulls CE to VBUS, so
# the charger charges whenever USB is in and nothing drives Q4; CHG_OFF high turns Q4 on
# and CE low. R44 holds Q4's gate low through any processor state (E9: <= 8.2k)
R("R43", S, "100k", "VBUS", "CHG_CE")
part("Q4", S, "Transistor_FET:Q_NMOS_GSD", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "CHG_OFF", "2": "GND", "3": "CHG_CE"}, "C8545", "2N7002", "Changjiang")
R("R44", S, "4.7k", "CHG_OFF", "GND")
R("R9", S, "100k", "CHG_DONE_N", "+3V3")       # STDBY: low when the charge is done
R("R10", S, "100k", "CHG_STAT_N", "+3V3")      # CHRG: low while charging
C("C2", S, "4.7u", "VSYS")
C("C3", S, "10u", "VBAT")
# load sharing: USB feeds the system through D5 while it charges the cell; without USB
# the cell feeds it through Q2. Q2's gate follows VBUS through D6 (R6 pulls it down), so
# Q2 turns on as VBUS drains, which R38 (above) does in a few ms. Until then D7 (below)
# carries VSYS and C9 covers the step (a dip below the chip's supply minimums can crash
# the MCU, and a reset turns the device off)
# R47 damps the ring a live plug-in drives through D5 into the VSYS capacitors: with no cell
# (or a tripped pack) VSYS peaked at about 6.6 V against the FM8002A's 6 V maximum; about
# 5.7 V with it. It costs 50 mV of VSYS at 0.5 A
R("R47", S, "0.1 1206", "VBUS", "VBUS_D5", fp=R1206)
part("D5", S, "Device:D_Schottky", "B5819W", "Diode_SMD:D_SOD-123", {"1": "VSYS", "2": "VBUS_D5"},
     "C8598", "B5819W SL", "Changjiang Electronics Tech (CJ)")
part("D6", S, "Device:D_Schottky", "BAT54WS", "Diode_SMD:D_SOD-323", {"1": "LS_GATE", "2": "VBUS"},
     "C7502694", "BAT54WS", "")
part("Q2", S, "Transistor_FET:Q_PMOS_GSD", "AO3401A", "Package_TO_SOT_SMD:SOT-23",
     {"1": "LS_GATE", "2": "VSYS", "3": "VBAT"}, "C15127", "AO3401A", "Alpha & Omega Semicon")
R("R6", S, "10k", "LS_GATE", "GND")
C("C9", S, "22u 25V", "VSYS")
# D7 carries VSYS from the cell during the unplug gap, until Q2's channel takes over: 0.34 V
# at 0.25 A against the body diode's 0.6. Without it, 3V3 falls under QSPI_IOVDD's 2.97 V
# minimum (the flash inside the RP2354B) on every unplug with the cell below about 3.7 V;
# with it, 3V3 stays above 2.97 V down to about 3.45 V, inside the 3.5 V cutoff. The cost:
# its reverse leakage (about 5 uA at 25 C, 50 uA at 75 C) flows into the cell while USB is in
part("D7", S, "Device:D_Schottky", "B5819W", "Diode_SMD:D_SOD-123", {"1": "VSYS", "2": "VBAT"},
     "C8598", "B5819W SL", "Changjiang Electronics Tech (CJ)")
# J2: Molex PicoBlade, 3.4 mm tall (the 5.5 mm JST PH it replaced needed a pad on the lowered lid,
# 2026-10-19). Cells come with PicoBlade ("MX1.25") plugs; their polarity varies by vendor: check
# the red lead lands on pin 1 (silk +) before plugging in
part("J2", S, "Connector_Generic_MountingPin:Conn_01x02_MountingPin", "53261-0271",
     "Connector_Molex:Molex_PicoBlade_53261-0271_1x02-1MP_P1.25mm_Horizontal",
     {"1": "CELL_P", "2": "GND", "MP": None}, "C177225", "532610271", "Molex")
# JP5: cut it and bridge it with an ammeter to read everything the cell gives or takes (sleep
# current, charge current)
JP("JP5", S, "CELL_P", "VBAT")
part("#FLG01", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VBUS"}, bom=False)
part("#FLG02", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "GND"}, bom=False)
part("#FLG03", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VSYS"}, bom=False)  # fed through D5 or Q2

# =================================================================================
# 3V3 LDO, power switch latch, sensing
# =================================================================================
S = "power.kicad_sch"
part("U3", S, "Regulator_Linear:ME6211C33M5", "ME6211C33M5G", "Package_TO_SOT_SMD:SOT-23-5",
     {"1": "VSYS", "2": "GND", "3": "PWR_EN", "4": None, "5": "3V3_LDO"},
     "C82942", "ME6211C33M5G-N", "MICRONE")
C("C5", S, "10u", "VSYS")
C("C6", S, "10u", "3V3_LDO")
C("C7", S, "10u", "3V3_LDO")
# JP1: cut it at bring-up to check the regulator alone before the rest of the board sees 3V3
JP("JP1", S, "3V3_LDO", "+3V3")
part("#FLG07", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "+3V3"}, bom=False)   # fed through JP1
# the power button is seat 1's Start contact (SW21, on the Buttons sheet): it joins VSYS to PWR_BTN_RAW
R("R13", S, "33k", "PWR_BTN_RAW", "GND")       # 33k, not 100k: about twice the surface leakage (sweat,
                                               # condensation) before the Start contact lifts PWR_EN
part("D2", S, "Diode:BAT54C", "BAT54C", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "PWR_HOLD", "3": "PWR_EN"}, "C37704", "BAT54C,215", "Nexperia")
R("R14", S, "10k", "VBUS", "VBUS_EN")
part("D3", S, "Device:D_Schottky", "BAT54WS", "Diode_SMD:D_SOD-323",
     {"1": "PWR_EN", "2": "VBUS_EN"}, "C7502694", "BAT54WS", "")
R("R15", S, "100k", "PWR_EN", "GND")          # 100k keeps the Start contact's pill-resistance margin (Q1 reads it)
# C47 slows PWR_EN's rise from VBUS (R14, about 1 ms) so a live plug-in can't lift the ME6211's
# CE above its VIN (VSYS, which lags behind R47/D5) plus 0.3 V, its absolute maximum
C("C47", S, "100n", "PWR_EN")
# RP2350-E9 can hold a released pad near 2 V; through D2 that would keep PWR_EN up after a
# crash. An external 4.7k (E9 needs 8.2k or less) holds PWR_HOLD low unless driven
R("R42", S, "4.7k", "PWR_HOLD", "GND")
part("Q1", S, "Transistor_FET:Q_NMOS_GSD", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "GND", "3": "PWR_BTN_N"}, "C8545", "2N7002", "Changjiang")
R("R16", S, "100k", "PWR_BTN_N", "+3V3")
R("R17", S, "470k", "VBAT", "VBAT_SENSE")
R("R18", S, "470k", "VBAT_SENSE", "GND")
C("C8", S, "100n", "VBAT_SENSE")
for i, net in enumerate(["VBUS", "VSYS", "VBAT", "+3V3", "GND"]):
    part(f"TP{i + 1}", S, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# bring-up test pads on the top side: the power latch, the load-sharing switch, the amplifier
for i, net in enumerate(["PWR_EN", "LS_GATE", "AMP_SHDN"]):
    part(f"TP{i + 6}", S, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# 2026-10-19 (Alex's debug-readiness review): the core's 1.1 V rail, the backlight's set point and the
# panel's SPI lines get probe pads too (the panel is the part most likely to need a logic analyser)
for i, (net, sheet) in enumerate([("+1V1", "mcu_power.kicad_sch"), ("BL_REF", "display.kicad_sch"),
                                  ("LCD_SCK", "display.kicad_sch"), ("LCD_MOSI", "display.kicad_sch"),
                                  ("LCD_CS", "display.kicad_sch"), ("LCD_DC", "display.kicad_sch"),
                                  ("GND", S), ("GND", S)]):    # two ground pads on top for scope clips
    part(f"TP{i + 9}", sheet, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# hand-solderable spare filter capacitors, fitted only if bring-up shows noise on these lines
DNP("C41", "usb_charger.kicad_sch", "C", "CHG_TEMP", "GND")
DNP("C42", "usb_charger.kicad_sch", "C", "CC1", "GND")
DNP("C43", "usb_charger.kicad_sch", "C", "CC2", "GND")

# =================================================================================
# RP2350B core (Raspberry Pi RP2350B minimal design, MIT) + flash, boot, debug
# =================================================================================
S = "mcu.kicad_sch"
BOARD_REV = 1             # this board's revision number (0-15), read from GPIO22-25 (R48-R51)
GPIO = {  # GPIO number: net
    # 2 layers: each net's pin faces where it goes, and along an edge the pins are in the
    # order their nets fan out, so routes don't cross (seat 1 below, seat 2 above,
    # charger and USB to the right, the panel's connector below the chip). The pins leave
    # the chip the way Raspberry Pi's layout takes them out (build_pcb.copy_escapes)
    2: "PWR_HOLD", 3: "PWR_BTN_N",
    4: "P2_UP", 5: "P2_DOWN", 6: "P2_LEFT", 7: "P2_RIGHT",
    8: "AUDIO_PWM", 9: "P2_SELECT", 10: "P2_START", 11: "AMP_EN",
    12: "P2_A", 13: "P2_B", 14: "P2_X", 15: "P2_Y",
    17: "CHG_STAT_N", 18: "CHG_DONE_N", 19: "CHG_OFF",
    20: "UART_TX", 21: "UART_RX",
    # left edge: seat 1's cross and the panel's TE leave south as one bus, after RUN
    28: "P1_UP", 29: "P1_DOWN", 30: "P1_LEFT", 31: "P1_RIGHT", 32: "LCD_TE",
    # the panel's pins in its connector's order (TE, CS, DC, SCK) where SPI0 allows;
    # GPIO33 is an SPI0 CSn pin as well, DC is a plain output
    33: "LCD_CS", 36: "LCD_MISO", 37: "LCD_DC", 38: "LCD_SCK_MCU", 39: "LCD_MOSI_MCU",
    # seat 1's Y (an ordinary button again since Start took over power) and Select, between
    # the panel's pins, facing seat 1 (2026-10-19)
    34: "P1_Y", 35: "P1_SELECT",
    # bottom edge, right half, left to right: the panel's reset, seat 1's face buttons,
    # the backlight, then the ADC inputs (GPIO40-47 are the ADC pins) for the USB-C CC
    # lines and the cell (P1 Start is the power button: read on GPIO3)
    40: "LCD_RST", 41: "P1_B", 42: "P1_X", 43: "P1_A", 44: "BL_PWM",
    45: "CC2_SENSE", 46: "CC1_SENSE", 47: "VBAT_SENSE",
    # the board's revision, 4 bits on GPIO22-25 (16 revisions): see R48-R51
    22: "REV0", 23: "REV1", 24: "REV2", 25: "REV3",
}
GPIO_PIN = {0: 77, 1: 78, 2: 79, 3: 80, 4: 1, 5: 2, 6: 3, 7: 4, 8: 6, 9: 7, 10: 8, 11: 9,
            12: 11, 13: 12, 14: 13, 15: 14, 16: 16, 17: 17, 18: 18, 19: 19, 20: 20, 21: 21,
            22: 22, 23: 23, 24: 25, 25: 26, 26: 27, 27: 28, 28: 36, 29: 37, 30: 38, 31: 39,
            32: 40, 33: 42, 34: 43, 35: 44, 36: 45, 37: 46, 38: 47, 39: 48, 40: 49, 41: 52,
            42: 53, 43: 54, 44: 55, 45: 56, 46: 57, 47: 58}
u4 = {str(GPIO_PIN[g]): GPIO.get(g) for g in range(48)}
u4.update({str(p): "+3V3" for p in (5, 15, 24, 29, 41, 50, 60, 76, 59, 64, 68, 69)})
u4.update({str(p): "+1V1" for p in (10, 32, 51, 65)})
u4.update({"61": "VREG_AVDD", "63": "VREG_LX", "62": "GND", "81": "GND",
           "30": "XIN", "31": "XOUT", "33": "SWCLK", "34": "SWDIO", "35": "RUN",
           "66": "USB_DM_MCU", "67": "USB_DP_MCU",
           # the RP2354B's 2 MB flash sits on these inside the package: only the chip
           # select (the BOOTSEL strap) is used outside
           "75": "QSPI_SS", "71": None, "72": None, "74": None, "73": None, "70": None})
part("U4", S, "MCU_RaspberryPi:RP2350B", "RP2354B", "battler:RP2350B_QFN80_RPi", u4,
     "C39843328", "RP2354B", "Raspberry Pi")
P = "mcu_power.kicad_sch"                         # the RP2350B's supply: decoupling + 1V1 switcher
for i in range(10):                              # IOVDD, ADC, VREG_VIN, OTP, QSPI decoupling
    C(f"C{10 + i}", P, "100n", "+3V3", fp=C0402_RPI)
for i in range(3):                               # DVDD
    C(f"C{20 + i}", P, "100n", "+1V1", fp=C0402_RPI)
C("C23", P, "10u", "+3V3")
C("C24", P, "4.7u 0402", "+3V3", fp=C0402_RPI_WIDE)     # VREG_VIN
C("C25", P, "4.7u 0402", "+1V1", fp=C0402_RPI_WIDE)     # VREG output
C("C26", P, "4.7u 0402", "VREG_AVDD", fp=C0402_RPI)
R("R19", P, "33", "+3V3", "VREG_AVDD", fp=R0402_RPI)
part("L2", P, "Device:L", "3.3u", "battler:AOTA-B201610S3R3-101-T", {"1": "+1V1", "2": "VREG_LX"},
     "C42411119", "AOTA-B201610S3R3-101-T", "Abracon")
part("X1", S, "Device:Crystal_GND24", "12MHz", "battler:ABM8-272-T3",
     {"1": "XIN", "3": "XO", "2": "GND", "4": "GND"}, "C20625731", "ABM8-272-T3", "Abracon")
C("C27", S, "15p", "XIN", fp=C0402_RPI)
C("C28", S, "15p", "XO", fp=C0402_RPI)
R("R20", S, "1k", "XOUT", "XO", fp=R0402_RPI)
# 22 ohm: a basic part in Raspberry Pi's own 0402 spot (a 27 ohm 0603 doesn't fit the copied
# layout, and the 27 ohm 0402 is an extended part); 22 ohm is fine for full-speed USB
R("R21", S, "27", "USB_DP", "USB_DP_MCU", fp=R0402_RPI)     # 27: the RP2350 datasheet requires it (Table 2),
R("R22", S, "27", "USB_DM", "USB_DM_MCU", fp=R0402_RPI)     # and Raspberry Pi's own minimal design fits it
R("R23", S, "10k", "+3V3", "QSPI_SS", fp=R0402_RPI)     # the flash's chip select high at power-up
# the revision strap: a 1k to GND on each REV bit makes it 0; a bit left unfitted reads 1
# through the pin's internal pull-up. JLC fits the 0 bits, so a new revision changes the BOM
# and not the copper (a resistor, not a hard tie, if firmware ever drove a pin high)
for b in range(4):
    if (BOARD_REV >> b) & 1:
        part(f"R{48 + b}", S, "Device:R", "DNP", R0402, {"1": f"REV{b}", "2": "GND"}, bom=False)
    else:
        R(f"R{48 + b}", S, "1k", f"REV{b}", "GND")
R("R25", "debug.kicad_sch", "1k", "QSPI_SS", "BOOTSEL", fp=R0402_RPI)
part("SW18", "debug.kicad_sch", "Switch:SW_Push", "BOOTSEL", "battler:SW_SPST_TS-1088-xR020",
     {"1": "BOOTSEL", "2": "GND"}, "C720477", "TS-1088-AR02016", "XKB")
R("R26", "debug.kicad_sch", "1k", "RUN", "RUN_SW")
part("SW19", "debug.kicad_sch", "Switch:SW_Push", "RESET", "battler:SW_SPST_TS-1088-xR020",
     {"1": "RUN_SW", "2": "GND"}, "C720477", "TS-1088-AR02016", "XKB")
part("J5", "debug.kicad_sch", "Connector_Generic_MountingPin:Conn_01x03_MountingPin", "SWD",
     "battler:JST_SH_SM03B-SRSS-TB_1x03-1MP_P1.00mm_Horizontal",
     {"1": "SWCLK", "2": "GND", "3": "SWDIO", "MP": None}, "C160403", "SM03B-SRSS-TB(LF)(SN)", "JST")
part("J6", "debug.kicad_sch", "Connector_Generic_MountingPin:Conn_01x03_MountingPin", "UART",
     "battler:JST_SH_SM03B-SRSS-TB_1x03-1MP_P1.00mm_Horizontal",
     {"1": "UART_RX", "2": "GND", "3": "UART_TX", "MP": None}, "C160403", "SM03B-SRSS-TB(LF)(SN)", "JST")
# J6 follows Raspberry Pi's 3-pin debug connector spec (RP-003139-SP), named from the board's
# side: pin 1 is the board's RX (the probe's TX arrives there), pin 3 the board's TX
# J7: the same SWD in a row of bare pads on the board's back at 2.54 mm, for pogo pins in a jig,
# a clip, or soldered wires: flash and debug without J5's cable or the USB port. Power through
# the 3V3 pad only with the cell out and nothing on USB. BOOTSEL to GND at reset enters the USB
# boot loader; RUN to GND resets
part("J7", "debug.kicad_sch", "Connector_Generic:Conn_01x06", "DEBUG PADS", "battler:Pogo_Pads_1x06_P2.54mm",
     {"1": "+3V3", "2": "SWCLK", "3": "SWDIO", "4": "RUN", "5": "BOOTSEL", "6": "GND"}, bom=False)
part("#FLG05", P, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "+1V1"}, bom=False)
part("#FLG06", P, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VREG_AVDD"}, bom=False)

# =================================================================================
# Display: EastRising ER-TFT035IPS-6 (3.5in IPS 320x480, ILI9488), its FPC in a 50-pin ZIF
# connector; the panel's six parallel backlight LEDs (common anode on VSYS, two cathode pins of
# three LEDs each) sink through two op-amp current regulators, one per cathode pin
# =================================================================================
S = "display.kicad_sch"
PANEL = {1: "LED_A", 2: "LED_K1", 3: "LED_K2",                  # panel pin -> net (datasheet 4.1)
         7: "LCD_3V3", 8: "LCD_3V3", 9: "LCD_3V3",                 # IM0-2 = 111: 4-wire 8-bit SPI
         10: "LCD_RST", 33: "LCD_MISO", 34: "LCD_MOSI",
         35: "LCD_3V3",                                            # RD: parallel read strobe, unused: high
         36: "LCD_SCK", 37: "LCD_DC", 38: "LCD_CS", 39: "LCD_TE",
         40: "LCD_3V3", 41: "LCD_3V3", 42: "LCD_3V3",              # VDDI x2, VCI
         43: "GND", 48: "GND", 49: "GND", 50: "GND"}
PANEL.update({p: "GND" for p in range(11, 33)})                   # VSYNC, HSYNC, DOTCLK, DE, DB17-0: low
PANEL.update({p: None for p in (4, 5, 6, 44, 45, 46, 47)})        # NC, and the touch pins (no touch layer)
# J3 faces seat 1 with its pin 1 on the left (seen from above); the folded FPC's contact 1
# lands on the right, so connector pin j carries panel pin 51 - j
fpc = {str(51 - p): net for p, net in PANEL.items()}
fpc["MP"] = None
part("J3", S, "Connector_Generic_MountingPin:Conn_01x50_MountingPin", "FH12-50S-0.5SH(55)",
     "Connector_FFC-FPC:Hirose_FH12-50S-0.5SH_1x50-1MP_P0.50mm_Horizontal", fpc,
     "C202116", "FH12-50S-0.5SH(55)", "HRS(Hirose)")
R("R28", S, "33", "LCD_SCK_MCU", "LCD_SCK")       # source termination, fitted next to the MCU
R("R29", S, "33", "LCD_MOSI_MCU", "LCD_MOSI")
C("C29", S, "1u", "LCD_3V3")
C("C30", S, "100n", "LCD_3V3")
# JP4: cut it to run the board with the panel unpowered, or to read the panel's logic current.
# With it open, firmware must hold the SPI pins low (the panel's inputs would feed it otherwise)
JP("JP4", S, "+3V3", "LCD_3V3")
part("#FLG08", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "LCD_3V3"}, bom=False)   # fed through JP4
# Backlight: a current sink per cathode pin. U6 (MCP6002) holds each 2.2 ohm sense resistor at
# BL_REF: 0.124 V at 100% duty = 56 mA a pin, 113 mA for the six LEDs (the panel's maximum is
# 120 mA). BL_REF is GPIO44's PWM (BL_PWM) through R60/R61 and C50 (tau 3.75 ms: 0.3% ripple at
# 36.6 kHz), so the LED current is DC: no PWM in the LEDs, nothing for the amplifier to hear.
# R54/R55 (330 ohm) isolate the FET's gate capacitance and C48/C49 with R58/R59 close a local
# loop: 72 deg phase margin or more at every corner simulated (sim/backlight.py). The sink needs
# about 0.13 V across it: regulation holds to a 3.5 V cell up to the panel's 3.2 V Vf. U6 runs
# from 3V3 and R56/R57 hold the gates low, so the backlight is off whenever 3V3 is.
part("U6", S, "battler:MCP6002", "MCP6002", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm",
     {"3": "BL_REF", "2": "BL_FB2", "1": "BL_OUT2", "5": "BL_REF", "6": "BL_FB1", "7": "BL_OUT1",
      "8": "+3V3", "4": "GND"},
     "C7377", "MCP6002T-I/SN", "Microchip Tech")
C("C51", S, "100n", "+3V3")                        # U6 supply
for k, (q, rs, rg, rpd, rf, cf) in ((1, ("Q5", "R52", "R54", "R56", "R58", "C48")),
                                    (2, ("Q6", "R53", "R55", "R57", "R59", "C49"))):
    part(q, S, "Transistor_FET:Q_NMOS_GSD", "AO3400A", "Package_TO_SOT_SMD:SOT-23",
         {"1": f"BL_G{k}", "2": f"BL_S{k}", "3": f"LED_K{k}"}, "C20917", "AO3400A", "Alpha & Omega Semicon")
    R(rs, S, "2.2 0603", f"BL_S{k}", "GND", fp=R0603)   # sense: 2.2 ohm x 56 mA = 0.124 V
    R(rg, S, "330", f"BL_OUT{k}", f"BL_G{k}")
    R(rpd, S, "100k", f"BL_G{k}", "GND")
    R(rf, S, "10k", f"BL_S{k}", f"BL_FB{k}")
    C(cf, S, "1n", f"BL_OUT{k}", f"BL_FB{k}")
R("R60", S, "100k", "BL_PWM", "BL_REF")            # GPIO44 PWM into the reference filter
R("R61", S, "3.9k", "BL_REF", "GND")               # 3.3 V x 3.9 / 103.9 = 0.124 V at 100%
C("C50", S, "1u", "BL_REF")
JP("JP2", S, "VSYS", "LED_A")                      # cut it and bridge with an ammeter to read the LED current

# =================================================================================
# Audio: PWM from the RP2354B, filtered, into an FM8002A bridge amplifier (8 ohm speaker)
# =================================================================================
S = "audio.kicad_sch"
part("U7", S, "battler:FM8002A", "FM8002A", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm",
     {"3": "AMP_BYP", "4": "AMP_INN", "2": "AMP_BYP", "1": "AMP_SHDN",
      "5": "SPK_P", "8": "SPK_N", "6": "AMP_VDD", "7": "GND"},
     "C94625", "FM8002A", "Shenzhen Fuman Elec")
# two RC stages (1k, 4.7 nF) take out the PWM carrier: with the PWM at 8 bits (586 kHz) it
# comes out 49 dB down and the audio band is -3 dB at about 14 kHz loaded. 10-bit PWM
# (146 kHz) would only be 26 dB down: firmware runs 8 bits (dithers for more resolution)
R("R32", S, "1k", "AUDIO_PWM", "AUDIO_F1")
C("C37", S, "4.7n", "AUDIO_F1")
R("R33", S, "1k", "AUDIO_F1", "AUDIO_F2")
C("C38", S, "4.7n", "AUDIO_F2")
C("C39", S, "220n", "AUDIO_F2", "AMP_CI")       # Ci: blocks the PWM's DC; 36 Hz corner with Ri
R("R34", S, "20k", "AMP_CI", "AMP_INN")          # Ri
R("R35", S, "20k", "SPK_P", "AMP_INN")           # Rf: gain 2 x Rf / Ri = 2
C("C40", S, "1u", "AMP_BYP")                     # half-supply reference, as the datasheet's circuit
R("R36", S, "100k", "AMP_SHDN", "AMP_VDD")       # SHUTDOWN high: off unless the MCU turns it on
part("Q3", S, "Transistor_FET:Q_NMOS_GSD", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "AMP_EN", "2": "GND", "3": "AMP_SHDN"}, "C8545", "2N7002", "Changjiang")
R("R37", S, "100k", "AMP_EN", "GND")             # off while the MCU boots
C("C34", S, "10u", "AMP_VDD")
C("C35", S, "100n", "AMP_VDD")
# JP3: cut it to take the amplifier off VSYS (a noisy or shorted amplifier), or to read its current
JP("JP3", S, "VSYS", "AMP_VDD")
part("#FLG09", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "AMP_VDD"}, bom=False)   # fed through JP3
part("J4", S, "Connector_Generic:Conn_01x02", "SPEAKER", "battler:Speaker_Wire_Pads_P4.0mm",
     {"1": "SPK_P", "2": "SPK_N"}, bom=False)


# =================================================================================
# Membrane contacts: DS Lite silicone over bare ENIG pads
# =================================================================================
S = "buttons.kicad_sch"
BUTTONS = ["P1_UP", "P1_DOWN", "P1_LEFT", "P1_RIGHT", "P1_A", "P1_B", "P1_X", "P1_Y",
           "P2_UP", "P2_DOWN", "P2_LEFT", "P2_RIGHT", "P2_A", "P2_B", "P2_X", "P2_Y"]
for i, net in enumerate(BUTTONS):
    part(f"SW{i + 1}", S, "Switch:SW_Push", net.replace("_", " "), "battler:DSL_button_4ring",
         {"1": net, "2": "GND"}, bom=False)
# Start and Select under each seat's DS Lite Start/Select membrane (it comes in the same pad
# kit), between the clusters near the case's end; smaller pills, so a 3-ring contact
SMALL = "battler:DSL_button_3ring"
part("SW20", S, "Switch:SW_Push", "P1 SELECT", SMALL, {"1": "P1_SELECT", "2": "GND"}, bom=False)
# the power button (2026-10-19, was seat 1's Y): this contact joins VSYS to PWR_BTN_RAW (the latch
# on the Power sheet); the MCU reads it as Start through Q1 on GPIO3 (PWR_BTN_N)
part("SW21", S, "Switch:SW_Push", "P1 START / PWR", SMALL, {"1": "PWR_BTN_RAW", "2": "VSYS"}, bom=False)
part("SW22", S, "Switch:SW_Push", "P2 SELECT", SMALL, {"1": "P2_SELECT", "2": "GND"}, bom=False)
part("SW23", S, "Switch:SW_Push", "P2 START", SMALL, {"1": "P2_START", "2": "GND"}, bom=False)

# =================================================================================
# Root sheet: mechanical parts
# =================================================================================
S = "battler.kicad_sch"
for i in range(6):
    part(f"H{i + 1}", S, "Mechanical:MountingHole", "M2", "MountingHole:MountingHole_2.2mm_M2", {}, bom=False)
for i in range(3):
    part(f"FID{i + 1}", S, "Mechanical:Fiducial", "Fiducial", "Fiducial:Fiducial_1mm_Mask2mm", {}, bom=False)


# ---- derived ---------------------------------------------------------------------
def nets():
    """{net: [(ref, pin), ...]} for every named net."""
    out = {}
    for p in PARTS:
        for pin, net in p["pins"].items():
            if net is not None:
                out.setdefault(net, []).append((p["ref"], pin))
    return out


def sheets_of(net):
    return {p["sheet"] for p in PARTS for n in p["pins"].values() if n == net}


def label_kind(net):
    if net in POWER:
        return "power"
    return "global" if len(sheets_of(net)) > 1 else "local"


if __name__ == "__main__":
    n = nets()
    refs = [p["ref"] for p in PARTS]
    assert len(refs) == len(set(refs)), "duplicate reference"
    lonely = [k for k, v in n.items() if len(v) < 2 and not k.startswith("P1_") and not k.startswith("P2_")]
    print(len(PARTS), "parts,", len(n), "nets")
    print("single-node nets:", lonely)
    for k in sorted(n):
        print(f"{k:14} {label_kind(k):6} {len(n[k]):3}  {sorted(sheets_of(k))}")
