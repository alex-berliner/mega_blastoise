# Bring-up

First power on a new board, on the bench, before it goes in the case. Each step
says what to measure and what it proves; stop at the first surprise.

Tools: bench supply with current limit, multimeter, an oscilloscope (for step 5's
unplug test), USB-C cable and a USB power meter, a Raspberry Pi Debug Probe (or any
CMSIS-DAP) with JST SH 3-pin cables, a 403450 cell that has been checked for polarity.

Test pads on the back: TP1 VBUS, TP2 VSYS, TP3 VBAT, TP4 3V3, TP5 GND, TP9 1V1 (the core's
own rail, a pad on its via left of the chip), TP11-TP14 the panel's SCK, MOSI, CS and DC (a logic
analyser's four channels, under J3's lead-in). On top: TP6 PWR_EN, TP7 LS_GATE, TP8 AMP_SHDN,
TP10 BL_REF (the backlight's set point, by U6), and TP15/TP16, two GND pads for a scope's ground
clip (by the charger and by the amplifier).

**SWD without the cable or USB:** J7 on the back is a row of six 1.6 mm pads at 2.54 mm, named
on the silkscreen: 3V3, CLK (SWCLK), DIO (SWDIO), RUN, BOOT (BOOTSEL), GND. A pogo-pin jig, a
clip, or six soldered wires reach the chip there; it carries the same SWD as J5. Power through
the 3V3 pad only with the cell out and USB unplugged (it feeds the 3V3 rail directly).
Shorting BOOT to GND while tapping RUN to GND puts the chip in its USB boot loader.

## 0. Unpowered

| Check | Expect | Catches |
|---|---|---|
| Resistance TP1, TP2, TP3, TP4 to TP5 | none below ~1 kOhm (TP4 reads lower as the caps charge) | solder bridges on the fine-pitch parts |
| Diode test from TP3 (VBAT) to TP2 (VSYS) | one diode drop (Q2's body diode), not a short | a shorted or misplaced Q2 |
| Look at J2's `+` mark and your cell's red lead | same pin | **reversed cell: the charger does not survive it** |

Five solder jumpers come bridged from the factory; cut one with a knife to isolate a fault
or to put a meter in series, bridge it again with a blob of solder:

- **JP1** (by U3) carries the 3.3 V regulator's output onto the 3V3 rail. Cut it to power
  the rail from a bench supply on TP4, or to measure the rail's draw through a meter across
  the pads.
- **JP2** (right of J3) carries VSYS to the panel's LED anode (LED_A). Cut it to run the
  panel with its backlight off, or to measure the backlight current.
- **JP3** (by C34, above the amplifier) carries VSYS to the amplifier (AMP_VDD). Cut it to
  take a noisy or shorted amplifier off the board, or to measure what it draws.
- **JP4** (above C29/C30, by J3) carries 3V3 to the panel's logic (LCD_3V3). Cut it to run
  the board with the panel unpowered, or to read the panel's logic current. With it open,
  firmware must hold the SPI pins low: the panel's inputs would feed it otherwise.
- **JP5** (right of J2) joins the cell to the board (CELL_P to VBAT). Cut it and bridge it with
  an ammeter to read everything the cell gives or takes: sleep current, charge current.

TP6 (PWR_EN), TP7 (LS_GATE) and TP8 (AMP_SHDN) are bare pads for a scope probe.
C41-C43 are empty 1206 footprints (C41 on the charger's TEMP input, C42/C43 on CC1/CC2): fit one by hand only
if bring-up shows noise there; see [assembly.md](assembly.md).

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

1. Print all 20 buttons (internal pull-ups): short each contact's combs with tweezers.
   The table in [pinmap.md](pinmap.md) says which GPIO is which contact; seat 1's Start
   (the power button) reads on GPIO3 (PWR_BTN_N).
2. Read ADC7 (GPIO47): with no cell it reads whatever the charger leaves on its output, not a
   cell voltage.
3. Read CHG_STAT_N and CHG_DONE_N: with USB in and no cell the TP4056 has nothing to
   charge, so they may flicker; that's expected.
4. SWD: attach with probe-rs or openocd through J5, then again through J7's pads.
5. Read REV3..REV0 (GPIO25-22, pull-ups on) as a number: 1 on this board (the REV on the
   back silkscreen).

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
5. Backlight: start PWM10 A on GPIO44 (BL_PWM) at 80% duty (12 bits at 36.6 kHz): BL_REF
   (C50) settles at about 99 mV within 15 ms, and each cathode pin draws 45 mA (15 mA per
   LED, three LEDs a pin), 90 mA in all through JP2. Check it on the sense resistors:
   about 99 mV across R52 (LED_K1) and R53 (LED_K2), within a few mV of each other, and
   the panel lit evenly. 100% duty is 113 mA (0.124 V), the panel's limit is 120 mA. Step
   the duty 18% <-> 95% and scope R52: no ringing (simulated phase margin 72 deg or more,
   board/sim/backlight_results.txt). Duty 0 (or GPIO44 low) turns it off. Also check, on a
   nearly flat cell (3.5 V) at 100%, that the drop across R52 stays at 0.124 V (headroom);
   and plug USB in with the backlight on and watch for a flash (simulation: a few us at
   up to 2.4x when VSYS jumps by 1.9 V in 2 us).
6. Tearing effect: enable it on the panel (TEON, 0x35) and watch GPIO32: one pulse per
   frame, about 60 Hz.

## 4. Audio

Start PWM4 A on GPIO8 (8 bits, about 586 kHz), idle it at half scale, set AMP_EN
(GPIO11) high, then play a 1 kHz sine at -20 dBFS into the speaker. On a scope at C39's
far side the carrier should be gone: a clean sine riding on half the supply. Listen for
hiss in silence; set AMP_EN low and check it goes quiet. Measure the delay from AMP_EN
high to full output (about 100 ms expected). Then, on USB, play a full-scale 1 kHz tone
for 10 minutes and check U7's temperature: if it runs hot, cap the PWM amplitude in
firmware while VSYS is above 4 V.

## 5. Cell, charger and the USB hand-over

1. Unplug USB. Connect the cell. Nothing should turn on (PWR_EN low): measure the cell
   current, expect about 20 uA (the divider, the charger's standby and D5's leakage), under 30.
2. Plug USB: CHG_STAT_N goes low while charging. The meter shows about 0.35 A more than
   the system draws (R5 sets about 333-364 mA). Drive CHG_OFF (GPIO19) high: CHG_STAT_N
   goes high and the extra current stops; low again and charging resumes.
3. ADC7 reads VBAT / 2 against the 3V3 rail: with the cell above about 3.6 V (3V3
   regulated), compare the firmware's computed VBAT with a meter on TP3; they should
   agree within about 50 mV. When CHG_DONE_N goes low (charge done), record the reading:
   that is the calibration point (the cell at 4.2 V +-1%).
4. Hot check after 10 minutes of charging: the TP4056 is warm, not too hot to touch. It
   throttles its own current if its die passes about 145 C. Measure U2 pin 1 (TEMP): about
   0.63 x VBUS at 25 C, falling as TH1 warms (0.45 x VBUS pauses the charge). Down near
   0.5 x VBUS (about 40 C) after 10 minutes means U2's heat is reaching TH1.
4a. Temperature cutoff: warm TH1 (left of the panel connector J3, at its seat-1 end) with a hair dryer, thermocouple
   beside it. Charging pauses (CHG_STAT_N and CHG_DONE_N both high) at about 46 C and
   resumes as it cools. A freezer spray or a cold pack on TH1 pauses it near 1 C.
5. **Unplug test.** Run the firmware from the cell with USB plugged in, full backlight and
   a sound playing, scope on TP4 (3V3) and TP2 (VSYS), trigger on VSYS falling, then pull
   the USB cable. VSYS drops from about 4.6 V to the cell voltage and the firmware must
   keep running. Expect VSYS at the cell less about 0.35 V for 1.5-3.3 ms first (D7,
   until R38 has drained VBUS and Q2 is on), with 3V3 about 0.1 V under that. Repeat with
   the cell at 3.5 V (the cutoff): 3V3 must stay above 2.97 V (the internal flash's
   minimum). If it doesn't, raise the firmware cutoff until it does, and note it for a
   respin (a lower-dropout 3V3 regulator).
6. **Hot plug, no cell.** Cell unplugged, scope on TP1 (VBUS) and TP2 (VSYS), trigger on
   VBUS rising, then plug in an A-to-C cable from an always-on USB-A port (the worst case:
   a USB-C source only switches VBUS on after it sees the board). VBUS must stay under
   8 V (the TP4056's maximum) and VSYS under 6.5 V (the ME6211's). Simulated with R41 in
   place: about 6.7 and 6.2 V with a 2 m cable.
7. **CE at plug-in.** Same setup with the cell connected and the board off, scope on TP6
   (PWR_EN) and TP2 (VSYS): PWR_EN must never be more than 0.3 V above VSYS (the ME6211's
   CE limit). C47 slows PWR_EN to about 1 ms for this; if it still leads, raise C47.

## 6. Power switch

With the cell connected and USB unplugged:

1. Press seat 1's Start button: 3V3 comes up while it's held. The firmware sets PWR_HOLD
   (GPIO2) high once Start has been held for about a second: release it, it stays on.
   Release before that and the board turns off again.
2. PWR_BTN_N (GPIO3) reads low while Y is held.
3. Firmware drops PWR_HOLD after a long press: the board turns off when Y is released.
   Off current back under 20 uA.
4. Press RESET (SW19) while running on the cell: the board turns off (the hold drops).
