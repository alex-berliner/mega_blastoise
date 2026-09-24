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
    "UART_TX": "UART0 TX to J6 pin 1 (debug console)",
    "UART_RX": "UART0 RX from J6 pin 3",
    "PWR_HOLD": "output: set high first thing at boot (the user is still holding the button); low turns the device off once the button is up",
    "PWR_BTN_N": "input, pulled up (R16): low while the power button is pressed",
    "I2S_DIN": "PIO I2S data to U7",
    "I2S_BCLK": "PIO I2S bit clock (side-set base)",
    "I2S_LRCLK": "PIO I2S word clock (side-set base + 1: embassy-rp's PioI2sOut wants BCLK, BCLK+1)",
    "AMP_SD": "output: high = amplifier on, left channel (SD_MODE ~2.2 V through R32); low = shutdown, 0.6 uA",
    "STATUS_LED": "output: high lights D4, the red error LED",
    "CHG_STAT_N": "input, pulled up (R10): low while the BQ24074 charges; also lights D1",
    "VIN_GOOD_N": "input, pulled up (R9): low while USB power is present",
    "CHG_EN1": "output: charger input limit, see below (pulled up by R8 until driven)",
    "CHG_EN2": "output: charger input limit, see below (pulled down by R7 until driven)",
    "LCD_DC": "display data/command",
    "LCD_RST": "display reset, active low",
    "BL_PWM": "PWM to the TPS61160 CTRL pin: backlight on/off and dimming, 5 to 100 kHz",
    "LCD_MISO": "SPI0 RX (panel readback, optional)",
    "LCD_CS": "SPI0 CSn (or drive as a GPIO)",
    "LCD_SCK_MCU": "SPI0 SCK, 33 ohm source termination R28",
    "LCD_MOSI_MCU": "SPI0 TX, 33 ohm source termination R29",
    "VBAT_SENSE": "ADC0: battery voltage / 2 (R17/R18, 470k each, 100 nF)",
    "CC1_SENSE": "ADC1: USB-C CC1 through 10k; the source's current offer",
    "CC2_SENSE": "ADC2: USB-C CC2 through 10k",
}
BUTTON_POS = {  # button net -> contact reference (SWn = case numbering)
    p["pins"]["1"]: p["ref"] for p in design.PARTS if p["sheet"] == "buttons.kicad_sch"
}


def pwm_slice(g):
    return f"PWM{(g >> 1) % 12}{'AB'[g & 1]}"


def main():
    lines = [
        "# RP2350B pin map (reference board)",
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
            use = f"membrane contact {BUTTON_POS[net]}, input with pull-up, low = pressed"
        else:
            use = ROLE.get(net, "")
        if net == "BL_PWM":
            use += f" ({pwm_slice(g)})"
        lines.append(f"| {g} | `{net}` | {use} |")
    lines += [
        "",
        "Other RP2350B pins: QSPI flash (W25Q128JVP, 16 MB), 12 MHz crystal, USB to the USB-C",
        "(27 ohm series, USBLC6 ESD), SWD and RUN to J5 and the RESET button, QSPI_SS to the BOOTSEL",
        "button through 1k.",
        "",
        "## Power control",
        "",
        "PWR_EN (the 3V3 regulator's enable) is the OR of: the power button, PWR_HOLD, and USB VBUS.",
        "",
        "- Turning on: the button starts the regulator; firmware must set PWR_HOLD high before the",
        "  user lets go (hold time is the boot time, well under a second).",
        "- Turning off: firmware sets PWR_HOLD low; power drops when the button is released.",
        "  Watch PWR_BTN_N for a long press, then drop PWR_HOLD.",
        "- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs",
        "  while charging: show a charging screen, turn the backlight off (BL_PWM low) and sleep.",
        "- A reset or crash releases PWR_HOLD (GPIOs float, pulled low), so the device turns off",
        "  unless USB is connected.",
        "",
        "## Charger input limit (BQ24074 EN2, EN1)",
        "",
        "| CHG_EN2 | CHG_EN1 | USB input limit |",
        "|---|---|---|",
        "| 0 | 1 | 500 mA (state at power-up: R7 pulls EN2 low, R8 pulls EN1 high) |",
        "| 1 | 0 | 1.34 A (ILIM resistor R5): only on a Type-C source offering 1.5 A or 3 A |",
        "| 0 | 0 | 100 mA |",
        "| 1 | 1 | suspend (no input current) |",
        "",
        "Reading the source's offer: with the 5.1k Rd on each CC, the CC line the cable uses sits at",
        "0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A. Take the",
        "higher of CC1_SENSE and CC2_SENSE; above about 0.66 V it is safe to select 1.34 A.",
        "Fast-charge current is fixed at 740 mA by R6 either way.",
        "",
        "## Battery voltage",
        "",
        "VBAT = 2 x ADC0 x 3.3 V / 4096 (12-bit). Read a few samples; the 100 nF holds the divider",
        "steady for the ADC's sampling capacitor. Full charge reads about 2.10 V at the pin.",
    ]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    open(OUT, "w").write("\n".join(lines) + "\n")
    print("wrote", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
