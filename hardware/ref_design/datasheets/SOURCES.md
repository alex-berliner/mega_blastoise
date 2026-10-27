# Datasheet sources

The PDFs are committed. The `.txt` copies are not: make them with `pdftotext -layout x.pdf x.txt`.
The file names below are what the notes and tools refer to; the source column is where each PDF came from.

| File | Part | Source |
|---|---|---|
| `er-tft024ips-3.pdf` | EastRising ER-TFT024IPS-3 2.4in IPS panel (fitted 2026-10-21 to 2026-10-26): drawing, FPC pinout, backlight, interface settings | https://www.buydisplay.com/download/manual/ER-TFT024IPS-3_Datasheet.pdf |
| `st7789v.pdf` | Sitronix ST7789V controller: commands, serial timing | https://www.newhavendisplay.com/appnotes/datasheets/LCDs/ST7789V.pdf |
| `er-tft035ips-6.pdf` | EastRising ER-TFT035IPS-6 3.5in IPS panel (fitted again since 2026-10-26: its drawing was not re-read, the maker's site was behind Cloudflare and the Wayback copy was down; the old design's numbers stand) | https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf |
| `rp2350.pdf` | RP2350 datasheet (the RP2354B is the RP2350B with 2 MB of flash inside) | https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf |
| (zip) | RP2350B minimal KiCad design, RP-010329-CA | https://pip.raspberrypi.com/documents/RP-010329-CA |
| `tp4056.pdf` | TP4056 Li-ion charger | https://www.lcsc.com/datasheet/lcsc_datasheet_2410121619_TOPPOWER-Nanjing-Extension-Microelectronics-TP4056-42-ESOP8_C16581.pdf |
| `rt9013.pdf` | Richtek RT9013 500 mA LDO (U3, RT9013-33GB, C47773), DS9013-10 April 2011 | https://www.richtek.com/assets/product_file/RT9013/DS9013-10.pdf (JLC's part page for C47773 links the same revision) |
| `me6211.pdf` | ME6211 LDO series (U3 until 2026-10-22) | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_MICRONE-Nanjing-Micro-One-Elec-ME6211C33R5G_C235316.pdf |
| `fm8002a.pdf` | FM8002A mono audio amplifier | https://www.chipsourcetek.com/DataSheet/FM8002A.pdf |
| `mcp6002.pdf` | Microchip MCP6002 dual op-amp (U6, backlight sinks), DS20001733L | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_Microchip-Tech-MCP6002T-I-SN_C7377.pdf |
| `ao3400a.pdf` | Alpha & Omega AO3400A N-MOSFET (Q5, Q6, backlight sinks), Rev 3 Dec 2011 | https://www.lcsc.com/datasheet/lcsc_datasheet_1811081213_Alpha---Omega-Semicon-AO3400A_C20917.pdf |
| `srv05-4.pdf` | Semtech SRV05-4 RailClamp USB ESD array (U1, SRV05-4.TCT, C13612), Revision 08/11/04 | https://www.semtech.com/products/circuit-protection/low-capacitance/srv05-4 (JLC's part page for C13612 links the same revision) |
| `srv05-4a.pdf` | Zhuhai Hongjiacheng SRV05-4A USB ESD array (U1 2026-10-20 to 10-22), Rev 2.0 | https://datasheet.lcsc.com/datasheet/pdf/896c5101b23d5999ade9616dd078e12f.pdf?productCode=C20615829 |
| `esp32-s3.pdf` | Espressif ESP32-S3 series datasheet v2.2 (candidate MCU, see docs/next_revision_proposal.md) | https://www.espressif.com/sites/default/files/documentation/esp32-s3_datasheet_en.pdf |
| `esp32-s3-trm.pdf` | ESP32-S3 Technical Reference Manual (LCD_CAM, MMU, GPIO matrix) | https://www.espressif.com/sites/default/files/documentation/esp32-s3_technical_reference_manual_en.pdf |
| `esp32-s3-hw-design-guidelines.pdf` | ESP32-S3 hardware design guidelines (power, antenna keepout) | https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32s3/esp-hardware-design-guidelines-en-master-esp32s3.pdf |
| `esp32-s3-mini-1.pdf` | ESP32-S3-MINI-1 / MINI-1U module datasheet | https://www.espressif.com/sites/default/files/documentation/esp32-s3-mini-1_mini-1u_datasheet_en.pdf |
| `esp32-s3-wroom-1.pdf` | ESP32-S3-WROOM-1 / WROOM-1U module datasheet (N16R8: IO35-37 taken by the octal PSRAM) | https://www.espressif.com/sites/default/files/documentation/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf |
| `tlv757p.pdf` | TI TLV757P 1 A LDO (candidate 3V3 regulator for Wi-Fi peaks) | https://www.ti.com/lit/ds/symlink/tlv757p.pdf |
| `sn74hc165.pdf` | TI SN74HC165 8-bit parallel-in shift register (button chain) | https://www.ti.com/lit/ds/symlink/sn74hc165.pdf |
| `rt9013.pdf` | Richtek RT9013 500 mA LDO, DS9013-10 (Internet Archive copy: the Richtek URL now returns 404) | https://web.archive.org/web/2024/https://www.richtek.com/assets/product_file/RT9013/DS9013-10.pdf |

The SPICE models for board/sim (Microchip's MCP6001/2/4 macromodel, Alpha & Omega's AO3400)
are not committed either: `board/sim/models.py` fetches and translates them.

`RPI-RP2350B-MINIMAL_LICENSE.txt` is committed: the RP2350B core (schematic
values, the QFN80, crystal and inductor footprints and their 3D models) comes
from Raspberry Pi's minimal design under that licence.
