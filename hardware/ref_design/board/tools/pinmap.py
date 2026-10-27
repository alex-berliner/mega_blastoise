"""Write docs/pinmap.md from design.py, so the firmware's view of the board
can't drift from the schematic.

    python3 tools/pinmap.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, BOARD)
import design  # noqa: E402

OUT = os.path.join(BOARD, "..", "docs", "pinmap.md")

IO = {4: 4, 5: 5, 6: 6, 7: 7, 8: 15, 9: 16, 10: 17, 11: 18, 12: 8, 13: 19, 14: 20, 15: 3, 16: 46, 17: 9, 18: 10, 19: 11,
      20: 12, 21: 13, 22: 14, 23: 21, 24: 47, 25: 48, 26: 45, 27: 0, 28: 35, 29: 36, 30: 37, 31: 38, 32: 39, 33: 40,
      34: 41, 35: 42, 36: 44, 37: 43, 38: 2, 39: 1}                      # module pin -> GPIO (datasheet Table 3-1)
ROLE = {
    "PWR_HOLD": "output: set high to stay on (see Power control); low turns the device off once the button is up",
    "AUDIO_PWM": "LEDC, 8 bits at 312.5 kHz (80 MHz / 256): audio through two RC stages (-3 dB at 6.7 kHz) into U7 (see Audio)",
    "AMP_EN": "output: high turns the FM8002A on (Q3 pulls its SHUTDOWN low); strapping pin, low at reset (R37 100k)",
    "CHG_OFF": "output: the BQ24073's CE, high stops charging; strapping pin, low at reset (R44 4.7k pulls it low)",
    "SR_PL": "output: the 74HC165s' SH/LD (U8, U9, U11 pin 1): low loads the 24 inputs, high to shift them out",
    "SR_CLK_MCU": "output: the 74HC165s' clock (SR_CLK) through R65 (33 ohm, at the module end); each rising edge shifts one bit",
    "SR_DATA": "input: U8's Q7, the end of the chain: 24 bits, low = pressed (see Buttons); strapping pin IO3 (JTAG source: leave the eFuse alone)",
    "LCD_DC": "display data/command (D/CX), straight to the panel",
    "LCD_RST": "display reset, active low",
    "LCD_TE": "input: the panel's tearing-effect signal (enable with TEON); start a frame on its edge",
    "LCD_WR_MCU": "LCD_CAM write strobe (WRX), 33 ohm source termination R78",
    "BL_PWM": "LEDC: backlight brightness, duty x 113 mA (filtered to a DC reference, see below); low = off",
    "VBAT_SENSE": "ADC1_CH0: battery voltage / 2 (R17/R18, 470k each, 100 nF)",
    "CC1_SENSE": "ADC1_CH1: USB-C CC1 through 10k; the source's current offer",
    "CC2_SENSE": "ADC1_CH3: USB-C CC2 through 10k",
    "BOOT": "strapping pin IO0: BOOT button SW18 and J7; low at reset enters the ROM's download mode; free as an input afterwards",
    "SD_CMD": "SDMMC CMD (GPIO matrix), 10k pull-up R79", "SD_CLK": "SDMMC CLK (GPIO matrix)",
    "SD_D0": "SDMMC D0, 10k pull-up R80", "SD_D1": "SDMMC D1, 10k pull-up R83", "SD_D2": "SDMMC D2, 10k pull-up R81",
    "SD_D3": "SDMMC D3, 10k pull-up R82", "SD_CD": "card detect: low with a card in, 10k pull-up R84",
    "USB_DM_MCU": "USB D- (IO19, the ROM's USB Serial/JTAG too), 22 ohm R22 to the connector",
    "USB_DP_MCU": "USB D+ (IO20), 22 ohm R21 to the connector",
    "EN": "chip enable: 10k / 1 uF (R23, C12), RESET button SW19 and J7 pull it low",
}
for k in range(8):
    ROLE[f"LCD_D{k}_MCU"] = f"LCD_CAM data bit {k} (panel DB{k}), 33 ohm source termination R{70 + k}" + (
        "; also UART0 TXD0: the ROM's boot log toggles it" if k == 5 else "; also UART0 RXD0" if k == 6 else "")
BUTTON_POS = {  # button net -> contact reference (SWn = case numbering), the contacts on their own GPIOs
    p["pins"]["1"]: p["ref"] for p in design.PARTS if p["sheet"] == "buttons.kicad_sch" and p["ref"].startswith("SW")
}


def main():
    E = design.ESP32
    pin_of = {net: n for n, net in E.items() if net}
    G = {net: IO[n] for net, n in pin_of.items() if n in IO}
    lines = [
        "# ESP32-S3 pin map (reference board, 2-layer build)",
        "",
        "Generated from `board/design.py` by `board/tools/pinmap.py`; edit those, not this file.",
        "",
        "Module: ESP32-S3-WROOM-1U-N16R8 (16 MB flash, 8 MB octal PSRAM; GPIO35-37 are the PSRAM's, not on the board),",
        "Wi-Fi through a U.FL antenna. Firmware: esp-hal / esp-idf-hal for the ESP32-S3. The panel is on the LCD_CAM",
        "peripheral in 8-bit i80 mode, the card on SDMMC (GPIO matrix), buttons through three 74HC165.",
        "",
        "| GPIO | Module pin | Net | Use |",
        "|---|---|---|---|",
    ]
    for n in sorted((n for n in E if n in IO), key=lambda n: IO[n]):
        net = E[n]
        if net is None:
            lines.append(f"| {IO[n]} | {n} | (not connected) | the octal PSRAM's on this module |")
        else:
            lines.append(f"| {IO[n]} | {n} | `{net}` | {ROLE.get(net, '')} |")
    lines += [
        "",
        "Other pins: EN (3) has 10k to 3V3 and 1 uF to ground (about 10 ms rise) and the RESET button; 3V3 (2) has 22 uF",
        "and 100 nF at the pin; GND pins 1, 40 and the 41 pad. Strapping pins at reset: IO0 is high (BOOT released),",
        "IO3 floats (JTAG source: leave the eFuse unburnt), IO45 is low (AMP_EN, R37), IO46 is low (CHG_OFF, R44).",
        "IO43/IO44 (LCD_D5/LCD_D6) are UART0: the ROM's boot log toggles LCD_D5 while LCD_WR stays still, so the",
        "panel sees nothing. USB (IO19/IO20) is the USB Serial/JTAG: flash and log over the board's USB-C; BOOT low at",
        "reset forces the download mode; J7's pads (3V3, EN, IO0, GND) are for a pogo jig.",
        "",
        "## Display (8-bit 8080 bus)",
        "",
        "The ER-TFT035IPS-6 (ILI9488) is strapped IM2-0 = 011 (8080, 8-bit): DB0-DB7 on LCD_D0-D7, WRX, D/CX, RST and TE, CSX tied",
        "low, RDX high. The minimum parallel write cycle is 40 ns (ILI9488 timing table, docs/display_parallel_proposal.md): LCD_CAM at 25 MHz",
        "or less. Pixels are RGB565 sent as two bytes (the panel's 16-bit-in-8 interface) or",
        "RGB666 as three. Probe pads: TP11 LCD_WR, TP12 LCD_DC, TP13 LCD_D0, TP14 LCD_TE.",
        "",
        "## Storage (microSD)",
        "",
        "J8 is a push-pull-free push-push slot with a card-detect switch (SD_CD low = card in, 10k pull-up). 4-bit SDMMC through the",
        f"GPIO matrix: CMD IO{G['SD_CMD']}, CLK IO{G['SD_CLK']}, D0-D3 IO{G['SD_D0']}-IO{G['SD_D3']}, CD IO{G['SD_CD']}; 10k pull-ups on CMD, D0-D3 and CD. The card's supply",
        "SD_VDD comes from 3V3 through JP6 (cut it to measure the card's current). 3.3 V signalling only.",
        "",
        "## Buttons",
        "",
        "Each seat's D-pad and A/B/X/Y (8 contacts to GND) come in through a 74HC165 at that seat (U8 seat 1, U9 seat 2);",
        "a third register U11 carries the Starts, the Selects and the charger's two status lines. The chain is",
        "U11 -> U9 -> U8 -> SR_DATA, 24 bits. A read: SR_PL low then high (parallel load), then 24 rising edges on",
        f"GPIO{G['SR_CLK_MCU']} (SR_CLK through R65); SR_DATA shows each bit before the edge that shifts it: P1 Y first, then P1 X, B, A,",
        "RIGHT, LEFT, DOWN, UP, then seat 2 in the same order (bits 0-15, 1 = released), then U11: bit 16 PWR_BTN_N (seat 1's",
        "Start, through Q1), 17 P2_START (seat 2's Start, through Q7), 18 P1_SELECT, 19 P2_SELECT, 20 CHG_STAT_N, 21 CHG_PG_N, 22 and 23",
        "always 1. A 16-bit read still works (the first 16 bits are unchanged). Between reads SR_PL rests high and the clock low.",
        "Both Starts are power buttons on VSYS (see below); the Selects have 47k pull-ups (R86, R87), the charger lines",
        "100k pull-ups (R9, R10).",
        "",
        "## Power control",
        "",
        "PWR_EN (the TLV75733P's enable, pulled down by R15) is the OR of: seat 1's Start (SW21 joins VSYS",
        "to PWR_BTN_RAW, into PWR_EN through D2), seat 2's Start (SW23 joins VSYS to P2_START_RAW, into",
        "PWR_EN through D8), PWR_HOLD (through D2), and USB VBUS (R14, D3). Either Start powers the",
        "device on; the firmware reads seat 1's as bit 16 of the chain (PWR_BTN_N, Q1) and seat 2's as bit 17 (P2_START, Q7).",
        "",
        "- Turning on: pressing either Start starts the regulator. Firmware sets PWR_HOLD high only once",
        "  that Start has been held for about a second (bits 16/17), so a knock in a bag doesn't turn the device on:",
        "  let go sooner and it powers off again.",
        "- Turning off: firmware sets PWR_HOLD low; power drops when both Starts are released.",
        "  Watch bits 16 and 17 for a long press, then drop PWR_HOLD.",
        "- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs",
        "  while charging: show a charging screen, turn the backlight off (BL_PWM low) and sleep.",
        "- A reset or crash releases PWR_HOLD (an input again) and R42 (4.7k) pulls it low, so the device turns off",
        "  unless USB is connected.",
        "- Enable the watchdog before any blocking init: a hang with PWR_HOLD high otherwise keeps the",
        "  device on with no way to turn it off but unplugging the cell.",
        "- Wi-Fi transmit peaks draw 355 mA (module datasheet, via docs/esp32s3_board_proposal.md) from 3V3 (TLV75733P: 1 A): the 22 uF at the module pin carries the",
        "  burst; with the cell under 3.5 V the reading and Wi-Fi are both near the dropout, so shut down at 3.50 V.",
        "",
        "## Pins at reset",
        "",
        "GPIOs reset as high-impedance inputs without pulls, except the strapping pins above (Table 4-1 of the module datasheet). The",
        "external 100k pull-ups on the charger lines and the 4.7k/100k pull-downs on PWR_HOLD, CHG_OFF and AMP_EN decide the",
        "level until firmware runs. SR_PL and SR_CLK float until driven: drive SR_PL high and SR_CLK low before the first read.",
        "BL_PWM floats; R61 holds BL_REF at 0, so the backlight is off. The board revision is on the silkscreen only.",
        "",
        "## Backlight (current sinks on BL_PWM)",
        "",
        f"GPIO{G['BL_PWM']} runs an LEDC channel, for example 12 bits at 19.5 kHz (80 MHz / 4096). R60/R61/C50 filter",
        "it into BL_REF (0.124 V at 100%, time constant 3.75 ms), and each cathode pin's sink holds",
        "its 2.2 ohm sense resistor at BL_REF: 56 mA a pin at 100% duty, 113 mA for the six LEDs",
        "(the panel's maximum is 120 mA; its typical rating, 15 mA per LED, is 80% duty). The LED",
        "current is DC, so the PWM frequency only sets the ripple (about 0.6% at 19.5 kHz, scaled from the 0.3% at 36.6 kHz",
        "that board/sim/backlight.py gave; not re-simulated). Current is",
        "proportional to duty; apply a gamma curve in firmware. Keep the lowest level at about 18%",
        "duty (3.4 mA per LED): below it the op-amp's offset (up to 4.5 mV against 22 mV) can make",
        "the two halves of the backlight visibly unequal. A brightness change settles in about",
        "15 ms (the filter); from off, the sinks start within about 1 ms and then follow BL_REF.",
        "Off = duty 0. Simulated with ngspice before layout: board/sim/backlight.py (results in",
        "board/sim/backlight_results.txt).",
        "",
        "## Charger (BQ24073) and USB power",
        "",
        "| CHG_PG_N (bit 21) | CHG_STAT_N (bit 20) | State |",
        "|---|---|---|",
        "| low | low | charging (270 mA, 242-295, set by R5) |",
        "| low | high | charge done, or paused: the cell outside 3.5-47 C (TH1) or CHG_OFF high |",
        "| low | 2 Hz | safety timer expired (30 min pre-charge, 5 h fast charge) |",
        "| high | high | no USB power |",
        "",
        "TH1 sits on the board at the cell's hole, against the cell's end; it lags the cell.",
        "",
        "The BQ24073 is a power-path charger: with USB in it runs the system from USB (VSYS at",
        "4.4 V) and charges with what is left, inside a 475 mA input limit (USB500, set by R43 on",
        "EN1); unplugged, VSYS follows the cell through its battery FET with no gap. A default-USB",
        "port is safe, but a heavy load (Wi-Fi, backlight) leaves little for charging: read the source's offer and",
        "dim the backlight on a weak one. With the 5.1k Rd on each CC, the CC line the cable uses",
        "sits at 0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A;",
        "take the higher of CC1_SENSE and CC2_SENSE.",
        "",
        "## Audio (LEDC into the FM8002A)",
        "",
        f"GPIO{G['AUDIO_PWM']} is an LEDC channel on its own timer (not the backlight's). Run it at 8 bits: at 80 MHz that's a 312.5 kHz carrier, which R32/C37 and",
        "R33/C38 (1.5k, 4.7 nF; board/sim/audio_filter.py) take 46 dB down, with the audio band -3 dB at about 6.7 kHz. Not 10 bits:",
        "78 kHz would sit much nearer the audio band (dither or noise-shape for more resolution). C39",
        "blocks the DC, and U7 drives the speaker bridge-tied at about 1.8x (R35/R34, with the filter's",
        "3k in front of R34). VSYS (the BQ24073's output) feeds it through JP3: about 0.5 W full scale",
        "on USB and 0.4-0.5 W on the cell, and it clips below a 3.7 V cell: back the amplitude off there.",
        "Idle at half scale so turning on doesn't pop. The FM8002A takes about 100 ms to wake (its",
        "bypass capacitor charges): keep AMP_EN high while the device is awake (about 4.4 mA), or raise",
        "it at least 150 ms before a sound, PWM idling. The speaker (15 x 10 mm, 8 ohm) lies on the",
        "board's top in the right bezel beside the screen and plugs into J4.",
        "",
        "## Battery voltage",
        "",
        f"VBAT = 2 x the ADC1 reading on GPIO{G['VBAT_SENSE']} (ADC1_CH0; ADC1 only: ADC2 cannot be used while Wi-Fi runs), 12-bit, attenuation",
        "12 dB (input range about 0 to 3.1 V; full charge reads about 2.10 V at the pin), with the eFuse calibration. Unlike the",
        "RP2350 board, the ADC does not use the 3V3 rail as its reference. Full charge reads about 2.10 V at the pin.",
        "",
        "Shut down at 3.50 V as computed, filtered and under load, never lower: once the TLV75733P drops out (a cell",
        "under about 3.4 V on a Wi-Fi burst) the rail falls with the cell, and 3.50 V also keeps 3V3 above the",
        "module's brown-out threshold as the load steps.",
        "",
        "Calibrate the gain once: when CHG_STAT_N goes high at the end of a charge (CHG_PG_N low, TH1",
        "in range, CHG_OFF low) the cell is at the BQ24073's 4.2 V +-1%. Only a full charge counts:",
        "the charger doesn't terminate while the input limit or its thermal limit is cutting the",
        "charge current.",
        "",
        "The divider's source impedance is 235k with C8 (100 nF) across the lower leg: average several samples a second",
        "rather than a single read, and compare with a meter at bring-up (unverified: the ESP32-S3 ADC's input leakage",
        "against 235k was not checked against its datasheet).",
    ]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    open(OUT, "w").write("\n".join(lines) + "\n")
    print("wrote", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
