# Bring-up

First power on a new board, on the bench, before it goes in the case. Each step
says what to measure and what it proves; stop at the first surprise.

Tools: bench supply with current limit, multimeter, an oscilloscope (for step 5's
unplug test), USB-C cable and a USB power meter, a Raspberry Pi Debug Probe (or any
CMSIS-DAP) with JST SH 3-pin cables, a 503450 cell that has been checked for polarity.

Test pads on the back: TP1 VBUS, TP2 VSYS, TP3 VBAT, TP4 3V3, TP5 GND.

## 0. Unpowered

| Check | Expect | Catches |
|---|---|---|
| Resistance TP1, TP2, TP3, TP4 to TP5 | none below ~1 kOhm (TP4 reads lower as the caps charge) | solder bridges on the fine-pitch parts |
| Diode test from TP3 (VBAT) to TP2 (VSYS) | one diode drop (Q2's body diode), not a short | a shorted or misplaced Q2 |
| Look at J2's `+` mark and your cell's red lead | same pin | **reversed cell: the charger does not survive it** |

## 1. USB only, no cell

Plug USB through the power meter.

| Check | Expect | Proves |
|---|---|---|
| TP1 VBUS | 5 V | connector, CC pull-downs (a Type-C charger only turns VBUS on when it sees them) |
| TP2 VSYS | 4.5 to 4.7 V | D5 feeds the system from USB (VBUS less a Schottky drop) |
| TP4 3V3 | 3.3 V | USB raises PWR_EN through R14/D3; the ME6211 runs |
| input current | under 40 mA | nothing is fighting |
| a USB drive called RP2350 appears on the computer | yes | the RP2354B's flash is blank from the factory, so it boots into BOOTSEL: chip, crystal, USB, 1.1 V core |

Blank flash always lands in BOOTSEL. With firmware already on it, hold SW18 (pinhole
near the left edge) and tap SW19 (reset) to get back there.

## 2. Test firmware

Load a bring-up build (UF2) that does each of these and reports over the UART on J6
(115200 baud, Debug Probe UART cable):

1. Print all 16 buttons (internal pull-ups): short each contact's combs with tweezers.
   The table in [pinmap.md](pinmap.md) says which GPIO is which contact; seat 1's Y
   (the power button) reads on GPIO3 (PWR_BTN_N).
2. Read ADC7 (GPIO47): with no cell it reads whatever the charger leaves on its output, not a
   cell voltage.
3. Read CHG_STAT_N and CHG_DONE_N: with USB in and no cell the TP4056 has nothing to
   charge, so they may flicker; that's expected.
4. SWD: attach with probe-rs or openocd through J5.

## 3. Display

1. Drive LCD_RST low, then high; send the ILI9488 init sequence over SPI0 at 10-20 MHz
   first (the datasheet rates serial writes to 20 MHz).
2. Read ID4 (0xD3) back through LCD_MISO at 5 MHz or less (the ILI9488's serial read
   cycle is 150 ns minimum): expect 0x94 0x88. If nothing reads back, look
   at the FPC first (fully in J3, square, lid shut), then at the interface pins: IM2-0
   (J3 pins 42-44, the panel's 9-7) must be high for 4-wire SPI.
3. Set COLMOD (0x3A) to 0x66: in SPI mode the ILI9488 only takes 18-bit colour, three
   bytes a pixel. Fill red, green, blue, white; check pixel order (RGB or BGR bit in 0x36)
   and rotation.
4. Raise SPI step by step; watch for corruption at the far edge of the panel. A full
   frame is 461 kB, so redraw only what changes.
5. Backlight: drive GPIO44 (BL_SDI) high, send 10 low pulses (1 us low, 1 us high), wait
   300 us, then 6 pulses and another 300 us: current level 6, 15 mA per sink, so 45 mA on
   each of the panel's two cathode pins (three LEDs each). The panel should light evenly;
   the AP3156's VOUT (C32) sits a few hundred mV above the LEDs' forward voltage (about
   3.0 V). Level 1 is 20 mA per LED, the panel's 120 mA maximum. Hold GPIO44 low for
   300 us and the backlight goes off.
6. Tearing effect: enable it on the panel (TEON, 0x35) and watch GPIO32: one pulse per
   frame, about 60 Hz.

## 4. Audio

Start PWM4 A on GPIO8 (10 bits, about 146 kHz), idle it at half scale, set AMP_EN
(GPIO11) high, then play a 1 kHz sine at -20 dBFS into the speaker. On a scope at C39's
far side the carrier should be gone: a clean sine riding on half the supply. Listen for
hiss in silence; set AMP_EN low and check it goes quiet. Measure the delay from AMP_EN
high to full output (about 100 ms expected). Then, on USB, play a full-scale 1 kHz tone
for 10 minutes and check U7's temperature: if it runs hot, cap the PWM amplitude in
firmware while VSYS is above 4 V.

## 5. Cell, charger and the USB hand-over

1. Unplug USB. Connect the cell. Nothing should turn on (PWR_EN low): measure the cell
   current, expect under 20 uA.
2. Plug USB: CHG_STAT_N goes low while charging. The meter shows about 0.5 A more than
   the system draws (R5 sets about 460-500 mA).
3. ADC7 reads VBAT / 2; compare with a meter on TP3.
4. Hot check after 10 minutes of charging: the TP4056 is warm, not too hot to touch. It
   throttles its own current if its die passes about 145 C.
5. **Unplug test.** Run the firmware from the cell with USB plugged in, scope on TP4 (3V3)
   and TP2 (VSYS), trigger on VSYS falling, then pull the USB cable. VSYS drops from about
   4.6 V to the cell voltage and the firmware must keep running. Expect VSYS to sit at
   the cell voltage less about 0.5 V for 3-8 ms first (Q2's body diode, until R38 has
   drained VBUS), with 3V3 about 0.1 V under that. Try it with a nearly flat cell too.
   If it resets (the device then turns off), fit a Schottky from VBAT to VSYS across Q2
   (a B5819W, as D5, anode to VBAT) and try again.

## 6. Power switch

With the cell connected and USB unplugged:

1. Press seat 1's Y button: 3V3 comes up while it's held. The firmware sets PWR_HOLD
   (GPIO2) high once Y has been held for about a second: release it, it stays on.
   Release before that and the board turns off again.
2. PWR_BTN_N (GPIO3) reads low while Y is held.
3. Firmware drops PWR_HOLD after a long press: the board turns off when Y is released.
   Off current back under 20 uA.
4. Press RESET (SW19) while running on the cell: the board turns off (the hold drops).
