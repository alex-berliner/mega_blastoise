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
reads it with no pins. So USBLC6-2SC6, TPS63001, 2N7002 and W25Q128JVP are
drawn with the symbols they derive from (same pins); the value is the real part.
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
C0402_RPI = "battler:C_0402_1005Metric_RPi"          # the copied core's passives
C0402_RPI_WIDE = "battler:C_0402_1005Metric_RPi_Wide"  # VREG_LX copper runs between these pads
R0402_RPI = "battler:R_0402_1005Metric_RPi"

RES = {  # value: (lcsc, mpn)  all UNI-ROYAL 0402 1% unless noted
    "0": ("C17168", "0402WGF0000TCE"),
    "27": ("C25100", "0402WGF270JTCE"),
    "33": ("C25105", "0402WGF330JTCE"),
    "100": ("C25076", "0402WGF1000TCE"),
    "1k": ("C11702", "0402WGF1001TCE"),
    "1.2k": ("C25862", "0402WGF1201TCE"),
    "5.1k": ("C25905", "0402WGF5101TCE"),
    "10k": ("C25744", "0402WGF1002TCE"),
    "47k": ("C25792", "0402WGF4702TCE"),
    "100k": ("C25741", "0402WGF1003TCE"),
    "470k": ("C25790", "0402WGF4703TCE"),
}
CAP = {  # value: (lcsc, mpn, footprint)
    "15p": ("C1548", "0402CG150J500NT", C0402),
    "100n": ("C1525", "CL05B104KO5NNNC", C0402),
    "220n": ("C16772", "CL05B224KO5NNNC", C0402),
    "1u": ("C52923", "CL05A105KA5NQNC", C0402),
    "4.7u 0402": ("C23733", "CL05A475MP5NRNC", C0402),
    "4.7u": ("C19666", "CL10A475KO8NNNC", C0603),
    "10u": ("C19702", "CL10A106KP8NNNC", C0603),
    "10u 25V": ("C96446", "CL10A106MA8NRNC", C0603),
    "1u 50V": ("C15849", "CL10A105KB8NNNC", C0603),
}


def R(ref, sheet, value, a, b, rot=0, fp=R0402):
    lcsc, mpn = RES[value]
    part(ref, sheet, "Device:R", value, fp, {"1": a, "2": b}, lcsc, mpn, "UNI-ROYAL", rot)


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
part("U1", S, "Power_Protection:USBLC6-2P6", "USBLC6-2SC6", "Package_TO_SOT_SMD:SOT-23-6",
     {"1": "USB_DP", "6": "USB_DP", "3": "USB_DM", "4": "USB_DM", "5": "VBUS", "2": "GND"},
     "C2687116", "USBLC6-2SC6", "STMicroelectronics")
C("C1", S, "10u 25V", "VBUS")
part("U2", S, "Battery_Management:BQ24074RGT", "BQ24074RGTR",
     "Package_DFN_QFN:VQFN-16-1EP_3x3mm_P0.5mm_EP1.6x1.6mm_ThermalVias",
     {"13": "VBUS", "10": "VSYS", "11": "VSYS", "2": "VBAT", "3": "VBAT", "1": "TS",
      "4": "GND", "5": "CHG_EN2", "6": "CHG_EN1", "7": "VIN_GOOD_N", "8": "GND", "17": "GND",
      "9": "CHG_STAT_N", "12": "ILIM", "14": None, "15": None, "16": "ISET"},
     "C54313", "BQ24074RGTR", "Texas Instruments")
R("R5", S, "1.2k", "ILIM", "GND")              # input limit 1610/1.2k = 1.34 A when EN2=1, EN1=0
R("R6", S, "1.2k", "ISET", "GND")              # fast charge 890/1.2k = 740 mA
part("RT1", S, "Device:Thermistor_NTC", "10k B3380", "Resistor_SMD:R_0603_1608Metric",
     {"1": "TS", "2": "GND"}, "C13564", "NCP18XH103F03RB", "Murata")  # on the board over the cell
R("R7", S, "100k", "CHG_EN2", "GND")           # boots in USB500 (EN2=0, EN1=1) until firmware
R("R8", S, "100k", "CHG_EN1", "+3V3")          # reads CC and picks a higher limit
R("R9", S, "100k", "VIN_GOOD_N", "+3V3")
R("R10", S, "100k", "CHG_STAT_N", "+3V3")
part("D1", S, "Device:LED", "amber", "LED_SMD:LED_0603_1608Metric",
     {"1": "CHG_STAT_N", "2": "CHG_LED_A"}, "C89811", "NCD0603Y2", "Foshan NationStar")
R("R11", S, "1k", "+3V3", "CHG_LED_A")
C("C2", S, "4.7u", "VSYS")
C("C3", S, "4.7u", "VBAT")
part("J2", S, "Connector_Generic_MountingPin:Conn_01x02_MountingPin", "S2B-PH-SM4-TB",
     "battler:JST_PH_S2B-PH-SM4-TB_1x02-1MP_P2.00mm_Horizontal",
     {"1": "VBAT", "2": "GND", "MP": None}, "C295747", "S2B-PH-SM4-TB(LF)(SN)", "JST")
part("#FLG01", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VBUS"}, bom=False)
part("#FLG02", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "GND"}, bom=False)

# =================================================================================
# 3V3 buck-boost, power switch latch, sensing
# =================================================================================
S = "power.kicad_sch"
part("U3", S, "Regulator_Switching:TPS63000", "TPS63001DRCR", "Package_SON:Texas_DRC0010J_ThermalVias",
     {"5": "VSYS", "8": "VINA", "6": "PWR_EN", "7": "GND", "4": "BB_L1", "2": "BB_L2",
      "1": "+3V3", "10": "+3V3", "9": "GND", "3": "GND", "11": "GND"},
     "C28060", "TPS63001DRCR", "Texas Instruments")
part("L1", S, "Device:L", "2.2u", "Inductor_SMD:L_1008_2520Metric", {"1": "BB_L1", "2": "BB_L2"},
     "C5832372", "FTC252012S2R2MBCA", "FH")
R("R12", S, "100", "VSYS", "VINA")
C("C4", S, "100n", "VINA")
part("#FLG03", S, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VINA"}, bom=False)   # VINA fed through R12
C("C5", S, "10u", "VSYS")
C("C6", S, "10u", "+3V3")
C("C7", S, "10u", "+3V3")
part("SW17", S, "Switch:SW_Push", "PWR", "battler:SW_Alps_SKRTLAE010_Pullback",
     {"1": "VSYS", "2": "PWR_BTN_RAW"}, "C110293", "SKRTLAE010", "ALPS")
R("R13", S, "100k", "PWR_BTN_RAW", "GND")
part("D2", S, "Diode:BAT54C", "BAT54C", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "PWR_HOLD", "3": "PWR_EN"}, "C37704", "BAT54C,215", "Nexperia")
R("R14", S, "10k", "VBUS", "VBUS_EN")
part("D3", S, "Device:D_Schottky", "BAT54WS", "Diode_SMD:D_SOD-323",
     {"1": "PWR_EN", "2": "VBUS_EN"}, "C7502694", "BAT54WS", "")
R("R15", S, "100k", "PWR_EN", "GND")
part("Q1", S, "Transistor_FET:Q_NMOS_GSD", "2N7002", "Package_TO_SOT_SMD:SOT-23",
     {"1": "PWR_BTN_RAW", "2": "GND", "3": "PWR_BTN_N"}, "C8545", "2N7002", "Changjiang")
R("R16", S, "100k", "PWR_BTN_N", "+3V3")
R("R17", S, "470k", "VBAT", "VBAT_SENSE")
R("R18", S, "470k", "VBAT_SENSE", "GND")
C("C8", S, "100n", "VBAT_SENSE")
for i, net in enumerate(["VBUS", "VSYS", "VBAT", "+3V3", "GND"]):
    part(f"TP{i + 1}", S, "Connector:TestPoint", net, "TestPoint:TestPoint_Pad_D1.0mm", {"1": net}, bom=False)

# =================================================================================
# RP2350B core (Raspberry Pi RP2350B minimal design, MIT) + flash, boot, debug
# =================================================================================
S = "mcu.kicad_sch"
GPIO = {  # GPIO number: net
    0: "UART_TX", 1: "UART_RX", 2: "PWR_HOLD", 3: "PWR_BTN_N",
    4: "P2_UP", 5: "P2_DOWN", 6: "P2_LEFT", 7: "P2_RIGHT",
    8: "I2S_DIN", 9: "I2S_BCLK", 10: "I2S_LRCLK", 11: "AMP_SD",
    12: "P2_A", 13: "P2_B", 14: "P2_X", 15: "P2_Y",
    16: "STATUS_LED", 17: "CHG_STAT_N", 18: "VIN_GOOD_N", 19: "CHG_EN1", 20: "CHG_EN2",
    24: "P1_A", 25: "P1_B", 26: "P1_X", 27: "P1_Y",
    28: "P1_UP", 29: "P1_DOWN", 30: "P1_LEFT", 31: "P1_RIGHT",
    32: "LCD_TE", 33: "LCD_DC", 34: "LCD_RST", 35: "BL_SDI",
    36: "LCD_MISO", 37: "LCD_CS", 38: "LCD_SCK_MCU", 39: "LCD_MOSI_MCU",
    40: "VBAT_SENSE", 41: "CC1_SENSE", 42: "CC2_SENSE",
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
           "75": "QSPI_SS", "71": "QSPI_SCLK", "72": "QSPI_SD0", "74": "QSPI_SD1",
           "73": "QSPI_SD2", "70": "QSPI_SD3"})
part("U4", S, "MCU_RaspberryPi:RP2350B", "RP2350B", "battler:RP2350B_QFN80_RPi", u4,
     "C42415655", "RP2350B", "Raspberry Pi")
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
R("R21", S, "27", "USB_DP", "USB_DP_MCU", fp=R0402_RPI)
R("R22", S, "27", "USB_DM", "USB_DM_MCU", fp=R0402_RPI)
part("U5", S, "Memory_Flash:W25Q32JVZP", "W25Q128JVPIQ", "Package_SON:WSON-8-1EP_6x5mm_P1.27mm_EP3.4x4.3mm",
     {"1": "FLASH_SS", "2": "QSPI_SD1", "3": "QSPI_SD2", "4": "GND", "5": "QSPI_SD0", "6": "QSPI_SCLK",
      "7": "QSPI_SD3", "8": "+3V3", "9": "GND"}, "C190862", "W25Q128JVPIQ", "Winbond")
R("R23", S, "10k", "+3V3", "FLASH_SS", fp=R0402_RPI)
R("R24", S, "0", "QSPI_SS", "FLASH_SS", fp=R0402_RPI)
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
     {"1": "UART_TX", "2": "GND", "3": "UART_RX", "MP": None}, "C160403", "SM03B-SRSS-TB(LF)(SN)", "JST")
R("R27", "debug.kicad_sch", "1k", "STATUS_LED", "STATUS_LED_A")
part("D4", "debug.kicad_sch", "Device:LED", "red", "LED_SMD:LED_0603_1608Metric",
     {"1": "GND", "2": "STATUS_LED_A"}, "C2286", "KT-0603R", "Hubei KENTO")
part("#FLG05", P, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "+1V1"}, bom=False)
part("#FLG06", P, "power:PWR_FLAG", "PWR_FLAG", "", {"1": "VREG_AVDD"}, bom=False)

# =================================================================================
# Display: QD3525 (3.5in IPS 320x480, ST7796U) on its 48-pin solder FPC; an AP3156
# charge pump drives the panel's six backlight LEDs (common anode, a cathode each)
# =================================================================================
S = "display.kicad_sch"
fpc = {"5": "+3V3", "6": "+3V3",                          # VDD, IOVCC
       "7": "LCD_TE", "8": "LCD_CS", "9": "LCD_DC", "10": "LCD_SCK",
       "11": "+3V3",                                      # RD: parallel read strobe, unused: high
       "12": "LCD_MOSI", "13": "LCD_MISO", "38": "LCD_RST",
       "39": "+3V3", "40": "+3V3", "41": "+3V3",          # IM2..IM0 = 111: 4-line 8-bit SPI
       "42": "LED_A"}
fpc.update({str(p): "GND" for p in range(14, 38)})        # GND, DB0-17, DEN, PCLK, HSYNC, VSYNC: tied low
fpc.update({str(43 + i): f"LED_K{i + 1}" for i in range(6)})
fpc.update({str(p): None for p in (1, 2, 3, 4)})          # resistive touch: not on this panel
part("J3", S, "Connector_Generic:Conn_01x48", "QD3525", "battler:QD3525_FPC_1x48_P0.80mm_Solder", fpc)
R("R28", S, "33", "LCD_SCK_MCU", "LCD_SCK")       # source termination, fitted next to the MCU
R("R29", S, "33", "LCD_MOSI_MCU", "LCD_MOSI")
C("C29", S, "1u", "+3V3")
C("C30", S, "100n", "+3V3")
part("U6", S, "battler:AP3156", "AP3156FVG-7", "Package_DFN_QFN:QFN-16-1EP_4x4mm_P0.65mm_EP2.5x2.5mm_ThermalVias",
     {"10": "VSYS", "2": "BL_SDI", "3": None, "6": "BL_C1P", "7": "BL_C1N", "5": "BL_C2P", "8": "BL_C2N",
      "4": "LED_A", "12": "LED_K1", "13": "LED_K2", "14": "LED_K3", "15": "LED_K4", "16": "LED_K5", "1": "LED_K6",
      "9": "GND", "11": "GND", "17": "GND"},
     "C443652", "AP3156FVG-7", "Diodes Incorporated")
C("C31", S, "1u", "VSYS")                          # charge pump input
C("C32", S, "1u", "LED_A")                         # charge pump output
C("C33", S, "1u", "BL_C1P", "BL_C1N")              # flying capacitors
C("C36", S, "1u", "BL_C2P", "BL_C2N")
R("R31", S, "100k", "BL_SDI", "GND")               # holds the driver off until firmware drives SDI

# =================================================================================
# Audio: MAX98357A class D into an 8 ohm speaker
# =================================================================================
S = "audio.kicad_sch"
part("U7", S, "Audio:MAX98357A", "MAX98357AETE+T", "Package_DFN_QFN:TQFN-16-1EP_3x3mm_P0.5mm_EP1.23x1.23mm_ThermalVias",
     {"1": "I2S_DIN", "16": "I2S_BCLK", "14": "I2S_LRCLK", "4": "AMP_SD_MODE", "2": None,
      "7": "VSYS", "8": "VSYS", "3": "GND", "11": "GND", "15": "GND", "17": "GND",
      "5": None, "6": None, "12": None, "13": None, "9": "SPK_P_AMP", "10": "SPK_N_AMP"},
     "C910544", "MAX98357AETE+T", "Analog Devices")
R("R32", S, "47k", "AMP_SD", "AMP_SD_MODE")      # 2.2 V on SD_MODE: left channel; 0 V: shutdown
C("C34", S, "10u", "VSYS")
C("C35", S, "100n", "VSYS")
for ref, a, b in (("FB1", "SPK_P_AMP", "SPK_P"), ("FB2", "SPK_N_AMP", "SPK_N")):
    part(ref, S, "Device:FerriteBead_Small", "120R@100MHz", "Inductor_SMD:L_0603_1608Metric",
         {"1": a, "2": b}, "C14709", "BLM18PG121SN1D", "Murata")
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
