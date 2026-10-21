# Datasheet sources

The PDFs are not committed (see `../.gitignore`). Download them here to work on
the design; the file names below are what the notes and tools refer to.

| File | Part | Source |
|---|---|---|
| `er-tft024ips-3.pdf` | EastRising ER-TFT024IPS-3 2.4in IPS panel (fitted): drawing, FPC pinout, backlight, interface settings | https://www.buydisplay.com/download/manual/ER-TFT024IPS-3_Datasheet.pdf |
| `st7789v.pdf` | Sitronix ST7789V controller: commands, serial timing | https://www.newhavendisplay.com/appnotes/datasheets/LCDs/ST7789V.pdf |
| `er-tft035ips-6.pdf` | EastRising ER-TFT035IPS-6 3.5in IPS panel (the earlier panel, until 2026-10-21) | https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf |
| `rp2350.pdf` | RP2350 datasheet (the RP2354B is the RP2350B with 2 MB of flash inside) | https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf |
| (zip) | RP2350B minimal KiCad design, RP-010329-CA | https://pip.raspberrypi.com/documents/RP-010329-CA |
| `tp4056.pdf` | TP4056 Li-ion charger | https://www.lcsc.com/datasheet/lcsc_datasheet_2410121619_TOPPOWER-Nanjing-Extension-Microelectronics-TP4056-42-ESOP8_C16581.pdf |
| `me6211.pdf` | ME6211 LDO series | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_MICRONE-Nanjing-Micro-One-Elec-ME6211C33R5G_C235316.pdf |
| `fm8002a.pdf` | FM8002A mono audio amplifier | https://www.chipsourcetek.com/DataSheet/FM8002A.pdf |
| `mcp6002.pdf` | Microchip MCP6002 dual op-amp (U6, backlight sinks), DS20001733L | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_Microchip-Tech-MCP6002T-I-SN_C7377.pdf |
| `ao3400a.pdf` | Alpha & Omega AO3400A N-MOSFET (Q5, Q6, backlight sinks), Rev 3 Dec 2011 | https://www.lcsc.com/datasheet/lcsc_datasheet_1811081213_Alpha---Omega-Semicon-AO3400A_C20917.pdf |
| `srv05-4a.pdf` | Zhuhai Hongjiacheng SRV05-4A USB ESD array (U1), Rev 2.0 | https://datasheet.lcsc.com/datasheet/pdf/896c5101b23d5999ade9616dd078e12f.pdf?productCode=C20615829 |

The SPICE models for board/sim (Microchip's MCP6001/2/4 macromodel, Alpha & Omega's AO3400)
are not committed either: `board/sim/models.py` fetches and translates them.

`RPI-RP2350B-MINIMAL_LICENSE.txt` is committed: the RP2350B core (schematic
values, the QFN80, crystal and inductor footprints and their 3D models) comes
from Raspberry Pi's minimal design under that licence.
