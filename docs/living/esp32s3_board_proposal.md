# Proposal: the ESP32-S3 board (next revision)

Status: chosen by Alex on 2026-10-27 as the next board. Nothing in `board/design.py`, the
KiCad files, the case or the firmware has changed yet. Where this conflicts with
`docs/living/next_revision_proposal.md` (written for the RP2354B), this file wins. The 60 fps panel
link is `docs/living/display_parallel_proposal.md`, with the pins below instead of its RP2350 ones.

## Why the MCU changes

Games become ordinary binaries that other people write, picked by a launcher (see
`next_revision_proposal.md` section 1). Pokemon alone is 1,367,472 B of flash today, so the
RP2354B's 2 MB leaves too little for anyone else's game, and Alex rejected splitting binaries.
The ESP32-S3-WROOM-1-N16R8 has 16 MB of flash and 8 MB of octal PSRAM. The PSRAM holds the
full 307,200 B frame, so the shared core needs no band rendering.

| | |
|---|---|
| Part | ESP32-S3-WROOM-1U-N16R8 (U.FL connector, external flex antenna), LCSC C3013946 (the PCB-antenna WROOM-1-N16R8 was C2913202) |
| Price | $5.35 at 1, $3.59 at 650+ (JLC search page, 2026-10-27) |
| Size | 18 x 19.2 x 3.2 mm (WROOM-1U), no antenna on the board; the antenna sits on the case's left wall on a 40 mm U.FL cable |
| Ambient limit | 65 C (the R8 PSRAM variant; `datasheets/esp32-s3-wroom-1.pdf`) |
| JLC assembly | Standard only, X-ray inspection |
| Radio | Wi-Fi is used from this board on (Alex, 2026-10-27, replacing the earlier no-radio decision). BLE stays off. The module may become the WROOM-1U-N16R8 with an external antenna, pending Blastoise Schematic's antenna analysis; that changes the part number. |

## Board changes

1. **MCU.** Remove the RP2354B, its crystal and the 1.1 V core parts. The module carries
   flash, PSRAM, crystal and antenna.
2. **Antenna.** The module's antenna end goes at a board edge with the keepout from
   `datasheets/esp32-s3-hw-design-guidelines.pdf`: no copper, cell or metal under or beside
   it, and plastic case wall over it. Check it fits the DS Lite outline.
3. **3V3 regulator.** U3 RT9013-33GB (500 mA) becomes the TLV75733P (1 A). Same SOT-23-5
   pinout: IN 1, GND 2, EN 3, NC 4, OUT 5 (`datasheets/tlv757p.txt` line 114,
   `datasheets/rt9013.txt` lines 91-99). Dropout 425 mV max at 1 A for 3.3 V out (line 298).
   Reason: Wi-Fi TX peaks at 355 mA and Espressif wants a 0.5 A supply. Add 22 uF plus
   0.1 uF at the module's 3V3 pin, and the 10k / 1 uF RC on EN.
4. **Panel.** 8-bit 8080 bus: IM2-0 strapped `011`, DB0-DB7, WR, DC, RST, TE; CS tied low, RD
   tied high; 33 ohm series on the data lines and WR. The SPI nets (SCK, MOSI, MISO, CS) and
   their test pads TP11-TP14 go; put test pads on WR, DC, D0 and TE instead.
5. **microSD.** Push-push slot, 4-bit SD bus plus card detect, 10k pull-ups on CMD and D0-D3.
   Card cutout in the case wall.
6. **Third 74HC165.** Add it to the chain (24 bits). It takes P1 Start (PWR_BTN_N), P2 Start
   (P2_START), P1 Select, P2 Select, CHG_STAT_N and CHG_PG_N. Tie its 2 spare inputs high.
   The power latch stays discrete; firmware reads Start's 1 s hold through the chain.
7. **USB.** D- and D+ go to IO19 and IO20 (the module's own USB). Firmware debug runs over
   USB Serial/JTAG on the same cable.
8. **Debug and buttons.** SWD (J5, J7) and the UART header (J6) go. J7's pads become 3V3,
   GND, EN and IO0. SW18 (BOOTSEL) becomes BOOT on IO0; SW19 (RESET) pulls EN low. R42 still
   holds PWR_HOLD low through reset, so RESET still turns the board off on the cell.
9. **Audio filter.** LEDC PWM runs from the 80 MHz APB clock: 8 bits is 312.5 kHz, not
   586 kHz. Re-tune the 2-stage RC filter for that carrier (or 7 bits at 625 kHz).

## Pin map (32 of 33 used)

The module exposes GPIO 0-21 and 35-48; the octal PSRAM takes 35-37, leaving 33. Display, SD
and LEDC route through the GPIO matrix, so only ADC and USB are fixed.

| GPIO | Signal | Note |
|---|---|---|
| 0 | spare, to J7's BOOT pad and SW18 | strapping pin: low at reset = download mode |
| 1 | VBAT_SENSE | ADC1_CH0 (ADC1 works with Wi-Fi on; ADC2 does not) |
| 2 | CC1_SENSE | ADC1_CH1 |
| 3 | SR_DATA | JTAG strap, ignored unless eFuses are burnt |
| 4 | CC2_SENSE | ADC1_CH3 |
| 5 | SR_PL | |
| 6 | LCD_DC | |
| 7 | LCD_RST | |
| 8 | AUDIO_PWM | LEDC |
| 9 | BL_PWM | LEDC; R61 keeps the backlight off at reset |
| 10 | LCD_TE | |
| 11 | SD_CMD | |
| 12 | SD_CLK | |
| 13-16 | SD_D0-D3 | |
| 17 | SD_CD | |
| 18 | SR_CLK | 60 us high glitch at power-up, harmless to the 74HC165s |
| 19, 20 | USB D-, D+ | fixed |
| 21 | PWR_HOLD | no glitch at reset; R42 pulls it low |
| 38-44, 47 | LCD D0-D7 | 43/44 carry the ROM's boot log; WR stays still, so the panel ignores it |
| 45 | AMP_EN | strapping pin: must be low at reset (low = amp off) |
| 46 | CHG_OFF | strapping pin: R44's pull-down matches its default |
| 48 | LCD_WR | |

Pin sources: `datasheets/esp32-s3-wroom-1.pdf` pin table and PSRAM footnote,
`datasheets/esp32-s3.pdf` strapping (Table 3-3, 3-4) and power-up glitch (Table 2-2) tables.
SPI-mode SD instead of 4-bit would free 2 more pins (3 spare).

## Open checks

1. Module plus antenna keepout on the board outline and in the case.
2. TLV75733P and microSD slot: LCSC part numbers, stock, Basic or Extended.
3. USB series resistors and ESD on IO19/20: follow the design guidelines.
4. Whether the panel bus holds 25 MHz through the GPIO matrix (firmware side; TRM LCD_CAM
   chapter).
5. EMC: Wi-Fi is on, so the board is an intentional radiator. It relies on the module's
   modular grant (keep to its antenna conditions; a WROOM-1U with our own antenna must use
   an antenna type the grant lists) and carries its FCC ID on the label, plus Part 15B for
   the rest of the board.

## Firmware with Wi-Fi

Plan for the firmware port (mb sw session), from the feasibility research on 2026-10-27:

1. Stack: esp-hal 1.2.2 (`unstable`), esp-rtos 0.4.0 with embassy, esp-radio 0.18.0 for
   Wi-Fi, embassy-net, embedded-tls 0.19.0 (TLS 1.3; `sim3.psim.us` negotiates TLS 1.3 with
   an ECDSA P-256 leaf and ISRG Root X2, so the `rustpki` and `p384` features). Xtensa needs
   the espup toolchain.
2. RAM: the frame (307,200 B) moves to PSRAM, which frees internal SRAM for the Wi-Fi heap
   (about 100 KB) and TLS (about 35 to 40 KB).
3. Showdown: the device opens `wss://sim3.psim.us/showdown/websocket` itself as a guest; the
   WebUSB bridge (`next_revision_proposal.md` section 3) becomes the fallback for places
   without Wi-Fi.
4. Setup: an SSID and password entry screen in the shared core (system UI, not battle UX),
   credentials saved in a flash data partition.
5. Power: about 250 mA with Wi-Fi on and the backlight at a typical level, so about 2.8 hours
   on the 700 mAh cell; use modem sleep between messages. Cap TX power if brownouts show near
   the 3.5 V cutoff.

Last updated: 2026-10-27 18:05 EDT
