# ESP32-S3 build: differences from the proposal's pin map

The proposal is `docs/esp32s3_board_proposal.md` (Pin map table). The generated map is `docs/pinmap.md`
(`board/tools/pinmap.py`). Compared row by row on 2026-10-27:

**GPIO assignments: no differences.** Every GPIO in the proposal's table carries the same net in `board/design.py`
(GPIO 0 BOOT, 1 VBAT_SENSE, 2 CC1_SENSE, 3 SR_DATA, 4 CC2_SENSE, 5 SR_PL, 6 LCD_DC, 7 LCD_RST, 8 AUDIO_PWM, 9 BL_PWM,
10 LCD_TE, 11-12 SD_CMD/CLK, 13-16 SD_D0-D3, 17 SD_CD, 18 SR_CLK, 19/20 USB, 21 PWR_HOLD, 38-44 and 47 LCD_D0-D7,
45 AMP_EN, 46 CHG_OFF, 48 LCD_WR). The data bit order is D0 = IO38 ... D4 = IO42, D5 = IO43, D6 = IO44, D7 = IO47.

Differences that are not GPIOs, for the firmware session:

1. **Buttons: 24-bit chain, not 16.** The proposal left the Starts and Selects on GPIOs; the build puts them, with the
   charger status lines, on a third 74HC165 (U11, chain U11 -> U9 -> U8). Bits 0-15 are unchanged; bit 16 PWR_BTN_N
   (the POWER button SW24, through Q1; not a Start), 17 P2_START, 18 P1_SELECT, 19 P2_SELECT, 20 CHG_STAT_N, 21 CHG_PG_N, 22 P1_START, 23 always 1. No GPIO is used
   by these (the proposal's table has no such rows either: the six GPIOs it needed are the ones counted spare).
2. **Series resistors.** 33 ohm R70-R77 on D0-D7 and R78 on WR (by the module), none on DC/RST/TE. SR_CLK has R65 (33 ohm).
3. **USB.** 22 ohm in series (R21/R22) next to the module; the capacitor pads C13/C14 are unfitted (guidelines).
4. **Audio filter.** 1.5k / 4.7n twice (R32/R33 1.5k), not 1k: -46.0 dB at 312.5 kHz, -3 dB at 6.7 kHz
   (`board/sim/audio_filter.py`; 1k would give -39.3 dB).
5. **Panel strap.** IM0 and IM1 high, IM2 low (011), CSX tied to ground, RDX high: no GPIO, but LCD_CS and LCD_RD are
   absent from the pin map for that reason.
6. **EN and BOOT** are nets with parts: EN 10k to 3V3 and 1 uF to ground, RESET (SW19) to ground; BOOT (SW18) to ground;
   J7 pads 3V3, EN, BOOT, GND.
7. **GPIO 35-37** are not connected (octal PSRAM).
8. **Part numbers.** Module LCSC C3013946 (the proposal said C2913202, the PCB-antenna module); TLV75733P C485517;
   microSD slot Hirose DM3AT-SF-PEJM5 C114218.

Not verified in this session: JLC Basic/Extended and stock of any of the new parts (the parts database was not
reachable); the DM3AT card-detect pin order against Hirose's drawing.

## Firmware map delta from the dedicated power button (2026-10-28, Alex)

SW24 (ROCPU TPS850 J14K) in seat 1's deck is the only power contact. Both Starts are plain buttons to GND with 47k pull-ups
(R88, R63); Q7, D8 and R62 are gone. Chain order U11 -> U9 -> U8, 24 bits, 0 = pressed:

| Bit | Before | Now |
|---|---|---|
| 16 | P1 Start and power (PWR_BTN_N) | **power button only** (PWR_BTN_N, through Q1); not a Start |
| 17 | P2 Start and power (P2_START) | P2 Start only; it no longer wakes the device |
| 22 | tied high | **P1 Start** (new) |
| 23 | tied high | tied high |
| 0-15, 18-21 | | unchanged |

Firmware: P1 Start moves from bit 16 to bit 22; bit 16 is Power (hold to turn off, and the boot-cause read); bit 17 is a plain Start.
