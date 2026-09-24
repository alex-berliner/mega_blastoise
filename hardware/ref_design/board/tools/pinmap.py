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
    "UART_TX": "UART1 TX to J6 pin 1 (debug console)",
    "UART_RX": "UART1 RX from J6 pin 3",
    "PWR_HOLD": "output: set high to stay on (see Power control); low turns the device off once the button is up",
    "PWR_BTN_N": "input, pulled up (R16): low while seat 1's Y is pressed. Y is the power button, so read seat 1's Y here",
    "AUDIO_PWM": "PWM4 A: audio, through two 16 kHz RC poles into U7 (see Audio)",
    "AMP_EN": "output: high turns the FM8002A on (Q3 pulls its SHUTDOWN low); low or floating = off (4 uA)",
    "CHG_STAT_N": "input, pulled up (R10): low while the TP4056 charges",
    "CHG_DONE_N": "input, pulled up (R9): low when the charge is done (USB present, cell full)",
    "LCD_DC": "display data/command",
    "LCD_RST": "display reset, active low",
    "LCD_TE": "input: the panel's tearing-effect signal (enable with TEON); start a frame on its edge",
    "BL_SDI": "output to the AP3156 SDI (1-wire): backlight current in 32 steps, see below; low > 256 us = off",
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
            use = f"membrane contact {BUTTON_POS[net]}, input with pull-up, low = pressed"
        else:
            use = ROLE.get(net, "")
        lines.append(f"| {g} | `{net}` | {use} |")
    lines += [
        "",
        "Other RP2354B pins: the 2 MB flash is inside the package, so of the QSPI pins only QSPI_SS",
        "is wired (10k pull-up R23, and the BOOTSEL button through 1k). 12 MHz crystal, USB to the",
        "USB-C (22 ohm series, SRV05-4 ESD), SWD and RUN to J5 and the RESET button.",
        "",
        "## Power control",
        "",
        "PWR_EN (the ME6211's enable) is the OR of: the power button (seat 1's Y contact, which",
        "joins VSYS to PWR_BTN_RAW), PWR_HOLD, and USB VBUS.",
        "",
        "- Turning on: pressing Y starts the regulator. Firmware sets PWR_HOLD high only once Y has",
        "  been held for about a second (read on PWR_BTN_N), so a knock in a bag doesn't turn the",
        "  device on: let go sooner and it powers off again.",
        "- Turning off: firmware sets PWR_HOLD low; power drops when the button is released.",
        "  Watch PWR_BTN_N for a long press, then drop PWR_HOLD.",
        "- With USB plugged in the 3V3 rail stays up whatever PWR_HOLD does, so the firmware runs",
        "  while charging: show a charging screen, turn the backlight off (BL_SDI low) and sleep.",
        "- A reset or crash releases PWR_HOLD (GPIOs float, pulled low), so the device turns off",
        "  unless USB is connected.",
        "",
        "## Backlight (AP3156 on BL_SDI)",
        "",
        "SDI idles high. A command is a burst of low pulses (each low 0.3 to 50 us, high 0.05 to",
        "50 us), ended by holding SDI high for more than 256 us; holding it low for more than 256 us",
        "shuts the driver down, and the next rising edge wakes it. Brightness takes two bursts:",
        "10 pulses (all six channels), then N pulses for current level N: 1 = 20 mA per LED (the",
        "panel's maximum), 6 = 15 mA (its typical rating), down to 31 = 3.6 mA; 32 = off; about",
        "0.5 dB a step. The driver starts at 0 mA and R31 holds SDI low, so the backlight stays off",
        "until firmware sets a level. Command table: AP3156 datasheet, Serial Digital Interface.",
        "",
        "## Charger (TP4056) and USB power",
        "",
        "| CHG_STAT_N | CHG_DONE_N | State |",
        "|---|---|---|",
        "| low | high | charging (500 mA, set by R5) |",
        "| high | low | charge done |",
        "| high | high | no USB power |",
        "",
        "With USB in, the system runs from USB through D5 and the cell only charges; unplugged,",
        "Q2 connects the cell. The charger has no input current limit: a default-USB port (500 mA)",
        "can be asked for charge current plus the system's draw, so read the source's offer and",
        "dim the backlight on a weak one. With the 5.1k Rd on each CC, the CC line the cable uses",
        "sits at 0.25-0.61 V for default USB power, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A;",
        "take the higher of CC1_SENSE and CC2_SENSE.",
        "",
        "## Audio (PWM into the FM8002A)",
        "",
        "GPIO8 is PWM4 A. At 150 MHz, 10 bits gives a 146 kHz carrier (8 bits: 586 kHz); R32/C37 and",
        "R33/C38 (two poles at 16 kHz) take it out, C39 blocks the DC, and U7 drives the speaker",
        "bridge-tied at a gain of 2 (R35/R34). Idle at half scale so turning on doesn't pop, and set",
        "AMP_EN high only while sound plays: the amplifier draws about 4.4 mA when on.",
        "",
        "## Battery voltage",
        "",
        "VBAT = 2 x ADC0 x 3.3 V / 4096 (12-bit). Read a few samples; the 100 nF holds the divider",
        "steady for the ADC's sampling capacitor. Full charge reads about 2.10 V at the pin. Shut down",
        "at about 3.35 V: below that the ME6211 can't hold 3.3 V, and the RP2354B's regulator needs",
        "VREG_AVDD at 3.135 V or more.",
    ]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    open(OUT, "w").write("\n".join(lines) + "\n")
    print("wrote", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
