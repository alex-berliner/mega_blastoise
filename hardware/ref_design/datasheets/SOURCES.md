# Datasheet sources

The PDFs are committed. The `.txt` copies are not: make them with `pdftotext -layout x.pdf x.txt`.
The file names below are what the notes and tools refer to; the source column is where each PDF came from.

| File | Part | Source |
|---|---|---|
| `er-tft024ips-3.pdf` | EastRising ER-TFT024IPS-3 2.4in IPS panel (fitted 2026-10-21 to 2026-10-26): drawing, FPC pinout, backlight, interface settings | https://www.buydisplay.com/download/manual/ER-TFT024IPS-3_Datasheet.pdf |
| `st7789v.pdf` | Sitronix ST7789V controller: commands, serial timing | https://www.newhavendisplay.com/appnotes/datasheets/LCDs/ST7789V.pdf |
| `er-tft035ips-6.pdf` | EastRising ER-TFT035IPS-6 3.5in IPS panel (fitted again since 2026-10-26: its drawing was not re-read, the maker's site was behind Cloudflare and the Wayback copy was down; the old design's numbers stand) | https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf |
| `rp2350.pdf` | RP2350 datasheet (the RP2354B is the RP2350B with 2 MB of flash inside) | https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf |
| `rp2350b_minimal.zip` | RP2350B minimal KiCad design, RP-010329-CA (RPI-RP2350B-MINIMAL_R4-S1: schematic, PCB, project, PDF, BOM, licence) | https://pip.raspberrypi.com/documents/RP-010329-CA |
| `tp4056.pdf` | TP4056 Li-ion charger | https://www.lcsc.com/datasheet/lcsc_datasheet_2410121619_TOPPOWER-Nanjing-Extension-Microelectronics-TP4056-42-ESOP8_C16581.pdf |
| `rt9013.pdf` | Richtek RT9013 500 mA LDO (U3, RT9013-33GB, C47773), DS9013-10 April 2011 (Internet Archive copy: the Richtek URL now returns 404) | https://web.archive.org/web/2024/https://www.richtek.com/assets/product_file/RT9013/DS9013-10.pdf |
| `me6211.pdf` | ME6211 LDO series (U3 until 2026-10-22) | https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_MICRONE-Nanjing-Micro-One-Elec-ME6211C33R5G_C235316.pdf |
| `fm8002a.pdf` | Shenzhen Fuman FM8002A mono audio amplifier (C94625), the maker's own sheet S&CIC1448, in Chinese | https://datasheet.lcsc.com/datasheet/pdf/93ff929f6b4549cc91e72469b9693df5.pdf?productCode=C94625 |
| `mcp6001.pdf` | Microchip MCP6001/1R/1U/2/4 family, DS20001733L: covers MCP6001T-I/OT (C116490) and MCP6002T-I/SN (U6, backlight sinks, C7377). There is no separate `mcp6002.pdf`: LCSC serves this same file for both parts | https://ww1.microchip.com/downloads/aemDocuments/documents/MSLD/ProductDocuments/DataSheets/MCP6001-1R-1U-2-4-1-MHz-Low-Power-Op-Amp-DS20001733L.pdf |
| `ao3400a.pdf` | Alpha & Omega AO3400A N-MOSFET (Q5, Q6, backlight sinks, C20917), Rev 3.1 July 2023 | https://www.aosmd.com/res/datasheets/AO3400A.pdf |
| `srv05-4.pdf` | Semtech SRV05-4 RailClamp USB ESD array (U1, SRV05-4.TCT, C13612), Revision 08/11/04. Semtech's product page lists a 2018-12-15 file behind a Salesforce link that would not download | https://datasheet.lcsc.com/datasheet/pdf/4abe760a30bb447e9c1dddced46e8df7.pdf?productCode=C13612 |
| `2n7002_cj.pdf` | Jiangsu Changjiang (CJ) 2N7002 N-MOSFET, SOT-23 (C8545), rev J Sep 2016 | https://datasheet.lcsc.com/datasheet/pdf/a141b8bd86b14475955ac8c4d3eea0a8.pdf?productCode=C8545 |
| `bat54c.pdf` | Nexperia BAT54C Schottky pair, common cathode (C37704), BAT54C v.6 1 July 2022 | https://datasheet.lcsc.com/datasheet/pdf/753e903e66757b8dda706efd5e61ce1e.pdf?productCode=C37704 |
| `bat54ws.pdf` | BAT54WS Schottky, SOD-323 (C7502694), Rev 2.1. The sheet is Zhuhai Hongjiacheng's; LCSC lists the brand as R+O | https://datasheet.lcsc.com/datasheet/pdf/7faa091588ad02f4716f52e384e3baab.pdf?productCode=C7502694 |
| `tctr0805.pdf` | Resistor.Today (Kaibu) TCTR series NTC thermistors, 0402 to 0805 (TCTR0805F10K0F4460T, C525375), in Chinese; series sheet, the 4460 K B value is not tabulated by name | https://datasheet.lcsc.com/datasheet/pdf/c5db8850fccde8f443455e61067f2d5e.pdf?productCode=C525375 |
| `jst_sh.pdf` | JST SH series catalogue: SM02B/SM03B-SRSS-TB side-entry header drawing and PC board layout (C160402, C160403) | https://www.jst-mfg.com/product/pdf/eng/eSH.pdf |
| `molex_53261.pdf` | Molex 53261 PicoBlade SMT right-angle header (53261-0271, C177225): sales drawing SD-53261-024 rev J with the recommended PC board pattern, plus tape (SD-53261-023) and mating section. Scanned, no text. LCSC files it under C189700, the same MPN | https://datasheet.lcsc.com/datasheet/pdf/8f584e54f84ed95b57c7e5b04274731e.pdf?productCode=C189700 |
| `hro_type-c-31-m-12.pdf` | HRO TYPE-C-31-M-12 USB-C receptacle (C165948): drawing, pinout, recommended PCB layout, 2020-12-08 rev A. Drawing only, almost no text | https://datasheet.lcsc.com/datasheet/pdf/9e56b777c022540fcce7c7f67825f55e.pdf?productCode=C165948 |
| `hirose_fh12.pdf` | Hirose FH12 series catalogue, Nov 2016 (FH12-50S-0.5SH(55), C202116): land and metal mask dimensions | https://datasheet.lcsc.com/datasheet/pdf/51b5cd75b0ad9d5ac5de1a2978a597c8.pdf?productCode=C202116 |
| `abm8-272.pdf` | Abracon ABM8-272-T3 crystal (C20625731), drawing 456603 revision B | https://abracon.com/datasheets/ABM8-272-T3.pdf |
| `aota-b201610.pdf` | Abracon AOTA-B201610S series molded power inductor (AOTA-B201610S3R3-101-T, C42411119), revision A 9/13/2024 | https://datasheet.lcsc.com/datasheet/pdf/0ca6a1bbc70fd13a89f7695ff7dd8558.pdf?productCode=C42411119 |
| `xkb_ts-1088.pdf` | TS-1088 tact switch family drawing with the PCB pad layout (TS-1088-AR02016, C720477), rev A. The drawing is DongGuan XunPu's (LCSC lists XUNPU); no text | https://datasheet.lcsc.com/datasheet/pdf/0475ac02febf455ca9ddcfb380b0df0d.pdf?productCode=C720477 |
| `uniroyal_4d02.pdf` | UNI-ROYAL chip resistor arrays, 4D02 among them (4D02WGJ0473TCE, C2028), 2017/06/12 | https://datasheet.lcsc.com/datasheet/pdf/27ea7f29d169b3fd0e42802e546ced43.pdf?productCode=C2028 |
| `dome_600-415s-260.pdf` | Hon Yuan (HYP) 600 series metal dome customer drawing, 600-****-*** part-number key (600-415S-260, C256258). Generic family drawing; no text | https://datasheet.lcsc.com/datasheet/pdf/5c1bc660c97aa5f5ab86d1c45a623c9a.pdf?productCode=C256258 |
| `samsung_cl05a475mp5nrnc.pdf` | Samsung CL05A475MP5NRNC 4.7 uF 10 V X5R 0402: characteristics sheet with the DC bias curve (Samsung document hosted by DigiKey) | https://media.digikey.com/pdf/Data%20Sheets/Samsung%20PDFs/CL05A475MP5NRNC_Char.pdf |
| `samsung_cl10a475ko8nnnc_dcbias.csv` | Samsung CL10A475KO8NNNC 4.7 uF 16 V X5R 0603: DC bias curve, 25 degC, 1 kHz, taken from the JSON in Samsung's component library page | https://weblib.samsungsem.com/mlcc/mlcc-ec-data-sheet.do?partNumber=CL10A475KO8NNN |
| `samsung_cl10a106kp8nnnc_dcbias.csv` | Samsung CL10A106KP8NNNC 10 uF 10 V X5R 0603: DC bias curve, as above | https://weblib.samsungsem.com/mlcc/mlcc-ec-data-sheet.do?partNumber=CL10A106KP8NNN |
| `samsung_cl10a106ma8nrnc_dcbias.csv` | Samsung CL10A106MA8NRNC 10 uF 25 V X5R 0603: DC bias curve, as above | https://weblib.samsungsem.com/mlcc/mlcc-ec-data-sheet.do?partNumber=CL10A106MA8NRN |
| `srv05-4a.pdf` | Zhuhai Hongjiacheng SRV05-4A USB ESD array (U1 2026-10-20 to 10-22), Rev 2.0 | https://datasheet.lcsc.com/datasheet/pdf/896c5101b23d5999ade9616dd078e12f.pdf?productCode=C20615829 |
| `esp32-s3.pdf` | Espressif ESP32-S3 series datasheet v2.2 (candidate MCU, see docs/living/next_revision_proposal.md) | https://www.espressif.com/sites/default/files/documentation/esp32-s3_datasheet_en.pdf |
| `esp32-s3-trm.pdf` | ESP32-S3 Technical Reference Manual (LCD_CAM, MMU, GPIO matrix) | https://www.espressif.com/sites/default/files/documentation/esp32-s3_technical_reference_manual_en.pdf |
| `esp32-s3-hw-design-guidelines.pdf` | ESP32-S3 hardware design guidelines (power, antenna keepout) | https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32s3/esp-hardware-design-guidelines-en-master-esp32s3.pdf |
| `esp32-s3-mini-1.pdf` | ESP32-S3-MINI-1 / MINI-1U module datasheet | https://www.espressif.com/sites/default/files/documentation/esp32-s3-mini-1_mini-1u_datasheet_en.pdf |
| `esp32-s3-wroom-1.pdf` | ESP32-S3-WROOM-1 / WROOM-1U module datasheet (N16R8: IO35-37 taken by the octal PSRAM) | https://www.espressif.com/sites/default/files/documentation/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf |
| `tlv757p.pdf` | TI TLV757P 1 A LDO (candidate 3V3 regulator for Wi-Fi peaks) | https://www.ti.com/lit/ds/symlink/tlv757p.pdf |
| `sn74hc165.pdf` | TI SN74HC165 8-bit parallel-in shift register (button chain) | https://www.ti.com/lit/ds/symlink/sn74hc165.pdf |

The SPICE models for board/sim (Microchip's MCP6001/2/4 macromodel, Alpha & Omega's AO3400)
are not committed either: `board/sim/models.py` fetches and translates them.

`RPI-RP2350B-MINIMAL_LICENSE.txt` is committed: the RP2350B core (schematic
values, the QFN80, crystal and inductor footprints and their 3D models) comes
from Raspberry Pi's minimal design under that licence.

microSD slots (2026-10-27, SD slot sizing for the ESP32-S3 board): `tf-01a_C91145.pdf` is the Korean Hroparts TF-01A drawing
(1 page, from LCSC C91145, https://datasheet.lcsc.com/datasheet/pdf/7650eac21a54674b08ad4ba21e17fceb.pdf?productCode=C91145);
`hirose_dm3_catalog_C114218.pdf` is Hirose's DM3 catalog (May 2017) as LCSC C114218 carries it, DM3AT-SF-PEJM5 on page 3
(https://datasheet.lcsc.com/datasheet/pdf/ed7c38214cbd45158a701879217600e9.pdf?productCode=C114218; hirose.com refused the
download); `jmeconn_5001B-TF128-054_C53278151.pdf` is the JMEconn 1.28 mm push-push drawing (LCSC C53278151,
https://datasheet.lcsc.com/datasheet/pdf/e7a21fcb804bb8d9a0644d5e97087279.pdf?productCode=C53278151). The TF-01A and JMEconn
drawings are images (no text layer): read by eye.

`rocpu_tps850.pdf`: ROCPU TPS 850 series, top actuated (2 pages; LCSC C2759975, the J14K 250 gf 1.4 mm), copied from the power-button analysis's download
(scratchpad esp32_pwrbtn/ds/C2759975): land drawing on page 2, which battler:Tact_ROCPU_TPS850 (tools/fp_tact.py) follows.
