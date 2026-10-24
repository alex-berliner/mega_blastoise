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

ROLE = {
    "PWR_HOLD": "output: set high to stay on (see Power control); low turns the device off once the button is up",
    "PWR_BTN_N": "input, pulled up (R16): low while seat 1's Start (SW21, a power button) is pressed, through Q1",
    "P2_START": "input, pulled up (R63): low while seat 2's Start (SW23, a power button) is pressed, through Q7",
    "AUDIO_PWM": "PWM7 B, 8 bits (586 kHz): audio through two RC stages (-3 dB at about 14 kHz) into U7 (see Audio)",
    "AMP_EN": "output: high turns the FM8002A on (Q3 pulls its SHUTDOWN low); low or floating = off (4 uA)",
    "CHG_STAT_N": "input, pulled up (R10): the BQ24073's CHG, low while charging, 2 Hz on a safety-timer fault",
    "CHG_PG_N": "input, pulled up (R9): the BQ24073's PGOOD, low while USB power is valid",
    "CHG_OFF": "output: the BQ24073's CE, high stops charging; low or released charges (R44 4.7k pulls it low)",
    "SR_PL": "output: the 74HC165s' SH/LD (U8, U9 pin 1): low loads the 16 buttons, high to shift them out",
    "SR_CLK_MCU": "output: the 74HC165s' clock (SR_CLK) through R65 (33 ohm, at the MCU end); each rising edge shifts one button",
    "SR_DATA": "input: U8's Q7, the end of the chain: 16 buttons, low = pressed (see Buttons)",
    "LCD_DC": "display data/command",
    "LCD_RST": "display reset, active low",
    "LCD_TE": "input: the panel's tearing-effect signal (enable with TEON); start a frame on its edge",
    "BL_PWM": "PWM6 A: backlight brightness, duty x 70 mA (filtered to a DC reference, see below); low = off",
    "LCD_MISO": "SPI0 RX (panel readback, optional)",
    "LCD_CS": "SPI0 CSn (or drive as a GPIO)",
    "LCD_SCK_MCU": "SPI0 SCK, 33 ohm source termination R28",
    "LCD_MOSI_MCU": "SPI0 TX, 33 ohm source termination R29",
    "VBAT_SENSE": "ADC7: battery voltage / 2 (R17/R18, 470k each, 100 nF); off with a cell in, the pin sits above IOVDD, clamped by its ESD diode (under 9 uA, accepted)",
    "CC1_SENSE": "ADC6: USB-C CC1 through 10k; the source's current offer",
    "CC2_SENSE": "ADC5: USB-C CC2 through 10k",
}
BUTTON_POS = {  # button net -> contact reference (SWn = case numbering), the contacts on their own GPIOs
    p["pins"]["1"]: p["ref"] for p in design.PARTS if p["sheet"] == "buttons.kicad_sch" and p["ref"].startswith("SW")
}


def pwm_slice(g):
    """RP2350 datasheet Table 3: GPIO0-31 on PWM0-7, GPIO32-47 on PWM8-11 twice over."""
    return f"PWM{(g >> 1) & 7 if g < 32 else 8 + ((g - 32) >> 1 & 3)} {'AB'[g & 1]}"


def main():
    G = {net: g for g, net in design.GPIO.items()}
    assert pwm_slice(G["AUDIO_PWM"])[:-2] != pwm_slice(G["BL_PWM"])[:-2], "audio and backlight on one PWM slice"
    lines = [
        "# RP2354B pin map (reference board, 2-layer build)",
        "",
        "Generated from `board/design.py` by `board/tools/pinmap.py`; edit those, not this file.",
        "",
        "Firmware: embassy-rp with the `rp235xb` feature (the current firmware builds for `rp235xa`,",
        "the 60-pin RP2350A on a Pico 2). Buttons read low when pressed; enable the internal pull-ups.",
        "",
        "| GPIO | Net | Use |",
        "|---|---|---|",
    ]
    for g in range(48):
        net = design.GPIO.get(g)
        if net is None:
            lines.append(f"| {g} | (spare) | not connected |")
            continue
        if net in BUTTON_POS:
            use = f"metal-dome contact {BUTTON_POS[net]} to GND, input with pull-up, low = pressed"
        else:
            use = ROLE.get(net, "")
        lines.append(f"| {g} | `{net}` | {use} |")
    lines += [
        "",
        "Other RP2354B pins: the 2 MB flash is inside the package, so of the QSPI pins only QSPI_SS",
        "is wired (10k pull-up R23, and the BOOTSEL button through 1k). 12 MHz crystal, USB to the",
        "USB-C (27 ohm series R21/R22, SRV05-4 ESD), SWD and RUN to J5 and the RESET button.",
        "",
        "## Buttons",
        "",
        "Each seat's D-pad and A/B/X/Y (8 contacts to GND) come in through a 74HC165 at that seat",
        "(U8 seat 1, U9 seat 2), pulled up by 10k arrays and chained (U9's Q7 into U8's DS). A read:",
        f"SR_PL low then high (parallel load), then 16 rising edges on GPIO{G['SR_CLK_MCU']} (SR_CLK through R65);",
        "SR_DATA shows each bit before the edge that shifts it: P1 Y first, then P1 X, B, A, RIGHT,",
        "LEFT, DOWN, UP, then seat 2 in the same order; 1 = released. Between reads SR_PL rests high",
        f"and the clock low. Start and Select are metal domes on their own GPIOs: both Selects (GPIO{G['P1_SELECT']} seat 1,",
        f"GPIO{G['P2_SELECT']} seat 2) to GND with the pin's pull-up; both Starts are power buttons on VSYS, read",
        f"through a transistor (GPIO{G['PWR_BTN_N']} seat 1, GPIO{G['P2_START']} seat 2), low = pressed.",
        "",
        "## Power control",
        "",
        "PWR_EN (the RT9013's enable, pulled down by R15) is the OR of: seat 1's Start (SW21 joins VSYS",
        "to PWR_BTN_RAW, into PWR_EN through D2), seat 2's Start (SW23 joins VSYS to P2_START_RAW, into",
        "PWR_EN through D8), PWR_HOLD (through D2), and USB VBUS (R14, D3). Either Start powers the",
        f"device on; the MCU reads seat 1's on PWR_BTN_N (GPIO{G['PWR_BTN_N']}, Q1) and seat 2's on P2_START (GPIO{G['P2_START']}, Q7).",
        "",
        "- Turning on: pressing either Start starts the regulator. Firmware sets PWR_HOLD high only once",
        "  that Start has been held for about a second (read on PWR_BTN_N or P2_START), so a knock in a",
        "  bag doesn't turn the device on: let go sooner and it powers off again.",
        "- Turning off: firmware sets PWR_HOLD low; power drops when both Starts are released.",
        "  Watch PWR_BTN_N and P2_START for a long press, then drop PWR_HOLD.",
        "- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs",
        "  while charging: show a charging screen, turn the backlight off (BL_PWM low) and sleep.",
        "- A reset or crash releases PWR_HOLD and R42 (4.7k) pulls it low, so the device turns off",
        "  unless USB is connected. R42 also covers RP2350-E9, which can hold a released pad near 2 V.",
        "- Enable the watchdog before any blocking init: a hang with PWR_HOLD high otherwise keeps the",
        "  device on with no way to turn it off but unplugging the cell.",
        "",
        "## Pins at reset",
        "",
        "Every GPIO resets with its pull-down on (RP2350 datasheet, Table 1674). Before reading them:",
        "turn on the pull-ups of the Selects, and turn off the pull-downs of",
        "PWR_BTN_N, P2_START, CHG_STAT_N and CHG_PG_N (their 100k external pull-ups otherwise sit at",
        "about 1-1.7 V against the pull-down). SR_PL resets low (loading) and the clock low: drive",
        "SR_PL high before clocking. ADC pins (VBAT_SENSE, CC1/CC2_SENSE): input buffer and",
        "pull-down both off. The board revision is on the silkscreen only (no strap pins since",
        "2026-10-24); firmware logs over USB or SWD (no UART connector).",
        "",
        "## Backlight (current sinks on BL_PWM)",
        "",
        f"GPIO{G['BL_PWM']} runs {pwm_slice(G['BL_PWM'])}, for example 12 bits at 36.6 kHz (150 MHz / 4096). R60/R61/C50 filter",
        "it into BL_REF (0.077 V at 100%, time constant 2.3 ms), and each sink (two cathode pins,",
        "two LEDs) holds its 2.2 ohm sense resistor at BL_REF: 35 mA a sink at 100% duty, 70 mA",
        "for the four LEDs (the panel's typical; its maximum is 80 mA). The LED",
        "current is DC, so the PWM frequency only sets the ripple (0.5% at 36.6 kHz). Current is",
        "proportional to duty; apply a gamma curve in firmware. Keep the lowest level at about 29%",
        "duty (5 mA per LED): below it the op-amp's offset (up to 4.5 mV against 22 mV) can make",
        "the two halves of the backlight visibly unequal. A brightness change settles in about",
        "10 ms (the filter); from off, the sinks start within about 1 ms and then follow BL_REF.",
        f"GPIO{G['BL_PWM']} resets low (pull-down), R61 holds BL_REF at 0 and the sinks run from 3V3, so the",
        "backlight is off until firmware starts the PWM and whenever 3V3 is off. Off = duty 0.",
        "Simulated with ngspice before layout: board/sim/backlight.py (results in",
        "board/sim/backlight_results.txt).",
        "",
        "## Charger (BQ24073) and USB power",
        "",
        "| CHG_PG_N | CHG_STAT_N | State |",
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
        "port is safe, but a heavy load leaves little for charging: read the source's offer and",
        "dim the backlight on a weak one. With the 5.1k Rd on each CC, the CC line the cable uses",
        "sits at 0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A;",
        "take the higher of CC1_SENSE and CC2_SENSE.",
        "",
        "## Audio (PWM into the FM8002A)",
        "",
        f"GPIO{G['AUDIO_PWM']} is {pwm_slice(G['AUDIO_PWM'])} (not the backlight's slice). Run it at 8 bits: at 150 MHz that's a 586 kHz carrier, which R32/C37 and",
        "R33/C38 (1k, 4.7 nF) take 49 dB down, with the audio band -3 dB at about 14 kHz. Not 10 bits:",
        "146 kHz would only come out 26 dB down (dither or noise-shape for more resolution). C39",
        "blocks the DC, and U7 drives the speaker bridge-tied at about 1.8x (R35/R34, with the filter's",
        "2k in front of R34). VSYS (the BQ24073's output) feeds it through JP3: about 0.5 W full scale",
        "on USB and 0.4-0.5 W on the cell, and it clips below a 3.7 V cell: back the amplitude off there.",
        "Idle at half scale so turning on doesn't pop. The FM8002A takes about 100 ms to wake (its",
        "bypass capacitor charges): keep AMP_EN high while the device is awake (about 4.4 mA), or raise",
        "it at least 150 ms before a sound, PWM idling. The speaker (15 x 10 mm, 8 ohm) lies on the",
        "board's top in the right bezel beside the screen and plugs into J4.",
        "",
        "## Battery voltage",
        "",
        "VBAT = 2 x ADC7 x 3.3 V / 4096 (12-bit), times a calibration gain (below). Full charge reads",
        "about 2.10 V at the pin.",
        "",
        "Shut down at 3.50 V as computed, filtered and under load, never lower. The ADC's reference",
        "is the 3V3 rail itself: once the RT9013 drops out (a cell under about 3.4 V) the rail falls",
        "with the cell and the computed VBAT levels off at 3.37-3.43 V whatever the cell really is,",
        "so a lower threshold never fires. 3.50 V also keeps 3V3 above the internal flash's 2.97 V",
        "minimum as the load steps.",
        "",
        "Calibrate the gain once: when CHG_STAT_N goes high at the end of a charge (CHG_PG_N low, TH1",
        "in range, CHG_OFF low) the cell is at the BQ24073's 4.2 V +-1%. Only a full charge counts:",
        "the charger doesn't terminate while the input limit or its thermal limit is cutting the",
        "charge current.",
        "",
        "Reading it: set the pad up once and keep its ADC channel for the whole program (in",
        "embassy-rp 0.8, dropping the `Channel` turns the pad's digital input and pull-down back on,",
        "which drains C8 and reads low; the digital input must stay off, RP2350-E9). Wait about",
        "200 ms after setting it up (the pad resets with its pull-down on; C8 recovers with a 23.5 ms",
        "time constant). Read bursts of up to 16 one-shot samples, at most once a second: free-running",
        "at 500 kS/s would read tens of percent low against this 235k source. Shut down only after",
        "several low bursts in a row. Pad leakage (1 uA maximum, nanoamps in practice) could offset",
        "the reading; bring-up step 5 compares ADC7 with a meter.",
    ]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    open(OUT, "w").write("\n".join(lines) + "\n")
    print("wrote", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
