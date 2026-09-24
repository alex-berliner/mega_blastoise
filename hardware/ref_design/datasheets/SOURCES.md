# Datasheet sources

The PDFs are not committed (see `../.gitignore`). Download them here to work on
the design; the file names below are what the notes and tools refer to.

| File | Part | Source |
|---|---|---|
| `qd3525.pdf` | QD3525 3.5in IPS panel: drawing, pinout, backlight (Shenzhen Quandong Electronic, rev A) | https://www.lcdwiki.com/res/MSP3525_MSP3526/QD3525-Specifications-A.pdf |
| `msp3525_schematic.pdf` | LCDwiki MSP3525 module: the panel maker's own hookup | https://www.lcdwiki.com/res/MSP3525_MSP3526/3.5inch_SPI_Module_MSP3525_MSP3526_Schematic.pdf |
| `st7796s.pdf` | Sitronix ST7796S controller | https://www.lcdwiki.com/res/MSP3525_MSP3526/ST7796S-Sitronix.pdf |
| `rp2350.pdf` | RP2350 datasheet | https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf |
| (zip) | RP2350B minimal KiCad design, RP-010329-CA | https://pip.raspberrypi.com/documents/RP-010329-CA |
| `bq24074.pdf` | TI BQ24074 charger | https://www.ti.com/lit/ds/symlink/bq24074.pdf |
| `tps63001.pdf` | TI TPS63001 buck-boost | https://www.ti.com/lit/ds/symlink/tps63001.pdf |
| `ap3156.pdf` | Diodes AP3156 6-channel charge pump LED driver | https://www.diodes.com/assets/Datasheets/AP3156.pdf |
| `max98357a.pdf` | Analog Devices MAX98357A amplifier | https://www.analog.com/media/en/technical-documentation/data-sheets/MAX98357A-MAX98357B.pdf |

`RPI-RP2350B-MINIMAL_LICENSE.txt` is committed: the RP2350B core (schematic
values, the QFN80, crystal and inductor footprints and their 3D models) comes
from Raspberry Pi's minimal design under that licence.
