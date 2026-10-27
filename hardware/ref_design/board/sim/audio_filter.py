"""The audio PWM's two-stage RC filter (R32/C37, R33/C38, then C39 and R34 into the amplifier): AC response, from the circuit
as design.py has it. A small nodal solver (numpy), no SPICE needed: five nodes' worth of R and C.

    python3 sim/audio_filter.py        # prints the table and writes sim/audio_filter_results.txt

The PWM carrier: LEDC on the ESP32-S3 runs from the 80 MHz APB clock, so 8 bits is 312.5 kHz (7 bits 625 kHz). The source is
the module's pin driving about 40 ohm. The load is C39 (220 nF) and R34 (20 k): C39 is a short at these frequencies.
"""
import os

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
RS, RL = 40.0, 20e3


def gain(f, r1, c1, r2, c2):
    s = 2j * np.pi * f
    a = np.array([[1 / (RS + r1) + s * c1 + 1 / r2, -1 / r2], [-1 / r2, 1 / r2 + s * c2 + 1 / RL]], dtype=complex)
    v = np.linalg.solve(a, np.array([1 / (RS + r1), 0], dtype=complex))
    return abs(v[1])


def db(x):
    return 20 * np.log10(x)


def f3db(r1, c1, r2, c2):
    for f in np.logspace(2, 5, 6000):
        if db(gain(f, r1, c1, r2, c2)) < -3:
            return f


rows = []
for name, r, c in (("old 1k / 4.7n (RP2354B, 586 kHz)", 1e3, 4.7e-9), ("1.2k / 4.7n", 1.2e3, 4.7e-9),
                   ("this board 1.5k / 4.7n", 1.5e3, 4.7e-9), ("1k / 10n", 1e3, 10e-9)):
    rows.append(f"{name:36s} -3 dB at {f3db(r, c, r, c) / 1e3:5.1f} kHz; at 20 kHz {db(gain(20e3, r, c, r, c)):6.1f} dB; "
                f"carrier 312.5 kHz {db(gain(312.5e3, r, c, r, c)):6.1f} dB, 586 kHz {db(gain(586e3, r, c, r, c)):6.1f} dB, "
                f"625 kHz {db(gain(625e3, r, c, r, c)):6.1f} dB")
print("\n".join(rows))
open(os.path.join(HERE, "audio_filter_results.txt"), "w").write("\n".join(rows) + "\n")
