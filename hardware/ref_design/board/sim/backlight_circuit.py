"""The backlight current sink (Display sheet), as an ngspice netlist.

Per channel (LED_K1 / LED_K2): an MCP6002 half (U6) drives an AO3400A (Q5 / Q6) through
RG (330 ohm); the FET's source sits on a 2.2 ohm sense resistor; RF (10k) brings the sense
voltage back to the - input and CF (1 nF) from the output to the - input closes a local
loop so the gate capacitance can't ring the loop. RPD (100k) holds the gate low while the
op-amp is unpowered. BL_REF is GPIO44's PWM through 100k into 3.9k || 1 uF.
The panel: LED_A on VSYS through the FPC, three LEDs per cathode pin, FPC inductance each way.
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OPAMP = open(os.path.join(HERE, "models", "mcp6001.lib")).read()
FET = open(os.path.join(HERE, "models", "ao3400.lib")).read()
NVT = 0.1018            # N * Vt of the LED fit


def led_model(name, shift):
    """White LED: 2.60 V at 1 mA, 3.00 V at 20 mA (panel datasheet typ), RS 5 ohm, 30 pF.
    shift moves the whole curve up (Vf corner: +0.2 V = 3.2 V max, +0.4 V = 3.4 V old max)."""
    IS = 8.5e-15 * math.exp(-shift / NVT)
    return f".model {name} D (IS={IS:.4g} N=3.94 RS=5 CJO=30p BV=5 IBV=10u)"


def netlist(vsys=4.2, ref="DC 0.1", vdd="DC 3.3", shift1=0.0, shift2=0.0, rg=330, rf=10e3, cf=1e-9,
            rs=2.2, lfpc=40e-9, vos1=0.0, vos2=0.0, vsys_src=None, extra="", ref_direct=True,
            gpio="DC 0", rtop=100e3, rbot=3.9e3, cref=1e-6, gate_pd=100e3, inject=None, rgpio=50):
    s = ["* backlight sink", OPAMP, FET, led_model("LEDA", shift1), led_model("LEDB", shift2)]
    s.append(f"VSYS vsys 0 {vsys_src or 'DC %g' % vsys}")
    s.append("RVS vsys vsysb 0.05")                     # source and track resistance
    s.append(f"VDD vdd 0 {vdd}")
    s.append("CVDD vdd 0 100n")
    s.append(f"LA vsysb anode {lfpc}")                  # LED_A through the FPC
    for g, mdl in ((1, "LEDA"), (2, "LEDB")):
        for k in range(3):
            s.append(f"D{g}{k} anode k{g}p {mdl}")
        s.append(f"LK{g} k{g}p d{g} {lfpc}")            # cathode back through the FPC
    if ref_direct:
        s.append(f"VREF ref 0 {ref}")
    else:
        s.append(f"VG gpio0 0 {gpio}")
        s.append(f"RGP gpio0 gpio {rgpio:g}")           # GPIO output resistance
        s.append(f"RTOP gpio ref {rtop:g}")
        s.append(f"RBOT ref 0 {rbot:g}")
        s.append(f"CREF ref 0 {cref:g}")
    for g, vos in ((1, vos1), (2, vos2)):
        s.append(f"VOS{g} inp{g} ref DC {vos:g}")       # offset, referred to the + input
        if inject == g:
            s.append(f"VINJ{g} inn{g} fbn{g} DC 0 AC 1")  # loop broken at the high-impedance input
        else:
            s.append(f"VINJ{g} inn{g} fbn{g} DC 0")
        s.append(f"XA{g} inp{g} inn{g} vdd 0 out{g} MCP6001")
        s.append(f"RG{g} out{g} g{g} {rg:g}")
        s.append(f"RPD{g} g{g} 0 {gate_pd:g}")
        s.append(f"XQ{g} d{g} g{g} s{g} AO3400")
        s.append(f"RS{g} s{g} 0 {rs:g}")
        s.append(f"RF{g} s{g} fbn{g} {rf:g}")
        s.append(f"CF{g} out{g} fbn{g} {cf:g}")
    s.append(extra)
    return "\n".join(s)
