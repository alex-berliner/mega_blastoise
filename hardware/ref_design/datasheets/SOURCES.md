# Datasheet sources

The PDFs are not committed (see `../.gitignore`). Download them here to work on
the design; the file names below are what the notes and tools refer to.

| File | Part | Source |
|---|---|---|
| `er-tft035ips-6.pdf` | EastRising ER-TFT035IPS-6 3.5in IPS panel: drawing, FPC pinout, backlight, interface settings | https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf |
| `ili9488.pdf` | Ilitek ILI9488 controller: commands, SPI timing | https://www.elecrow.com/download/ILI9488%20Data%20Sheet_100.pdf |
| `rp2350.pdf` | RP2350 datasheet (the RP2354B is the RP2350B with 2 MB of flash inside) | https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf |
| (zip) | RP2350B minimal KiCad design, RP-010329-CA | https://pip.raspberrypi.com/documents/RP-010329-CA |
| `tp4056.pdf` | TP4056 Li-ion charger | https://www.lcsc.com/datasheet/lcsc_datasheet_2410121619_TOPPOWER-Nanjing-Extension-Microelectronics-TP4056-42-ESOP8_C16581.pdf |
| `me6211.pdf` | ME6211 LDO series | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_MICRONE-Nanjing-Micro-One-Elec-ME6211C33R5G_C235316.pdf |
| `fm8002a.pdf` | FM8002A mono audio amplifier | https://www.chipsourcetek.com/DataSheet/FM8002A.pdf |
| `ap3156.pdf` | Diodes AP3156 6-channel charge pump LED driver | https://www.diodes.com/assets/Datasheets/AP3156.pdf |
| `srv05-4.pdf` | Semtech SRV05-4.TCT USB ESD array (U1), Revision 08/11/04 | https://www.lcsc.com/datasheet/lcsc_datasheet_1809212114_SEMTECH-SRV05-4-TCT_C13612.pdf |

`RPI-RP2350B-MINIMAL_LICENSE.txt` is committed: the RP2350B core (schematic
values, the QFN80, crystal and inductor footprints and their 3D models) comes
from Raspberry Pi's minimal design under that licence.
