# RP2354B pin map (reference board, 2-layer build)

Generated from `board/design.py` by `board/tools/pinmap.py`; edit those, not this file.

Firmware: embassy-rp with the `rp235xb` feature (the current firmware builds for `rp235xa`,
the 60-pin RP2350A on a Pico 2). Buttons read low when pressed; enable the internal pull-ups.

| GPIO | Net | Use |
|---|---|---|
| 0 | (spare) | not connected |
| 1 | (spare) | not connected |
| 2 | `PWR_HOLD` | output: set high to stay on (see Power control); low turns the device off once the button is up |
| 3 | `PWR_BTN_N` | input, pulled up (R16): low while seat 1's Y is pressed. Y is the power button, so read seat 1's Y here |
| 4 | `P2_UP` | membrane contact SW9, input with pull-up, low = pressed |
| 5 | `P2_DOWN` | membrane contact SW10, input with pull-up, low = pressed |
| 6 | `P2_LEFT` | membrane contact SW11, input with pull-up, low = pressed |
| 7 | `P2_RIGHT` | membrane contact SW12, input with pull-up, low = pressed |
| 8 | (spare) | not connected |
| 9 | `P2_SELECT` | membrane contact SW22, input with pull-up, low = pressed |
| 10 | `P2_START` | membrane contact SW23, input with pull-up, low = pressed |
| 11 | (spare) | not connected |
| 12 | `P2_A` | membrane contact SW13, input with pull-up, low = pressed |
| 13 | `P2_B` | membrane contact SW14, input with pull-up, low = pressed |
| 14 | `P2_X` | membrane contact SW15, input with pull-up, low = pressed |
| 15 | `P2_Y` | membrane contact SW16, input with pull-up, low = pressed |
| 16 | (spare) | not connected |
| 17 | `CHG_STAT_N` | input, pulled up (R10): low while the TP4056 charges |
| 18 | `CHG_DONE_N` | input, pulled up (R9): low when the charge is done (USB present, cell full) |
| 19 | `CHG_OFF` | output: high pauses charging (Q4 pulls the TP4056's CE low); low or released charges (R44 4.7k pulls it low) |
| 20 | `UART_TX` | UART1 TX to J6 pin 3 (debug console; Raspberry Pi debug connector pinout) |
| 21 | `UART_RX` | UART1 RX from J6 pin 1 |
| 22 | `REV0` | input, pull-up: board revision bit 0 (1 = R48 unfitted, 0 = R48 fitted to GND) |
| 23 | `REV1` | input, pull-up: board revision bit 1 (R49) |
| 24 | `REV2` | input, pull-up: board revision bit 2 (R50) |
| 25 | `REV3` | input, pull-up: board revision bit 3 (R51) |
| 26 | (spare) | not connected |
| 27 | (spare) | not connected |
| 28 | `P1_UP` | membrane contact SW1, input with pull-up, low = pressed |
| 29 | `P1_DOWN` | membrane contact SW2, input with pull-up, low = pressed |
| 30 | `P1_LEFT` | membrane contact SW3, input with pull-up, low = pressed |
| 31 | `P1_RIGHT` | membrane contact SW4, input with pull-up, low = pressed |
| 32 | `LCD_TE` | input: the panel's tearing-effect signal (enable with TEON); start a frame on its edge |
| 33 | `LCD_CS` | SPI0 CSn (or drive as a GPIO) |
| 34 | `P1_Y` | membrane contact SW8, input with pull-up, low = pressed |
| 35 | `P1_SELECT` | membrane contact SW20, input with pull-up, low = pressed |
| 36 | `LCD_MISO` | SPI0 RX (panel readback, optional) |
| 37 | `LCD_DC` | display data/command |
| 38 | `LCD_SCK_MCU` | SPI0 SCK, 33 ohm source termination R28 |
| 39 | `LCD_MOSI_MCU` | SPI0 TX, 33 ohm source termination R29 |
| 40 | `LCD_RST` | display reset, active low |
| 41 | `P1_B` | membrane contact SW6, input with pull-up, low = pressed |
| 42 | `P1_X` | membrane contact SW7, input with pull-up, low = pressed |
| 43 | `P1_A` | membrane contact SW5, input with pull-up, low = pressed |
| 44 | `BL_PWM` | PWM10 A: backlight brightness, duty x 70 mA (filtered to a DC reference, see below); low = off |
| 45 | `CC2_SENSE` | ADC5: USB-C CC2 through 10k |
| 46 | `CC1_SENSE` | ADC6: USB-C CC1 through 10k; the source's current offer |
| 47 | `VBAT_SENSE` | ADC7: battery voltage / 2 (R17/R18, 470k each, 100 nF); off with a cell in, the pin sits above IOVDD, clamped by its ESD diode (under 9 uA, accepted) |

Other RP2354B pins: the 2 MB flash is inside the package, so of the QSPI pins only QSPI_SS
is wired (10k pull-up R23, and the BOOTSEL button through 1k). 12 MHz crystal, USB to the
USB-C (22 ohm series, SRV05-4 ESD), SWD and RUN to J5 and the RESET button.

## Power control

PWR_EN (the ME6211's enable) is the OR of: the power button (seat 1's Y contact, which
joins VSYS to PWR_BTN_RAW), PWR_HOLD, and USB VBUS.

- Turning on: pressing Y starts the regulator. Firmware sets PWR_HOLD high only once Y has
  been held for about a second (read on PWR_BTN_N), so a knock in a bag doesn't turn the
  device on: let go sooner and it powers off again.
- Turning off: firmware sets PWR_HOLD low; power drops when the button is released.
  Watch PWR_BTN_N for a long press, then drop PWR_HOLD.
- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs
  while charging: show a charging screen, turn the backlight off (BL_PWM low) and sleep.
- A reset or crash releases PWR_HOLD and R42 (4.7k) pulls it low, so the device turns off
  unless USB is connected. R42 also covers RP2350-E9, which can hold a released pad near 2 V.
- Enable the watchdog before any blocking init: a hang with PWR_HOLD high otherwise keeps the
  device on with no way to turn it off but unplugging the cell.

## Pins at reset

Every GPIO resets with its pull-down on (RP2350 datasheet, Table 1674). Before reading them:
turn on the pull-ups of REV0-REV3 and of the buttons, and turn off the pull-downs of
PWR_BTN_N, CHG_STAT_N and CHG_DONE_N (their 100k external pull-ups otherwise sit at about
1-1.7 V against the pull-down). ADC pins (VBAT_SENSE, CC1/CC2_SENSE): input buffer and
pull-down both off. Board revision = REV3..REV0 read as a binary number (1 on this board).

## Backlight (current sinks on BL_PWM)

GPIO44 runs PWM10 A, for example 12 bits at 36.6 kHz (150 MHz / 4096). R60/R61/C50 filter
it into BL_REF (0.077 V at 100%, time constant 2.3 ms), and each sink (two cathode pins,
two LEDs) holds its 2.2 ohm sense resistor at BL_REF: 35 mA a sink at 100% duty, 70 mA
for the four LEDs (the panel's typical; its maximum is 80 mA). The LED
current is DC, so the PWM frequency only sets the ripple (0.5% at 36.6 kHz). Current is
proportional to duty; apply a gamma curve in firmware. Keep the lowest level at about 29%
duty (5 mA per LED): below it the op-amp's offset (up to 4.5 mV against 22 mV) can make
the two halves of the backlight visibly unequal. A brightness change settles in about
10 ms (the filter); from off, the sinks start within about 1 ms and then follow BL_REF.
GPIO44 resets low (pull-down), R61 holds BL_REF at 0 and the sinks run from 3V3, so the
backlight is off until firmware starts the PWM and whenever 3V3 is off. Off = duty 0.
Simulated with ngspice before layout: board/sim/backlight.py (results in
board/sim/backlight_results.txt).

## Charger (TP4056) and USB power

| CHG_STAT_N | CHG_DONE_N | State |
|---|---|---|
| low | high | charging (about 333-364 mA, set by R5) |
| high | low | charge done |
| high | high | no USB power, or the cell is outside 1-46 C (TH1): charging paused |

Both high with USB in (a CC line reads a source) means the temperature pause: TH1, on the
board over the cell, stopped the charge; it resumes when TH1 is back in range.

With USB in, the system runs from USB through D5 and the cell only charges; unplugged,
Q2 connects the cell. The charger has no input current limit: a default-USB port (500 mA)
can be asked for charge current plus the system's draw, so read the source's offer and
dim the backlight on a weak one. With the 5.1k Rd on each CC, the CC line the cable uses
sits at 0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A;
take the higher of CC1_SENSE and CC2_SENSE.

## Audio

None since 2026-10-21: the cell sits in a hole in the board, and with every machine-placed
part on the top the speaker and its amplifier didn't fit. GPIO8 (PWM4 A, the old AUDIO_PWM) and
GPIO11 (the old AMP_EN) are left unconnected.

## Battery voltage

VBAT = 2 x ADC7 x 3.3 V / 4096 (12-bit), times a calibration gain (below). Full charge reads
about 2.10 V at the pin.

Shut down at 3.50 V as computed, filtered and under load, never lower. The ADC's reference
is the 3V3 rail itself: once the ME6211 drops out (a cell under about 3.4 V) the rail falls
with the cell and the computed VBAT levels off at 3.37-3.43 V whatever the cell really is,
so a lower threshold never fires. 3.50 V also keeps 3V3 above the internal flash's 2.97 V
minimum through a USB unplug (D7 carries that gap).

Calibrate the gain once: when CHG_DONE_N goes low (charge done) the cell is at the TP4056's
4.2 V +-1%. Only a full charge counts: on a weak USB source the cell shares the load, the
charger stays in constant voltage and CHG_DONE_N never goes low.

Reading it: set the pad up once and keep its ADC channel for the whole program (in
embassy-rp 0.8, dropping the `Channel` turns the pad's digital input and pull-down back on,
which drains C8 and reads low; the digital input must stay off, RP2350-E9). Wait about
200 ms after setting it up (the pad resets with its pull-down on; C8 recovers with a 23.5 ms
time constant). Read bursts of up to 16 one-shot samples, at most once a second: free-running
at 500 kS/s would read tens of percent low against this 235k source. Shut down only after
several low bursts in a row. Pad leakage (1 uA maximum, nanoamps in practice) could offset
the reading; bring-up step 5 compares ADC7 with a meter.
