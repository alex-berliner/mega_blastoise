# RP2350B pin map (reference board)

Generated from `board/design.py` by `board/tools/pinmap.py`; edit those, not this file.

Firmware: embassy-rp with the `rp235xb` feature (the current firmware builds for `rp235xa`,
the 60-pin RP2350A on a Pico 2). Buttons read low when pressed; enable the internal pull-ups.

| GPIO | Net | Use |
|---|---|---|
| 0 | `UART_TX` | UART0 TX to J6 pin 1 (debug console) |
| 1 | `UART_RX` | UART0 RX from J6 pin 3 |
| 2 | `PWR_HOLD` | output: set high first thing at boot (the user is still holding the button); low turns the device off once the button is up |
| 3 | `PWR_BTN_N` | input, pulled up (R16): low while the power button is pressed |
| 4 | `P2_UP` | membrane contact SW9, input with pull-up, low = pressed |
| 5 | `P2_DOWN` | membrane contact SW10, input with pull-up, low = pressed |
| 6 | `P2_LEFT` | membrane contact SW11, input with pull-up, low = pressed |
| 7 | `P2_RIGHT` | membrane contact SW12, input with pull-up, low = pressed |
| 8 | `I2S_DIN` | PIO I2S data to U7 |
| 9 | `I2S_BCLK` | PIO I2S bit clock (side-set base) |
| 10 | `I2S_LRCLK` | PIO I2S word clock (side-set base + 1: embassy-rp's PioI2sOut wants BCLK, BCLK+1) |
| 11 | `AMP_SD` | output: high = amplifier on, left channel (SD_MODE ~2.2 V through R32); low = shutdown, 0.6 uA |
| 12 | `P2_A` | membrane contact SW13, input with pull-up, low = pressed |
| 13 | `P2_B` | membrane contact SW14, input with pull-up, low = pressed |
| 14 | `P2_X` | membrane contact SW15, input with pull-up, low = pressed |
| 15 | `P2_Y` | membrane contact SW16, input with pull-up, low = pressed |
| 16 | `STATUS_LED` | output: high lights D4, the red error LED |
| 17 | `CHG_STAT_N` | input, pulled up (R10): low while the BQ24074 charges; also lights D1 |
| 18 | `VIN_GOOD_N` | input, pulled up (R9): low while USB power is present |
| 19 | `CHG_EN1` | output: charger input limit, see below (pulled up by R8 until driven) |
| 20 | `CHG_EN2` | output: charger input limit, see below (pulled down by R7 until driven) |
| 21 | (spare) | not connected |
| 22 | (spare) | not connected |
| 23 | (spare) | not connected |
| 24 | `P1_A` | membrane contact SW5, input with pull-up, low = pressed |
| 25 | `P1_B` | membrane contact SW6, input with pull-up, low = pressed |
| 26 | `P1_X` | membrane contact SW7, input with pull-up, low = pressed |
| 27 | `P1_Y` | membrane contact SW8, input with pull-up, low = pressed |
| 28 | `P1_UP` | membrane contact SW1, input with pull-up, low = pressed |
| 29 | `P1_DOWN` | membrane contact SW2, input with pull-up, low = pressed |
| 30 | `P1_LEFT` | membrane contact SW3, input with pull-up, low = pressed |
| 31 | `P1_RIGHT` | membrane contact SW4, input with pull-up, low = pressed |
| 32 | (spare) | not connected |
| 33 | `LCD_DC` | display data/command |
| 34 | `LCD_RST` | display reset, active low |
| 35 | `BL_PWM` | PWM to the TPS61160 CTRL pin: backlight on/off and dimming, 5 to 100 kHz (PWM5B) |
| 36 | `LCD_MISO` | SPI0 RX (panel readback, optional) |
| 37 | `LCD_CS` | SPI0 CSn (or drive as a GPIO) |
| 38 | `LCD_SCK_MCU` | SPI0 SCK, 33 ohm source termination R28 |
| 39 | `LCD_MOSI_MCU` | SPI0 TX, 33 ohm source termination R29 |
| 40 | `VBAT_SENSE` | ADC0: battery voltage / 2 (R17/R18, 470k each, 100 nF) |
| 41 | `CC1_SENSE` | ADC1: USB-C CC1 through 10k; the source's current offer |
| 42 | `CC2_SENSE` | ADC2: USB-C CC2 through 10k |
| 43 | (spare) | not connected |
| 44 | (spare) | not connected |
| 45 | (spare) | not connected |
| 46 | (spare) | not connected |
| 47 | (spare) | not connected |

Other RP2350B pins: QSPI flash (W25Q128JVP, 16 MB), 12 MHz crystal, USB to the USB-C
(27 ohm series, USBLC6 ESD), SWD and RUN to J5 and the RESET button, QSPI_SS to the BOOTSEL
button through 1k.

## Power control

PWR_EN (the 3V3 regulator's enable) is the OR of: the power button, PWR_HOLD, and USB VBUS.

- Turning on: the button starts the regulator; firmware must set PWR_HOLD high before the
  user lets go (hold time is the boot time, well under a second).
- Turning off: firmware sets PWR_HOLD low; power drops when the button is released.
  Watch PWR_BTN_N for a long press, then drop PWR_HOLD.
- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs
  while charging: show a charging screen, turn the backlight off (BL_PWM low) and sleep.
- A reset or crash releases PWR_HOLD (GPIOs float, pulled low), so the device turns off
  unless USB is connected.

## Charger input limit (BQ24074 EN2, EN1)

| CHG_EN2 | CHG_EN1 | USB input limit |
|---|---|---|
| 0 | 1 | 500 mA (state at power-up: R7 pulls EN2 low, R8 pulls EN1 high) |
| 1 | 0 | 1.34 A (ILIM resistor R5): only on a Type-C source offering 1.5 A or 3 A |
| 0 | 0 | 100 mA |
| 1 | 1 | suspend (no input current) |

Reading the source's offer: with the 5.1k Rd on each CC, the CC line the cable uses sits at
0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A. Take the
higher of CC1_SENSE and CC2_SENSE; above about 0.66 V it is safe to select 1.34 A.
Fast-charge current is fixed at 740 mA by R6 either way.

## Battery voltage

VBAT = 2 x ADC0 x 3.3 V / 4096 (12-bit). Read a few samples; the 100 nF holds the divider
steady for the ADC's sampling capacitor. Full charge reads about 2.10 V at the pin.
