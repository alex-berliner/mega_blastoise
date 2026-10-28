"""The reference design as data: every part, its value, footprint, LCSC number
and which net each pin lands on.

This file is the source for the schematic (tools/build_sch.py writes it) and
the check that the schematic says what was meant (tools/check_netlist.py
compares KiCad's exported netlist against it).

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

Symbols are KiCad's own for the real part where its library has one (the
RP2354B, 2N7002, AO3400A, AO3401A); generic ones (Device:R, Device:D_Schottky)
carry the part number as their value.

This is the 2-layer ESP32-S3 build (2026-10-27, docs/esp32s3_board_proposal.md): an ESP32-S3-WROOM-1U-N16R8
module (16 MB flash, 8 MB octal PSRAM, Wi-Fi on, its antenna a flex antenna on a U.FL cable), BQ24073 power-path
charger, TLV75733P 3V3 (1 A), a microSD slot, PWM audio into an FM8002A, either seat's Start as the power button, a
board thermistor at the cell's hole (no LEDs), every hole >= 0.3 mm, and an ER-TFT035IPS-6 panel on its 8-bit 8080
bus in a ZIF connector. Three 74HC165s read the buttons, both Starts and Selects and the charger's status.
"""

POWER = {"GND", "+3V3", "VBUS", "VSYS", "VBAT"}

SHEETS = [
    # file, sheet name (also the prefix KiCad puts on local net names)
    ("usb_charger.kicad_sch", "USB_Charger"),
    ("power.kicad_sch", "Power"),
    ("mcu.kicad_sch", "MCU"),
    ("storage.kicad_sch", "Storage"),
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
    "1.5k": ("C25867", "0402WGF1501TCE"),
    "2.4k": ("C25882", "0402WGF2401TCE"),
    "3.3k": ("C25890", "0402WGF3301TCE"),
    "3.74k": ("C15172", "0402WGF3741TCE"),
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
    "220k": ("C25767", "0402WGF2203TCE"),      # JLC Preferred
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
    "22u 6V3": ("C59461", "CL10A226MQ8NRNC", C0603),     # 22 uF 6.3 V X5R 0603: the module's bulk cap in the strip beside its 3V3 pin (stock and Basic/Extended unverified)
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
     "C13612", "SRV05-4.TCT", "Semtech")
# U1 (2026-10-22, Alex): Semtech's SRV05-4.TCT (C13612, Extended) again, which JLC stocks, so a
# 30-board order needs no pre-order; hongjiacheng's SRV05-4A (C20615829) held it 2026-10-20 to
# 10-22. Same pinout (I/O on 1, 3, 4, 6; GND 2; REF 5). JLC's footprint for C13612 is KiCad's
# turned 180 degrees (C20615829's was 90): check U1's pin 1 in JLC's placement preview
# C1 through R41 (2.2 ohm): damps a ceramic input cap ringing when a live source is plugged in;
# the BQ24073 asks 1-10 uF on IN, which is rated 28 V absolute with OVP at 6.6 V
R("R41", S, "2.2 0603", "VBUS", "VBUS_C1", fp=R0603)
C("C1", S, "10u 25V", "VBUS_C1")
# VBUS bleeder: lets C1 and PWR_EN fall about 90 ms after an unplug (2026-10-21: 10k, was 1k
# when D5's leakage had to be overcome too)
R("R38", S, "10k", "VBUS", "GND")
# 2026-10-21 (row 16 of Alex's list): the TP4056 and its load sharing (D5, D6, D7, Q2, Q4,
# R6, R39, R40, R47, C9) became a BQ24073 power-path charger: OVP (6.6 V), OUT regulated at
# 4.4 V into VSYS with the system first (DPPM), a 475 mA input limit with VIN-DPM, a 5 h
# safety timer, and a TS input. Its battery FET supplements OUT 40 mV under VBAT: no unplug gap
part("U2", S, "Battery_Management:BQ24073RGT", "BQ24073RGTR",
     "battler:VQFN-16-1EP_3x3mm_RGT0016C",   # TI's land pattern, 0.26 pad to exposed pad (tools/fp_u2.py); its pad vias from route.py (0.3 mm)
     {"13": "VBUS", "10": "VSYS", "11": "VSYS", "2": "VBAT", "3": "VBAT", "16": "CHG_ISET",
      "12": "CHG_ILIM", "6": "CHG_EN1", "5": "GND", "4": "CHG_OFF", "15": "GND", "14": None,
      "1": "CHG_TEMP", "7": "CHG_PG_N", "9": "CHG_STAT_N", "8": "GND", "17": "GND"},
     "C15220", "BQ24073RGTR", "Texas Instruments")
# pin 15 is TD on the '73 (termination enabled when low); KiCad's symbol, shared with the '74,
# names it ITERM. TMR (14) open: the default timers (pre-charge 30 min, fast charge 5 h)
R("R5", S, "3.3k", "CHG_ISET", "GND")          # ICHG = 890 / 3.3k = 270 mA (242-295); 27 mA pre-charge and
                                               # termination
R("R64", S, "3.3k", "CHG_ILIM", "GND")         # ILIM must not float (it would disable charging); used only in
                                               # ILIM mode, which EN1/EN2 don't select
R("R43", S, "220k", "VBUS", "CHG_EN1")         # EN1 high, EN2 low: USB500 (475 mA in, VIN-DPM at 4.5 V); from
                                               # VBUS, so a board without a cell can't start in USB100
# CE (active low) straight from GPIO43: charging on by default (internal 285k pull-down); CHG_OFF
# high stops it. R44 holds the pad low through any processor state (E9: <= 8.2k)
R("R44", S, "4.7k", "CHG_OFF", "GND")
R("R9", S, "100k", "CHG_PG_N", "+3V3")         # PGOOD: low while VIN is valid
R("R10", S, "100k", "CHG_STAT_N", "+3V3")      # CHG: low while charging, 2 Hz on a timer fault
# cell temperature: TS biased at 75 uA, charging between 0.30 V (hot) and 2.1 V (cold). TH1 is a 10k 1% NTC with the highest B JLC stocks
# at 10k, Resistor.Today's TCTR0805F10K0F4460T (B4460, C525375, 0805): at the charger's worst corner charging runs from 4.6 C to 48.2 C
# (the 0402 B3950 it replaced: 1.4 C to 51.5 C). No stocked 10k NTC has the B of 5130 a 45 C hot edge needs (V&V DEF-48, DEC-05).
# The 403450 has no NTC wire, so TH1 sits on the board at the edge of the cell's hole, which the case pushes the cell against; it
# reads the cell through the board's edge and lags it
part("TH1", S, "Device:Thermistor_NTC", "10k B4460", "Resistor_SMD:R_0805_2012Metric", {"1": "CHG_TEMP", "2": "GND"},
     "C525375", "TCTR0805F10K0F4460T", "Resistor.Today")
C("C2", S, "4.7u", "VSYS")
C("C3", S, "10u", "VBAT")
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

# =================================================================================
# 3V3 LDO, power switch latch, sensing
# =================================================================================
S = "power.kicad_sch"
# U3 (2026-10-27, the ESP32-S3 board): TI's TLV75733PDBVR (C485517), 1 A, for the RT9013-33GB (500 mA): the module's Wi-Fi
# transmit peaks at 355 mA and Espressif asks for a 0.5 A supply. Same SOT-23-5 pinout (IN 1, GND 2, EN 3, NC 4, OUT 5), the
# same KiCad symbol. VIN 1.45-5.5 V against VSYS at most 4.5 V; EN high 1.0 V min (6 V absolute with VIN); dropout 425 mV
# at 1 A (3.3 V out); stable from 1 to 200 uF at the output (C6 + C7 + the module's 22 uF = 42 uF)
part("U3", S, "Regulator_Linear:ME6211C33M5", "TLV75733PDBVR", "Package_TO_SOT_SMD:SOT-23-5",
     {"1": "VSYS", "2": "GND", "3": "PWR_EN", "4": None, "5": "3V3_LDO"},
     "C485517", "TLV75733PDBVR", "Texas Instruments")
C("C5", S, "10u", "VSYS")
C("C6", S, "10u", "3V3_LDO")
C("C7", S, "10u", "3V3_LDO")
# JP1: cut it at bring-up to check the regulator alone before the rest of the board sees 3V3
JP("JP1", S, "3V3_LDO", "+3V3")
part("#FLG07", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "+3V3"}, bom=False)   # fed through JP1
# the power button is SW24 (2026-10-28, Alex: a dedicated button in seat 1's deck, Starts are plain buttons; Buttons sheet): it joins VSYS to PWR_BTN_RAW
R("R13", S, "33k", "PWR_BTN_RAW", "GND")       # 33k, not 100k: about twice the surface leakage (sweat,
                                               # condensation) before the Start contact lifts PWR_EN
part("D2", S, "Diode:BAT54C", "BAT54C", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "PWR_HOLD", "3": "PWR_EN"}, "C37704", "BAT54C,215", "Nexperia")
R("R14", S, "47k", "VBUS", "VBUS_EN")      # 47k (was 10k): U3 EN stays inside its 6 V limit up to about 9 V of VBUS (V&V DEC-17)
part("D3", S, "Device:D_Schottky", "BAT54WS", "Diode_SMD:D_SOD-323",
     {"1": "PWR_EN", "2": "VBUS_EN"}, "C7502694", "BAT54WS", "")
R("R15", S, "100k", "PWR_EN", "GND")
# U3's EN (6 V absolute) sees VBUS through R14 and D3, U3's EN1 through R43: 47k and 220k keep it inside its limit up to about 9.1 V
# (EN) and 12.4 V (EN1) of VBUS, with no clamp (V&V DEC-17)
# C47 slows PWR_EN's rise from VBUS (R14 47k, about 40 ms) so a live plug-in couldn't lift the ME6211's
# CE above its VIN plus 0.3 V; the RT9013's EN has no such limit (6 V absolute), C47 stays as a filter
C("C47", S, "1u", "PWR_EN")
# RP2350-E9 can hold a released pad near 2 V; through D2 that would keep PWR_EN up after a
# crash. An external 4.7k (E9 needs 8.2k or less) holds PWR_HOLD low unless driven
R("R42", S, "4.7k", "PWR_HOLD", "GND")
part("Q1", S, "Transistor_FET:2N7002", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "GND", "3": "PWR_BTN_N"}, "C8545", "2N7002", "Changjiang")
R("R16", S, "100k", "PWR_BTN_N", "+3V3")
# (until 2026-10-28 seat 2's Start also turned the device on: D8, R62 and Q7 are gone with the dedicated power button)
# VBAT/2 into a unity buffer (U10, MCP6001 SOT-23-5 from 3V3), so GPIO47's pad leakage (1 uA at most) flows from the op-amp's output and not
# through 235 kohm (0.47 V of VBAT, nine times REQ-BAT-12's window: V&V DEF-40, DEC-07). Off, U10 is unpowered and the pin reads 0 V
R("R17", S, "470k", "VBAT", "VBAT_DIV")
R("R18", S, "470k", "VBAT_DIV", "GND")
C("C8", S, "100n", "VBAT_DIV")
part("U10", S, "Amplifier_Operational:MCP6L91T-EOT", "MCP6001", "Package_TO_SOT_SMD:SOT-23-5",
     {"1": "VBAT_SENSE", "2": "GND", "3": "VBAT_DIV", "4": "VBAT_SENSE", "5": "+3V3"}, "C116490", "MCP6001T-I/OT", "Microchip Tech")
C("C54", S, "100n", "+3V3")
for i, net in enumerate(["VBUS", "VSYS", "VBAT", "+3V3", "GND"]):
    part(f"TP{i + 1}", S, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# bring-up test pads on the top side: the power latch, the charger's TS node (TP7 was the load-
# sharing switch's gate), the amplifier's SHUTDOWN
# (each on the sheet that draws its net)
for i, (net, sheet) in enumerate([("PWR_EN", S), ("CHG_TEMP", "usb_charger.kicad_sch"), ("AMP_SHDN", "audio.kicad_sch")]):
    part(f"TP{i + 6}", sheet, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# 2026-10-19 (Alex's debug-readiness review): the core's 1.1 V rail, the backlight's set point and the
# panel's SPI lines get probe pads too (the panel is the part most likely to need a logic analyser)
# (2026-10-27: the panel's bus is 8080: WR, DC, D0 and TE get the pads that SCK, MOSI, CS and DC had; TP9 is the audio PWM)
for i, (net, sheet) in enumerate([("AUDIO_PWM", "audio.kicad_sch"), ("BL_REF", "display.kicad_sch"),
                                  ("LCD_WR", "display.kicad_sch"), ("LCD_DC", "display.kicad_sch"),
                                  ("LCD_D0", "display.kicad_sch"), ("LCD_TE", "display.kicad_sch"),
                                  ("GND", S), ("GND", S)]):    # two ground pads on top for scope clips
    part(f"TP{i + 9}", sheet, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)
# hand-solderable spare filter capacitors, fitted only if bring-up shows noise on these lines
DNP("C42", "usb_charger.kicad_sch", "C", "CC1", "GND")
DNP("C43", "usb_charger.kicad_sch", "C", "CC2", "GND")

# =================================================================================
# ESP32-S3-WROOM-1U-N16R8 (the board's MCU since 2026-10-27), its USB lines, EN and BOOT
# =================================================================================
S = "mcu.kicad_sch"
BOARD_REV = 2             # this board's revision number, on the silkscreen
# module pin -> net. The module's own pin numbers (datasheet table 3-1); GPIOn is "IOn" on the module. The pins are the proposal's
# (docs/esp32s3_board_proposal.md), see docs/esp32s3_pinmap_diff.md for what differs and why
ESP32 = {
    1: "GND", 2: "+3V3", 3: "EN",
    27: "BOOT",                                  # IO0: strapping pin, low at reset = download mode
    39: "VBAT_SENSE", 38: "CC1_SENSE", 4: "CC2_SENSE",   # IO1, IO2, IO4: ADC1 (works with Wi-Fi on)
    15: "SR_DATA", 5: "SR_PL", 11: "SR_CLK_MCU",          # IO3, IO5, IO18: the three 74HC165s
    6: "LCD_DC", 7: "LCD_RST", 18: "LCD_TE",              # IO6, IO7, IO10
    12: "AUDIO_PWM", 17: "BL_PWM",                        # IO8, IO9: LEDC
    19: "SD_CMD", 20: "SD_CLK", 21: "SD_D0", 22: "SD_D1", 8: "SD_D2", 9: "SD_D3", 10: "SD_CD",   # IO11-IO17
    13: "USB_DM_MCU", 14: "USB_DP_MCU",                   # IO19, IO20: the module's USB
    23: "PWR_HOLD",                                       # IO21
    31: "LCD_D0_MCU", 32: "LCD_D1_MCU", 33: "LCD_D2_MCU", 34: "LCD_D3_MCU", 35: "LCD_D4_MCU",     # IO38-IO42
    37: "LCD_D5_MCU", 36: "LCD_D6_MCU", 24: "LCD_D7_MCU",                                         # IO43 (TXD0), IO44 (RXD0), IO47
    25: "LCD_WR_MCU",                                     # IO48
    26: "AMP_EN", 16: "CHG_OFF",                          # IO45, IO46: strapping pins, held low by R37 and R44
    28: None, 29: None, 30: None,                         # IO35-IO37: the octal PSRAM's
    40: "GND", 41: "GND",
}
part("U4", S, "RF_Module:ESP32-S3-WROOM-1", "ESP32-S3-WROOM-1U-N16R8", "battler:ESP32-S3-WROOM-1U",
     {str(k): v for k, v in ESP32.items()}, "C3013946", "ESP32-S3-WROOM-1U-N16R8", "Espressif Systems")
C("C10", S, "22u 6V3", "+3V3")                   # the module's 3V3 pin: 22 uF (Espressif) with 100 nF; 0603 (CL10A226MQ8NRNC, C59461, 6.3 V X5R) so it fits the strip beside the pin: within 5 mm of it (V&V F1)
C("C11", S, "100n", "+3V3")
# EN: 10k to 3V3 and 1 uF to ground (power-up delay), J7's EN pad pulls it low. Never floating
R("R23", S, "10k", "+3V3", "EN")
C("C12", S, "1u", "EN")
# USB D-/D+: 22 ohm in series close to the module and spare pads for capacitors to ground (not fitted), as the design guidelines
# ask (1.3.13); the line goes on to the ESD array U1 and the receptacle. The module pulses D+ at power-up: PWR_HOLD is the
# thing that must be stable, and it is (R42)
R("R21", S, "22", "USB_DP", "USB_DP_MCU")
R("R22", S, "22", "USB_DM", "USB_DM_MCU")
part("C13", S, "Device:C", "DNP", C0402, {"1": "USB_DP_MCU", "2": "GND"}, bom=False)
part("C14", S, "Device:C", "DNP", C0402, {"1": "USB_DM_MCU", "2": "GND"}, bom=False)
# J7: BOOT (IO0) and EN on bare pads on the board's back with 3V3 and GND (pogo jig, clip); power through its 3V3 pad only with the
# cell out and nothing on USB. BOOT low at reset enters the ROM's download mode. The BOOT and RESET buttons (SW18, SW19) under pinholes
# are not on this board: the microSD slot took the left bezel and the right one has no room (progress.md, 2026-10-28)
part("J7", "debug.kicad_sch", "Connector_Generic:Conn_01x04", "DEBUG PADS", "battler:Pogo_Pads_1x04_P2.54mm",
     {"1": "+3V3", "2": "EN", "3": "BOOT", "4": "GND"}, bom=False)

# =================================================================================
# Display: EastRising ER-TFT035IPS-6 (3.5in IPS 320x480, ILI9488), its FPC in a 50-pin ZIF
# connector, on its 8-bit 8080 bus since 2026-10-27 (docs/display_parallel_proposal.md); the panel's six parallel
# backlight LEDs (common anode on VSYS, two cathode pins of three LEDs each) sink through two op-amp current regulators,
# one per cathode pin
# =================================================================================
S = "display.kicad_sch"
PANEL = {1: "LED_A", 2: "LED_K1", 3: "LED_K2",                  # panel pin -> net (datasheet 4.1)
         7: "LCD_3V3", 8: "LCD_3V3", 9: "GND",                      # IM0, IM1 high, IM2 low: 011 = 8080 8-bit
         10: "LCD_RST",
         33: None,                                                 # SDO: left open when not in use
         34: "GND",                                                # SDA: fixed to DGND when not in use
         35: "LCD_3V3",                                            # RD (RDX): read strobe, unused: high
         36: "LCD_WR", 37: "LCD_DC",                               # WRX, D/CX
         38: "GND",                                                # CSX tied low: the panel is always selected
         39: "LCD_TE",
         40: "LCD_3V3", 41: "LCD_3V3", 42: "LCD_3V3",              # VDDI x2, VCI
         43: "GND", 48: "GND", 49: "GND", 50: "GND"}
PANEL.update({p: "GND" for p in range(11, 25)})                   # VSYNC, HSYNC, DOTCLK, DE, DB17-DB8: low
PANEL.update({32 - k: f"LCD_D{k}" for k in range(8)})             # DB0 (pin 32) .. DB7 (pin 25)
PANEL.update({p: None for p in (4, 5, 6, 44, 45, 46, 47)})        # NC, and the touch pins (no touch layer)
# J3 faces seat 1 with its pin 1 on the left (seen from above); the folded FPC's contact 1
# lands on the right, so connector pin j carries panel pin 51 - j
fpc = {str(51 - p): net for p, net in PANEL.items()}
fpc["MP"] = None
part("J3", S, "Connector_Generic_MountingPin:Conn_01x50_MountingPin", "FH12-50S-0.5SH(55)",
     "Connector_FFC-FPC:Hirose_FH12-50S-0.5SH_1x50-1MP_P0.50mm_Horizontal", fpc,
     "C202116", "FH12-50S-0.5SH(55)", "HRS(Hirose)")
# 33 ohm in series on D0-D7 and WR, fitted next to the module (source termination)
for k in range(8):
    R(f"R{70 + k}", S, "33", f"LCD_D{k}_MCU", f"LCD_D{k}")
R("R78", S, "33", "LCD_WR_MCU", "LCD_WR")
C("C29", S, "1u", "LCD_3V3")
C("C30", S, "100n", "LCD_3V3")
# JP4: cut it to run the board with the panel unpowered, or to read the panel's logic current.
# With it open, firmware must hold the bus pins low (the panel's inputs would feed it otherwise)
JP("JP4", S, "+3V3", "LCD_3V3")
part("#FLG08", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "LCD_3V3"}, bom=False)   # fed through JP4
# Backlight: a current sink per cathode pin. U6 (MCP6002) holds each 2.2 ohm sense resistor at
# BL_REF: 0.124 V at 100% duty = 56 mA a pin, 113 mA for the six LEDs (the panel's maximum is
# 120 mA). BL_REF is GPIO28's PWM (BL_PWM) through R60/R61 and C50 (tau 3.75 ms: 0.3% ripple at
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
    part(q, S, "Transistor_FET:AO3400A", "AO3400A", "Package_TO_SOT_SMD:SOT-23",
         {"1": f"BL_G{k}", "2": f"BL_S{k}", "3": f"LED_K{k}"}, "C20917", "AO3400A", "Alpha & Omega Semicon")
    R(rs, S, "2.2 0603", f"BL_S{k}", "GND", fp=R0603)   # sense: 2.2 ohm x 56 mA = 0.124 V
    R(rg, S, "330", f"BL_OUT{k}", f"BL_G{k}")
    R(rpd, S, "100k", f"BL_G{k}", "GND")
    R(rf, S, "10k", f"BL_S{k}", f"BL_FB{k}")
    C(cf, S, "1n", f"BL_OUT{k}", f"BL_FB{k}")
R("R60", S, "100k", "BL_PWM", "BL_REF")            # GPIO28 PWM into the reference filter
R("R61", S, "3.74k", "BL_REF", "GND")              # 3.3 V x 3.74 / 103.74 = 0.119 V at 100% (117.7 mA worst case; 3.9k gave 122.4 over the panel's 120: V&V DEF-39)
C("C50", S, "1u", "BL_REF")
JP("JP2", S, "VSYS", "LED_A")                      # cut it and bridge with an ammeter to read the LED current

# =================================================================================
# Audio: PWM from the ESP32-S3 (LEDC), filtered, into an FM8002A bridge amplifier (8 ohm speaker)
# =================================================================================
S = "audio.kicad_sch"
part("U7", S, "battler:FM8002A", "FM8002A", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm",
     {"3": "AMP_BYP", "4": "AMP_INN", "2": "AMP_BYP", "1": "AMP_SHDN",
      "5": "SPK_P", "8": "SPK_N", "6": "AMP_VDD", "7": "GND"},
     "C94625", "FM8002A", "Shenzhen Fuman Elec")
# two RC stages (1.5k, 4.7 nF) take out the PWM carrier: the ESP32-S3's LEDC runs from the 80 MHz APB clock, so 8 bits is
# 312.5 kHz (it was 586 kHz on the RP2354B). The old 1k filter would be only 39 dB down there; 1.5k is 46 dB down, with the audio
# band -3 dB at 6.7 kHz (a 15 x 10 mm speaker has little above that; sim/audio_filter.py). 7 bits at 625 kHz would be 58 dB down
R("R32", S, "1.5k", "AUDIO_PWM", "AUDIO_F1")
C("C37", S, "4.7n", "AUDIO_F1")
R("R33", S, "1.5k", "AUDIO_F1", "AUDIO_F2")
C("C38", S, "4.7n", "AUDIO_F2")
C("C39", S, "220n", "AUDIO_F2", "AMP_CI")       # Ci: blocks the PWM's DC; 36 Hz corner with Ri
R("R34", S, "20k", "AMP_CI", "AMP_INN")          # Ri
R("R35", S, "20k", "SPK_P", "AMP_INN")           # Rf: gain 2 x Rf / Ri = 2
C("C40", S, "1u", "AMP_BYP")                     # half-supply reference, as the datasheet's circuit
R("R36", S, "100k", "AMP_SHDN", "AMP_VDD")       # SHUTDOWN high: off unless the MCU turns it on
part("Q3", S, "Transistor_FET:2N7002", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "AMP_EN", "2": "GND", "3": "AMP_SHDN"}, "C8545", "2N7002", "Changjiang")
R("R37", S, "100k", "AMP_EN", "GND")             # off while the MCU boots
C("C34", S, "10u", "AMP_VDD")
C("C35", S, "100n", "AMP_VDD")
# JP3: cut it to take the amplifier off VSYS (a noisy or shorted amplifier), or to read its current
JP("JP3", S, "VSYS", "AMP_VDD")
part("#FLG09", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "AMP_VDD"}, bom=False)   # fed through JP3
# J4 (2026-10-20, Alex: no soldering at assembly): the speaker (INGHAi GSPK151035PN) plugs in. Since 2026-10-27 J4 is a JST SH
# 2-pin header (1.0 mm, SM02B-SRSS-TB, C160402, the 2-pin of J5's family) and the speaker's leads carry an SH plug: the cell's
# PicoBlade plug (J2, 1.25 mm) cannot enter it (V&V DF-170, DEC-33). The speaker lies in a hole in the board beside the
# screen (placement.CUTOUTS), J4 just above it
part("J4", S, "Connector_Generic_MountingPin:Conn_01x02_MountingPin", "SM02B-SRSS-TB",
     "Connector_JST:JST_SH_SM02B-SRSS-TB_1x02-1MP_P1.00mm_Horizontal",
     {"1": "SPK_P", "2": "SPK_N", "MP": None}, "C160402", "SM02B-SRSS-TB(LF)(SN)", "JST")


# =================================================================================
# microSD (2026-10-27): a push-push slot, 4-bit bus plus card detect
# =================================================================================
S = "storage.kicad_sch"
# Hirose DM3AT-SF-PEJM5 (C114218): push-push, 8 contacts and a normally open card-detect switch that closes to ground when
# a card is in, 1.68 mm tall. The module's SDMMC host takes any GPIO (the matrix), so the bus follows the module's pin order
part("J8", S, "Connector:Micro_SD_Card_Det_Hirose_DM3AT", "DM3AT-SF-PEJM5", "battler:microSD_HC_Hirose_DM3AT-SF-PEJM5",
     {"1": "SD_D2", "2": "SD_D3", "3": "SD_CMD", "4": "SD_VDD", "5": "SD_CLK", "6": "GND", "7": "SD_D0", "8": "SD_D1",
      "9": "SD_CD", "10": "GND", "SH": "GND"}, "C114218", "DM3AT-SF-PEJM5", "HRS(Hirose)")
# 10k pull-ups on CMD and D0-D3 (the SD spec's idle levels; D3 doubles as the card's SPI chip select) and on the detect line
for k, net in enumerate(("SD_CMD", "SD_D0", "SD_D1", "SD_D2", "SD_D3", "SD_CD")):
    R(f"R{79 + k}", S, "10k", "+3V3", net)
# the card's supply through JP6: cut it to power the card from outside or to read its current; 10 uF and 100 nF beside the slot
JP("JP6", S, "+3V3", "SD_VDD")
part("#FLG10", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "SD_VDD"}, bom=False)     # fed through JP6
C("C60", S, "10u", "SD_VDD")
C("C61", S, "100n", "SD_VDD")

# =================================================================================
# Membrane contacts: DS Lite silicone over bare ENIG pads
# =================================================================================
S = "buttons.kicad_sch"
# 2026-10-21: each seat's D-pad and A/B/X/Y (8 contacts to GND, low = pressed) come in through a
# 74HC165 at that seat, its inputs pulled up by two 10k arrays. The two are chained (seat 2's
# Q7 into seat 1's DS) on one load and one clock, so only SR_PL, SR_CLK, the chain and 3V3
# cross the board beside the cell's hole. A read: SR_PL low then high (parallel load), then 16
# clocks; SR_DATA shows P1 Y first (U8's D7), then P1 X, B, A, RIGHT, LEFT, DOWN, UP, then
# seat 2 in the same order; 1 = released. Start and Select stay on their own GPIOs
BUTTONS = ["P1_UP", "P1_DOWN", "P1_LEFT", "P1_RIGHT", "P1_A", "P1_B", "P1_X", "P1_Y",
           "P2_UP", "P2_DOWN", "P2_LEFT", "P2_RIGHT", "P2_A", "P2_B", "P2_X", "P2_Y"]
for i, net in enumerate(BUTTONS):
    part(f"SW{i + 1}", S, "Switch:SW_Push", net.replace("_", " "), "battler:DSL_button_4ring",
         {"1": net, "2": "GND"}, bom=False)
SR = "Package_SO:TSSOP-16_4.4x5mm_P0.65mm"     # 1.2 mm: fits under the decks
for u, seat, ds, q7 in (("U8", "P1", "SR_CHAIN", "SR_DATA"), ("U9", "P2", "SR_CHAIN2", "SR_CHAIN")):
    pins = {"1": "SR_PL", "2": "SR_CLK", "15": "GND", "8": "GND", "16": "+3V3", "10": ds, "9": q7, "7": None}
    for k, d in enumerate(("11", "12", "13", "14", "3", "4", "5", "6")):   # D0..D7
        pins[d] = BUTTONS[(0 if seat == "P1" else 8) + k]
    part(u, S, "74xx:74HC165", "SN74HC165PWR", SR, pins, "C43420", "SN74HC165PWR", "Texas Instruments")
# U11 (2026-10-27, the ESP32-S3 board) is the third register, 8 more bits after seat 2's: both Starts and both Selects (their
# GPIOs went to the SD bus), the charger's PGOOD and CHG status; its two spare inputs are tied high. 24 clocks read the lot:
# bits 0 to 15 are as before (P1 Y .. UP, then P2), bits 16 to 23 are U11's D7 .. D0: POWER (PWR_BTN_N, Q1 from SW24), P2 START, P1 SELECT,
# P2 SELECT, CHG_STAT_N, CHG_PG_N, P1 START, 1. The Starts have 47k pull-ups (R88, R63) and the charger's lines theirs (R9, R10)
part("U11", S, "74xx:74HC165", "SN74HC165PWR", SR,
     {"1": "SR_PL", "2": "SR_CLK", "15": "GND", "8": "GND", "16": "+3V3", "10": "+3V3", "9": "SR_CHAIN2", "7": None,
      "11": "+3V3", "12": "P1_START", "13": "CHG_PG_N", "14": "CHG_STAT_N", "3": "P2_SELECT", "4": "P1_SELECT",
      "5": "P2_START", "6": "PWR_BTN_N"},
     "C43420", "SN74HC165PWR", "Texas Instruments")
C("C62", S, "100n", "+3V3")                    # U11 supply
R("R86", S, "47k", "+3V3", "P1_SELECT")        # the Selects had the MCU's own pull-ups
R("R87", S, "47k", "+3V3", "P2_SELECT")
R("R88", S, "47k", "+3V3", "P1_START")
R("R63", S, "47k", "+3V3", "P2_START")           # 47k like R86-R88: the pill's contact-resistance margin (V&V DEC-24)
# SR_CLK crosses the whole board to both registers: R65, at the RP2354B's end, slows its edges
# (source termination) so they ring less and couple less into SR_PL and SR_DATA, which run beside
# it up the left strip (a clearance rule for that left SR_CLK or SR_PL to U9 unrouted, 2026-10-22)
R("R65", S, "33", "SR_CLK_MCU", "SR_CLK")
C("C52", S, "100n", "+3V3")                    # U8 supply
C("C53", S, "100n", "+3V3")                    # U9 supply
RPACK = "Resistor_SMD:R_Array_Convex_4x0402"
for k, rn in enumerate(("RN1", "RN2", "RN3", "RN4")):
    nets_ = BUTTONS[4 * k:4 * k + 4]
    part(rn, S, "Device:R_Pack04", "47k", RPACK,
         {"1": nets_[0], "2": nets_[1], "3": nets_[2], "4": nets_[3],
          "5": "+3V3", "6": "+3V3", "7": "+3V3", "8": "+3V3"},
         "C2028", "4D02WGJ0473TCE", "UNI-ROYAL")   # 47k pull-ups: margin for the membrane pill's contact resistance (V&V DEC-24)
# Start and Select under each seat's DS Lite Start/Select membrane, between the clusters near the
# case's end: metal snap domes (2026-10-21, row 4: HYP 600-415S-260, 4.0 mm, hand-placed under
# Kapton dots), a 1.2 mm centre pad (1.6 before the 0.30 mm hole rule) and a 3.4-4.6 mm ring. On the power contacts the ring is VSYS,
# so the dome screens the signal pad
SMALL = "battler:Dome_4mm_Round"
part("SW20", S, "Switch:SW_Push", "P1 SELECT", SMALL, {"1": "P1_SELECT", "2": "GND"}, bom=False)
part("SW21", S, "Switch:SW_Push", "P1 START", SMALL, {"1": "P1_START", "2": "GND"}, bom=False)
part("SW22", S, "Switch:SW_Push", "P2 SELECT", SMALL, {"1": "P2_SELECT", "2": "GND"}, bom=False)
part("SW23", S, "Switch:SW_Push", "P2 START", SMALL, {"1": "P2_START", "2": "GND"}, bom=False)
# The power button (2026-10-28): a top-actuated ROCPU TPS850 J14K (C2759975, 250 gf, 1.4 mm) under seat 1's deck between U8 and the Start/Select
# frame, its cap through a recessed pocket in the deck. It joins VSYS to PWR_BTN_RAW (the latch on the Power sheet); the MCU reads it
# through Q1 as PWR_BTN_N (bit 16 of the chain). Pads 1 (two) and 2 (two) are the switch's contact pairs
part("SW24", S, "Switch:SW_Push", "POWER", "battler:Tact_ROCPU_TPS850", {"1": "PWR_BTN_RAW", "2": "VSYS"},
     "C2759975", "TPS850 J14K V SMTR LFS", "ROCPU")

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
