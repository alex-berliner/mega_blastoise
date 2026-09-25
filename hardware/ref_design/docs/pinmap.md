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
| 8 | `AUDIO_PWM` | PWM4 A: audio, through two 16 kHz RC poles into U7 (see Audio) |
| 9 | (spare) | not connected |
| 10 | (spare) | not connected |
| 11 | `AMP_EN` | output: high turns the FM8002A on (Q3 pulls its SHUTDOWN low); low or floating = off (4 uA) |
| 12 | `P2_A` | membrane contact SW13, input with pull-up, low = pressed |
| 13 | `P2_B` | membrane contact SW14, input with pull-up, low = pressed |
| 14 | `P2_X` | membrane contact SW15, input with pull-up, low = pressed |
| 15 | `P2_Y` | membrane contact SW16, input with pull-up, low = pressed |
| 16 | (spare) | not connected |
| 17 | `CHG_STAT_N` | input, pulled up (R10): low while the TP4056 charges |
| 18 | `CHG_DONE_N` | input, pulled up (R9): low when the charge is done (USB present, cell full) |
| 19 | (spare) | not connected |
| 20 | `UART_TX` | UART1 TX to J6 pin 1 (debug console) |
| 21 | `UART_RX` | UART1 RX from J6 pin 3 |
| 22 | (spare) | not connected |
| 23 | (spare) | not connected |
| 24 | (spare) | not connected |
| 25 | (spare) | not connected |
| 26 | (spare) | not connected |
| 27 | (spare) | not connected |
| 28 | `P1_UP` | membrane contact SW1, input with pull-up, low = pressed |
| 29 | `P1_DOWN` | membrane contact SW2, input with pull-up, low = pressed |
| 30 | `P1_LEFT` | membrane contact SW3, input with pull-up, low = pressed |
| 31 | `P1_RIGHT` | membrane contact SW4, input with pull-up, low = pressed |
| 32 | `LCD_TE` | input: the panel's tearing-effect signal (enable with TEON); start a frame on its edge |
| 33 | `LCD_CS` | SPI0 CSn (or drive as a GPIO) |
| 34 | (spare) | not connected |
| 35 | (spare) | not connected |
| 36 | `LCD_MISO` | SPI0 RX (panel readback, optional) |
| 37 | `LCD_DC` | display data/command |
| 38 | `LCD_SCK_MCU` | SPI0 SCK, 33 ohm source termination R28 |
| 39 | `LCD_MOSI_MCU` | SPI0 TX, 33 ohm source termination R29 |
| 40 | `LCD_RST` | display reset, active low |
| 41 | `P1_B` | membrane contact SW6, input with pull-up, low = pressed |
| 42 | `P1_X` | membrane contact SW7, input with pull-up, low = pressed |
| 43 | `P1_A` | membrane contact SW5, input with pull-up, low = pressed |
| 44 | `BL_SDI` | output to the AP3156 SDI (1-wire): backlight current in 32 steps, see below; low > 256 us = off |
| 45 | `CC2_SENSE` | ADC5: USB-C CC2 through 10k |
| 46 | `CC1_SENSE` | ADC6: USB-C CC1 through 10k; the source's current offer |
| 47 | `VBAT_SENSE` | ADC7: battery voltage / 2 (R17/R18, 470k each, 100 nF) |

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
  while charging: show a charging screen, turn the backlight off (BL_SDI low) and sleep.
- A reset or crash releases PWR_HOLD (GPIOs float, pulled low), so the device turns off
  unless USB is connected.

## Backlight (AP3156 on BL_SDI)

SDI idles high. A command is a burst of low pulses (each low 0.3 to 50 us, high 0.05 to
50 us), ended by holding SDI high for more than 256 us; holding it low for more than 256 us
shuts the driver down, and the next rising edge wakes it. Brightness takes two bursts:
10 pulses (all six channels), then N pulses for current level N: 1 = 20 mA per LED (the
panel's maximum), 6 = 15 mA (its typical rating), down to 31 = 3.6 mA; 32 = off; about
0.5 dB a step. The driver starts at 0 mA and R31 holds SDI low, so the backlight stays off
until firmware sets a level. Command table: AP3156 datasheet, Serial Digital Interface.

## Charger (TP4056) and USB power

| CHG_STAT_N | CHG_DONE_N | State |
|---|---|---|
| low | high | charging (about 460-500 mA, set by R5) |
| high | low | charge done |
| high | high | no USB power |

With USB in, the system runs from USB through D5 and the cell only charges; unplugged,
Q2 connects the cell. The charger has no input current limit: a default-USB port (500 mA)
can be asked for charge current plus the system's draw, so read the source's offer and
dim the backlight on a weak one. With the 5.1k Rd on each CC, the CC line the cable uses
sits at 0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A;
take the higher of CC1_SENSE and CC2_SENSE.

## Audio (PWM into the FM8002A)

GPIO8 is PWM4 A. At 150 MHz, 10 bits gives a 146 kHz carrier (8 bits: 586 kHz); R32/C37 and
R33/C38 (two poles at 16 kHz) take it out, C39 blocks the DC, and U7 drives the speaker
bridge-tied at a gain of 2 (R35/R34). Idle at half scale so turning on doesn't pop, and set
AMP_EN high only while sound plays: the amplifier draws about 4.4 mA when on.

## Battery voltage

VBAT = 2 x ADC7 x 3.3 V / 4096 (12-bit). Read a few samples; the 100 nF holds the divider
steady for the ADC's sampling capacitor. Full charge reads about 2.10 V at the pin. Shut down
at about 3.35 V: below that the ME6211 can't hold 3.3 V, and the RP2354B's regulator needs
VREG_AVDD at 3.135 V or more.
