"""Project symbol library (lib/battler.kicad_sym): parts KiCad's libraries don't have.

    python3 tools/sym_lib.py

Konnect's create_symbol writes pins only (no body, pin names and numbers hidden),
which reads badly on a schematic, so this writes the symbols and has kicad-cli
re-save the file through KiCad's own reader and writer. Pin numbers and names
come from the datasheets; the body is a plain box.
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.kicad_sym")

# name: (reference prefix, value, footprint, datasheet, description, body half width, half height, pins)
# pin: (number, name, electrical type, side, position along the side in 2.54 mm steps)
SYMBOLS = {
    "MCP6002": (
        "U", "MCP6002", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm",
        "https://ww1.microchip.com/downloads/en/DeviceDoc/MCP6001-1R-1U-2-4-1-MHz-Low-Power-Op-Amp-DS20001733L.pdf",
        "Dual 1 MHz rail-to-rail input/output op-amp, 1.8-6 V; both halves in one unit",
        7.62, 7.62,
        [("3", "+INA", "input", "L", 3), ("2", "-INA", "input", "L", 2),
         ("5", "+INB", "input", "L", -2), ("6", "-INB", "input", "L", -3),
         ("1", "OUTA", "output", "R", 2), ("7", "OUTB", "output", "R", -2),
         ("8", "VDD", "power_in", "T", 0), ("4", "VSS", "power_in", "B", 0)]),
    "FM8002A": (
        "U", "FM8002A", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm",
        "https://www.lcsc.com/product-detail/Audio-Power-OpAmps_Shenzhen-Fuman-Elec_C94625.html",
        "2 W mono bridge-tied audio amplifier, 2.5-5.5 V, gain 2 x Rf/Ri, SHUTDOWN high = off",
        10.16, 7.62,       # wide enough that SHUTDOWN's name clears GND's, which sits right of centre
        [("3", "+IN", "input", "L", 2), ("4", "-IN", "input", "L", 1), ("2", "BYPASS", "passive", "L", -1),
         ("1", "SHUTDOWN", "input", "L", -2), ("5", "VO1", "output", "R", 1), ("8", "VO2", "output", "R", -1),
         ("6", "VDD", "power_in", "T", 1), ("7", "GND", "power_in", "B", 1)]),
}
PIN_LEN = 2.54


def q(s):
    return '"' + s.replace('"', '\\"') + '"'


def prop(name, value, x, y, hide=False, justify=None):
    j = f" (justify {justify})" if justify else ""
    h = " (hide yes)" if hide else ""
    return (f"\t\t(property {q(name)} {q(value)}\n\t\t\t(at {x:g} {y:g} 0)\n"
            f"\t\t\t(effects (font (size 1.27 1.27)){j}{h})\n\t\t)\n")


def symbol(name, spec):
    ref, value, fp, ds, descr, hw, hh, pins = spec
    out = f"\t(symbol {q(name)}\n\t\t(pin_names (offset 1.016))\n\t\t(exclude_from_sim no)\n\t\t(in_bom yes)\n\t\t(on_board yes)\n"
    out += prop("Reference", ref, -hw, hh + PIN_LEN + 1.91, justify="left")
    out += prop("Value", value, hw, hh + PIN_LEN + 1.91, justify="right")
    out += prop("Footprint", fp, 0, -hh - 5.08, hide=True)
    out += prop("Datasheet", ds, 0, -hh - 7.62, hide=True)
    out += prop("Description", descr, 0, -hh - 10.16, hide=True)
    out += f"\t\t(symbol {q(name + '_0_1')}\n"
    out += (f"\t\t\t(rectangle (start {-hw:g} {hh:g}) (end {hw:g} {-hh:g})\n"
            f"\t\t\t\t(stroke (width 0.254) (type default))\n\t\t\t\t(fill (type background))\n\t\t\t)\n")
    out += "\t\t)\n"
    out += f"\t\t(symbol {q(name + '_1_1')}\n"
    for num, pname, etype, side, k in pins:
        if side == "L":
            x, y, a = -hw - PIN_LEN, k * 2.54, 0
        elif side == "R":
            x, y, a = hw + PIN_LEN, k * 2.54, 180
        elif side == "T":
            x, y, a = k * 2.54, hh + PIN_LEN, 270
        else:
            x, y, a = k * 2.54, -hh - PIN_LEN, 90
        out += (f"\t\t\t(pin {etype} line\n\t\t\t\t(at {x:g} {y:g} {a})\n\t\t\t\t(length {PIN_LEN:g})\n"
                f"\t\t\t\t(name {q(pname)} (effects (font (size 1.27 1.27))))\n"
                f"\t\t\t\t(number {q(num)} (effects (font (size 1.27 1.27))))\n\t\t\t)\n")
    out += "\t\t)\n\t\t(embedded_fonts no)\n\t)\n"
    return out


def main():
    text = '(kicad_symbol_lib\n\t(version 20241209)\n\t(generator "battler_sym_lib")\n\t(generator_version "10.0")\n'
    for name, spec in SYMBOLS.items():
        text += symbol(name, spec)
    text += ")\n"
    with tempfile.TemporaryDirectory() as tmp:
        raw = os.path.join(tmp, "battler.kicad_sym")
        open(raw, "w").write(text)
        # KiCad reads it and writes it back in its own form: a malformed symbol fails here
        saved = os.path.join(tmp, "saved.kicad_sym")
        r = subprocess.run(["kicad-cli", "sym", "upgrade", "--force", "-o", saved, raw], capture_output=True, text=True)
        if r.returncode or not os.path.exists(saved):
            sys.exit(f"kicad-cli rejected the library:\n{r.stdout}{r.stderr}")
        open(LIB, "w").write(open(saved).read())
    print("wrote", LIB, "with", ", ".join(SYMBOLS))


if __name__ == "__main__":
    main()
