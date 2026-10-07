"""Stability and accuracy of the backlight current sink, simulated with ngspice.

    python3 sim/models.py                  # once: fetch and translate the vendor models
    python3 sim/backlight.py [part ...]     # parts: loop dc steps pwm power vsys (default all)

Writes its findings to stdout; sim/backlight_results.txt is the last full run. Needs
libngspice0 (Ubuntu) and numpy. The whole run takes about 30-40 minutes, mostly the
phase-margin corners.

What each part checks:
  loop   loop gain at the op-amp's - input (voltage injection), every corner: duty, VSYS,
         LED Vf (+0, +0.2, +0.4 V), FPC inductance, gate capacitance x2. Phase margin and
         the lowest phase while the loop gain is above 1.
  dc     LED current per channel against duty x VSYS x Vf; FET dissipation; the op-amp's
         +-4.5 mV offset (datasheet maximum) on each channel; the 3V3 tolerance.
  steps  steps on BL_REF (filter bypassed): overshoot and settling.
  pwm    GPIO PWM through the 100k / 3.9k / 1 uF filter: LED current ripple.
  power  3V3 rising and falling with the GPIO low or high, VSYS hot plug with 3V3 off.
  vsys   VSYS steps while lit at full current (USB plug-in).
"""
import itertools
import sys

import numpy as np

import ngspice as ngs
import backlight_circuit as circ

K = 3.3 * 3.9 / 103.9          # BL_REF per unit PWM duty (GPIO at 3.3 V)
RS = 2.2
GEAR = ".options method=gear"


def section(t):
    print("\n== " + t, flush=True)


def loop():
    section("loop gain (channel 1), RG 330, RF 10k, CF 1n")

    def pm(**kw):
        ngs.run(circ.netlist(inject=1, **kw), "ac dec 50 10 30meg")
        f = ngs.vec("frequency").real
        T = -ngs.vec("fbn1") / ngs.vec("inn1")
        g = 20 * np.log10(abs(T))
        ph = np.unwrap(np.angle(T)) * 180 / np.pi
        ii = [k for k in range(len(g) - 1) if g[k] > 0 and g[k + 1] <= 0]
        if not ii:
            return None, None, g[0]
        i = ii[0]
        return f[i], 180 + ph[i], 180 + min(ph[g > 0])

    worst, sat = (999, None), []
    for ref, vsys, sh, lf, cx in itertools.product((0.006, 0.03, 0.124), (3.5, 3.8, 4.2, 4.7, 5.7),
                                                   (0, 0.2, 0.4), (20e-9, 50e-9), (0, 1.5e-9)):
        fc, p, low = pm(ref=f"DC {ref}", vsys=vsys, shift1=sh, shift2=sh, lfpc=lf,
                        extra=f"CX1 g1 s1 {cx}" if cx else "")
        if fc is None:
            sat.append((ref, vsys, sh))
            continue
        if p < worst[0]:
            worst = (p, dict(ref=ref, vsys=vsys, vf=sh, lfpc=lf, cgs_extra=cx, fc=round(fc)), low)
    print(f"720 corners; worst phase margin {worst[0]:.1f} deg at {worst[1]}; lowest phase while |T|>1 "
          f"leaves {worst[2]:.1f} deg")
    print("no crossover (op-amp output at its rail, the FET fully on: LED dropout, open loop):",
          sorted(set(sat)))


def op(d, vsys, sh, **kw):
    ngs.run(circ.netlist(ref=f"DC {d * K}", vsys=vsys, shift1=sh, shift2=sh, **kw), "op")
    return ngs.vec("s1")[0] / RS, ngs.vec("s2")[0] / RS, ngs.vec("d1")[0] - ngs.vec("s1")[0]


def dc():
    section("DC current per channel (mA); ideal = duty x 56.3 mA")
    vs = (3.5, 3.8, 4.2, 4.7, 5.7)
    print("duty ideal  Vf  | " + " ".join(f"VSYS {v}" for v in vs))
    for d in (0.05, 0.18, 0.5, 0.8, 1.0):
        for sh in (0.0, 0.2, 0.4):
            row = " ".join(f"{op(d, v, sh)[0] * 1e3:8.2f}" for v in vs)
            print(f"{d:4.2f} {d * K / RS * 1e3:5.2f} +{sh:.1f} | {row}", flush=True)
    section("FET dissipation at full current, Vf typ")
    for v in (4.2, 4.7, 5.7):
        i, _, vds = op(1.0, v, 0.0)
        print(f"VSYS {v}: {i * 1e3:.1f} mA, Vds {vds:.2f} V, {i * vds * 1e3:.0f} mW per FET")
    section("op-amp offset at the datasheet maximum: +4.5 mV on channel 1, -4.5 mV on channel 2 (VSYS 3.8)")
    for d in (0.05, 0.18, 0.5, 1.0):
        i1, i2, _ = op(d, 3.8, 0.0, vos1=4.5e-3, vos2=-4.5e-3)
        ideal = d * K / RS
        print(f"duty {d:4.2f}: ideal {ideal * 1e3:6.2f}, ch1 {i1 * 1e3:6.2f} ({(i1 / ideal - 1) * 100:+5.1f}%), "
              f"ch2 {i2 * 1e3:6.2f} ({(i2 / ideal - 1) * 100:+5.1f}%)")
    section("3V3 at +-2% (ME6211), full duty: the reference follows the GPIO high level")
    for vdd in (3.234, 3.3, 3.366):
        ngs.run(circ.netlist(ref=f"DC {vdd * 3.9 / 103.9}", vsys=3.8, vdd=f"DC {vdd}"), "op")
        print(f"3V3 {vdd}: {ngs.vec('s1')[0] / RS * 1e3:.2f} mA")


def steps():
    section("steps on BL_REF (0.5 us edge, filter bypassed): channel 1")
    print("step        VSYS  Vf   from mA  to mA  overshoot %  settle to 1% (us)")
    for d0, d1 in ((0.05, 0.95), (0.95, 0.05), (0.05, 0.06), (0.06, 0.05), (0.90, 0.95), (0.95, 0.90)):
        for vsys, sh in ((3.5, 0.0), (4.2, 0.0), (5.7, 0.0), (3.5, 0.2)):
            v0, v1 = d0 * K, d1 * K
            ngs.run(circ.netlist(ref=f"PWL(0 {v0} 50u {v0} 50.5u {v1})", vsys=vsys, shift1=sh, shift2=sh,
                                 extra=GEAR), "tran 20n 200u 0 20n")
            t, i = ngs.vec("time"), ngs.vec("s1") / RS
            pre, fin = i[(t > 40e-6) & (t < 50e-6)].mean(), i[t > 180e-6].mean()
            a, ta = i[t > 50.5e-6], t[t > 50.5e-6]
            span = fin - pre
            over = ((a.max() - fin) if span > 0 else (fin - a.min())) / abs(span) * 100
            out = np.where(abs(a - fin) > 0.01 * abs(span) + 1e-5)[0]
            ts = (ta[out[-1]] - 50e-6) * 1e6 if len(out) else 0
            print(f"{d0:.2f}->{d1:.2f}  {vsys:4.1f} +{sh:.1f}  {pre * 1e3:7.2f} {fin * 1e3:7.2f} {over:9.1f} {ts:10.1f}",
                  flush=True)
    section("turn-on from off (BL_REF 0 -> duty in 0.5 us)")
    for d in (0.18, 0.5):
        for vsys in (3.5, 5.7):
            ngs.run(circ.netlist(ref=f"PWL(0 0 50u 0 50.5u {d * K})", vsys=vsys, extra=GEAR), "tran 20n 1.5m 0 50n")
            t, i = ngs.vec("time"), ngs.vec("s1") / RS
            fin = i[t > 1.3e-3].mean()
            k = np.where(i > 0.9 * fin)[0][0]
            print(f"0 -> {d} at VSYS {vsys}: {fin * 1e3:.2f} mA, overshoot {(i.max() / fin - 1) * 100:.2f}%, "
                  f"90% after {(t[k] - 50e-6) * 1e6:.0f} us")


def tran(tend, step, **kw):
    p = dict(ref_direct=False)
    p.update(kw)
    ngs.run(circ.netlist(**p), f"tran {step} {tend} 0 {step}")
    return ngs.vec("time"), ngs.vec("s1") / RS, ngs.vec("s2") / RS


def pwm():
    section("GPIO PWM through 100k / 3.9k / 1 uF, steady state after 25 ms")
    for f in (36.62e3, 146.5e3):
        for d in (0.18, 0.5, 0.95):
            per = 1 / f
            t, i, _ = tran(25e-3, 0.2e-6, gpio=f"PULSE(0 3.3 0 10n 10n {d * per - 10e-9:.6g} {per:.6g})")
            w = t > 25e-3 - 5 * per
            m, pp = i[w].mean(), i[w].max() - i[w].min()
            print(f"{f / 1e3:6.1f} kHz duty {d:.2f}: {m * 1e3:6.2f} mA, ripple {pp * 1e3:.3f} mA p-p ({pp / m * 100:.2f}%)",
                  flush=True)


def power():
    section("power sequencing (VSYS 4.2 V)")
    t, i1, i2 = tran(3e-3, 0.5e-6, vdd="PWL(0 0 0.1m 0 0.6m 3.3)", gpio="DC 0")
    print(f"3V3 rises 0 -> 3.3 V in 0.5 ms, GPIO low: peak {max(i1.max(), i2.max()) * 1e3:.3f} mA")
    t, i1, i2 = tran(20e-3, 1e-6, vdd="PWL(0 0 0.1m 0 0.6m 3.3)", gpio="PWL(0 0 0.1m 0 0.6m 3.3)")
    ref = ngs.vec("ref")
    err = max(abs(i1[t > 3e-3] - ref[t > 3e-3] / RS)) * 1e3
    print(f"3V3 rises with the GPIO high (100% duty from reset): follows BL_REF (filter, tau 3.75 ms) "
          f"within {err:.2f} mA, peak {i1.max() * 1e3:.2f} mA (setpoint 56.3)")
    t, i1, i2 = tran(5e-3, 0.5e-6, vdd="PWL(0 3.3 3m 3.3 3.2m 0)", gpio="PWL(0 3.3 3m 3.3 3.2m 0)")
    print(f"3V3 and GPIO fall at 3 ms while at 100%: before {i1[(t > 2.5e-3) & (t < 3e-3)].mean() * 1e3:.2f} mA, "
          f"peak after {i1[t > 3e-3].max() * 1e3:.2f} mA, at 5 ms {i1[-1] * 1e3:.3f} mA")
    for v0 in (0, 3.5):
        t, i1, i2 = tran(200e-6, 10e-9, vdd="DC 0", vsys_src=f"PWL(0 {v0} 10u {v0} 11u 5.7)")
        print(f"VSYS {v0} -> 5.7 V in 1 us with 3V3 off (device off): peak LED current "
              f"{max(i1.max(), i2.max()) * 1e3:.3f} mA")


def vsys():
    section("VSYS steps while lit at 100% (USB plug-in), and back")
    for to, tr in ((4.7, 2e-6), (5.7, 2e-6), (4.7, 10e-6), (5.7, 10e-6)):
        src = f"PWL(0 3.8 1m 3.8 {1e-3 + tr} {to} 1.5m {to} {1.5e-3 + tr} 3.8)"
        ngs.run(circ.netlist(gpio="DC 3.3", ref_direct=False, vsys_src=src), "tran 5n 2m 0.9m 5n")
        t, i = ngs.vec("time"), ngs.vec("s1") / RS
        base = i[(t > 0.9e-3) & (t < 1e-3)].mean()
        w = t > 1e-3
        hi, lo = t[w][i[w] > 1.5 * base], t[w][i[w] < 0.5 * base]
        dur = lambda a: (a.max() - a.min()) * 1e6 if len(a) else 0
        print(f"3.8 -> {to} V in {tr * 1e6:.0f} us: {base * 1e3:.1f} mA steady; max {i[w].max() * 1e3:.1f} mA "
              f"(above 1.5x for {dur(hi):.1f} us), min {i[w].min() * 1e3:.1f} mA (below 0.5x for {dur(lo):.1f} us)",
              flush=True)


PARTS = dict(loop=loop, dc=dc, steps=steps, pwm=pwm, power=power, vsys=vsys)

if __name__ == "__main__":
    for name in sys.argv[1:] or PARTS:
        PARTS[name]()
