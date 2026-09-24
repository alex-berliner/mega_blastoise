# Bring-up

First power on a new board, on the bench, before it goes in the case. Each step
says what to measure and what it proves; stop at the first surprise.

Tools: bench supply with current limit, multimeter, USB-C cable and a USB power
meter, a Raspberry Pi Debug Probe (or any CMSIS-DAP) with JST SH 3-pin cables,
a 103450 cell that has been checked for polarity.

Test pads on the back: TP1 VBUS, TP2 VSYS, TP3 VBAT, TP4 3V3, TP5 GND.

## 0. Unpowered

| Check | Expect | Catches |
|---|---|---|
| Resistance TP1, TP2, TP3, TP4 to TP5 | none below ~1 kOhm (TP4 reads lower as the caps charge) | solder bridges on the QFNs |
| Diode test across J2 | the BQ24074's BAT pin looks like a diode, not a short | reversed or shorted cell connector |
| Look at J2's `+` mark and your cell's red lead | same pin | **reversed cell: the charger does not survive it** |

## 1. USB only, no cell

Plug USB through the power meter.

| Check | Expect | Proves |
|---|---|---|
| TP1 VBUS | 5 V | connector, CC pull-downs (a Type-C charger only turns VBUS on when it sees them) |
| TP2 VSYS | 4.3 to 4.5 V | BQ24074 power path (OUT regulates to 4.4 V) |
| TP4 3V3 | 3.3 V | USB raises PWR_EN through R14/D3; TPS63001 runs |
| input current | under 40 mA | nothing is fighting |
| a USB drive called RP2350 appears on the computer | yes | blank flash boots into BOOTSEL: RP2350B, crystal, USB, 1.1 V core |

Blank flash always lands in BOOTSEL. With firmware already on it, hold SW18 (pinhole
near the left edge) and tap SW19 (reset) to get back there.

## 2. Test firmware

Load a bring-up build (UF2) that does each of these and reports over the UART on J6
(115200 baud, Debug Probe UART cable):

1. Blink D4 (GPIO16): the error LED.
2. Print all 16 buttons (internal pull-ups): short each contact's combs with tweezers.
   The table in [pinmap.md](pinmap.md) says which GPIO is which contact.
3. Read ADC0: with no cell it reads the charger's battery-detect level, not a cell voltage.
4. Read CHG_STAT_N and VIN_GOOD_N: VIN_GOOD_N is low with USB in.
5. SWD: attach with probe-rs or openocd through J5.

## 3. Display

1. Drive LCD_RST low, then high; send the ST7796 init sequence over SPI0 at 20 MHz first.
2. Read ID (0x04) back through LCD_MISO: expect the ST7796 ID bytes. If reads fail but
   writes work, the panel's SPI mode (set inside the module; the RM bit in register 0xB0
   picks SPI or RGB for pixel data) is the first suspect.
3. Fill red, green, blue, white; check pixel order (RGB or BGR bit in 0x36) and rotation.
4. Raise SPI to 62.5 MHz; watch for corruption at the far edge of the panel.
5. Backlight: PWM on GPIO35 at 20 kHz, 100% duty. The voltage across R30 should read
   0.20 V (56 mA through the LEDs). At 0% duty the TPS61160 shuts down.

## 4. Audio

Start the PIO I2S (BCLK GPIO9, LRCLK GPIO10, DIN GPIO8), drive AMP_SD (GPIO11) high,
play a 1 kHz sine at -20 dBFS into the speaker. Check for hiss with the amp on and
silence; drive AMP_SD low and check it goes quiet.

## 5. Cell and charger

1. Unplug USB. Connect the cell. Nothing should turn on (PWR_EN low): measure the
   cell current, expect under 20 uA.
2. Plug USB: D1 (amber) lights while charging; CHG_STAT_N low. Charge current (from the
   meter) about 0.5 A input-limited, up to 740 mA once the firmware selects the 1.34 A
   limit on a 1.5 A or 3 A Type-C charger (see pinmap.md, charger input limit).
3. ADC0 reads VBAT / 2; compare with a meter on TP3.
4. Hot check after 10 minutes of charging: the BQ24074 is warm, not too hot to touch.

## 6. Power switch

With the cell connected and USB unplugged:

1. Press the power button: 3V3 comes up while it's held. The firmware sets PWR_HOLD
   (GPIO2) high at boot: release the button, it stays on.
2. PWR_BTN_N (GPIO3) reads low while the button is held.
3. Firmware drops PWR_HOLD after a long press: the board turns off when the button is
   released. Off current back under 20 uA.
4. Press RESET (SW19) while running on the cell: the board turns off (the hold drops).
