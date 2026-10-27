"""Hand layouts of the reference schematic's sheets, block by block.

Each function returns a schdraw.Sheet: every part of the sheet placed (origin,
rotation, where its Reference/Value go), wired to its neighbours, and labelled
only where a net leaves its block. Coordinates are mm on the A4 page (x right,
y down), on KiCad's 1.27 mm grid. The parts, values and nets all come from
design.py; this file only says where things go and which pins a wire joins.
"""
from schdraw import Sheet, G, tw

H = 1.27


def buttons():
    sh = Sheet("buttons.kicad_sch")

    def column(refs, x0, y0, pitch=15.24):
        """Contacts one under another, pins to the right labelled, a GND bus on the left;
        a contact on VSYS (the power button) gets the rail instead of the bus."""
        bus_x = x0 - 12.7
        gnd_rows = []
        for i, ref in enumerate(refs):
            y = y0 + i * pitch
            sh.place(ref, x0, y, 180, f="c")
            sh.tag(f"{ref}.1", 5.08)
            if sh.parts[ref]["pins"]["2"] == "GND":
                sh.wire(f"{ref}.2", (bus_x, y))
                gnd_rows.append(y)
            else:
                sh.rail("VSYS", sh.stub(f"{ref}.2", 5.08), "left")
        if len(gnd_rows) > 1:
            sh.wire((bus_x, gnd_rows[0]), (bus_x, gnd_rows[-1]))
        if gnd_rows:
            sh.gnd((bus_x, gnd_rows[-1]))

    for col, first in enumerate((1, 9)):
        x0 = 35.56 + col * 124.46
        sh.heading(x0 - 17.78, 22.86, f"Seat {col + 1}: D-pad and face buttons")
        column([f"SW{first + i}" for i in range(8)], x0, 35.56)
    # Start and Select: the smaller contacts; seat 1's Start is also the power button
    sh.heading(86.36, 22.86, "Seat 1 Start/Select")
    column(["SW20"], 104.14, 35.56)
    column(["SW21"], 104.14, 50.8)
    sh.text(86.36, 63.5, "P1 START is the power button: it joins VSYS")
    sh.text(86.36, 67.31, "to PWR_BTN_RAW (Power sheet); the MCU reads")
    sh.text(86.36, 71.12, "it through Q1 as PWR_BTN_N.")
    sh.heading(231.14, 22.86, "Seat 2 Start/Select")
    column(["SW22", "SW23"], 248.92, 35.56)
    # the two shift registers and their pull-up arrays
    for col, (u, c, rns) in enumerate((("U8", "C52", ("RN1", "RN2")), ("U9", "C53", ("RN3", "RN4")))):
        x0 = 63.5 + col * 124.46
        sh.place(u, x0 + 20.32, 76.2, 0, f=(0, -15.24))
        for n in ("1", "2", "15", "8", "16", "10", "9", "11", "12", "13", "14", "3", "4", "5", "6"):
            sh.tag(f"{u}.{n}", 5.08)
        sh.nc(f"{u}.7")
        sh.place(c, x0 + 40.64, 76.2, 0, f=("r", G))
        sh.tag(f"{c}.1", G); sh.tag(f"{c}.2", G)
        for k, rn in enumerate(rns):
            sh.place(rn, x0 + 12.7 + k * 20.32, 129.54, 0, f=(0, -10.16))
            for n in "12345678":
                sh.tag(f"{rn}.{n}", 5.08)
    # the clock's series resistor (on the board by the RP2354B's pin)
    sh.heading(116.84, 101.6, "SR_CLK source termination")
    sh.place("R65", 129.54, 111.76, 90)
    sh.tag("R65.1", 5.08)
    sh.tag("R65.2", 5.08)
    sh.text(17.78, 165.1, "D-pad and face buttons: bare ENIG combs under DS Lite silicone, read through a 74HC165 per seat (chained: U9's")
    sh.text(17.78, 168.91, "Q7 into U8's DS; SR_PL low loads, 16 SR_CLK rises shift P1 Y..UP then P2 Y..UP out of SR_DATA, low = pressed).")
    sh.text(17.78, 172.72, "Start/Select: 4 mm metal domes (HYP 600-415S-260), their own GPIOs; both Starts sit on VSYS (Power sheet).")
    return sh


def audio():
    sh = Sheet("audio.kicad_sch")
    Ux, Uy = 124.46, 76.2                       # FM8002A
    Yc = Uy - 2.54                              # signal row: PWM in to -IN
    sh.place("U7", Ux, Uy, 0, f=(12.7, 10.16))
    # -- PWM low-pass, DC block, input resistor: left to right along the signal row
    sh.heading(17.78, Yc - 17.78, "PWM low-pass filter (two RC stages)")
    sh.place("R32", 36.83, Yc, 90)
    sh.place("C37", 44.45, Yc + 3.81, 0, f=("r", G))
    sh.place("R33", 55.88, Yc, 90)
    sh.place("C38", 67.31, Yc + 3.81, 0, f=("r", G))
    sh.place("C39", 78.74, Yc, 90)
    sh.place("R34", 93.98, Yc, 90, f=("at", 93.98, Yc - 3.81))
    a = (27.94, Yc)
    sh.wire(a, "R32.1")
    sh.label("AUDIO_PWM", a, "left")
    sh.wire("R32.2", "R33.1"); sh.wire((44.45, Yc), "C37.1"); sh.tag("C37.2", 0)
    sh.wire("R33.2", "C39.1"); sh.wire((67.31, Yc), "C38.1"); sh.tag("C38.2", 0)
    sh.wire("C39.2", "R34.1")
    sh.heading(73.66, Yc - 17.78, "DC block, Ri")
    node = (99.06, Yc)
    sh.wire("R34.2", "U7.4")
    # -- feedback Rf over the amplifier, from VO1 back to -IN
    Yf = Uy - 33.02
    right = Ux + 27.94
    sh.place("R35", (node[0] + right) / 2, Yf, 270)
    sh.wire(node, (node[0], Yf), "R35.2")
    sh.wire("R35.1", (right, Yf), (right, Yc))
    sh.text(right + 2.54, Yf, "Rf: gain 2 x R35 / R34 = 2")
    # -- supply, inside the feedback loop
    Yv = Uy - 20.32
    sh.heading(Ux - 17.78, Yf - 8.89, "Amplifier (FM8002A) and its supply")
    sh.place("JP3", Ux - 10.16, Yv, 0)
    sh.rail("VSYS", "JP3.1")
    sh.place("C34", Ux + 7.62, Yv + 3.81, 0, f=("r", G))
    sh.place("C35", Ux + 17.78, Yv + 3.81, 0, f=("r", G))
    sh.wire("JP3.2", (Ux + 17.78, Yv))
    sh.wire((Ux + 2.54, Yv), "U7.6")
    sh.wire((Ux + 7.62, Yv), "C34.1"); sh.tag("C34.2", 0)
    sh.wire((Ux + 17.78, Yv), "C35.1"); sh.tag("C35.2", 0)
    sh.flag("#FLG09", (Ux + 12.7, Yv))
    sh.label("AMP_VDD", sh.wire((Ux - 2.54, Yv), (Ux - 2.54, Yv - G)), "up")
    sh.tag("U7.7", G)
    # -- bypass: +IN and BYPASS both on AMP_BYP, C40 to ground
    sh.tag("U7.3")
    sh.place("C40", Ux - 20.32, Uy + 2.54 + 3.81, 0, f=("l", G))
    by = (Ux - 30.48, Uy + 2.54)
    sh.wire("U7.2", by)
    sh.label("AMP_BYP", by, "left")
    sh.wire((Ux - 20.32, Uy + 2.54), "C40.1"); sh.tag("C40.2", 0)
    # -- shutdown: R36 holds SHUTDOWN high (off), Q3 pulls it low when AMP_EN is high
    Ysd = Uy + 17.78
    sh.heading(Ux - 66.04, Ysd + 25.4, "Shutdown control")
    sh.wire("U7.1", (Ux - 15.24, Uy + 5.08), (Ux - 15.24, Ysd))
    sh.place("R36", Ux - 35.56, Ysd - 3.81, 180, f=("l", -G))
    sh.place("Q3", Ux - 25.4 - 2.54, Ysd + 7.62, 0, f="r")
    sl = (Ux - 48.26, Ysd)
    sh.wire((Ux - 15.24, Ysd), sl)
    sh.wire((Ux - 35.56, Ysd), "R36.1")
    sh.tag("R36.2")
    sh.wire((Ux - 25.4, Ysd), "Q3.3")
    sh.tag("Q3.2", 0)
    tp(sh, "TP8", sl, "l")
    # net names where no wire runs through their text
    sh.names = {"AMP_SHDN": (sl[0] + 1.27, Ysd, 0), "SPK_P": (Ux + 15.24, Yc, 0)}
    gl = (Ux - 63.5, Ysd + 7.62)
    sh.wire("Q3.1", gl)
    sh.label("AMP_EN", gl, "left")
    sh.place("R37", Ux - 43.18, Ysd + 7.62 + 3.81, 0, f=("r", G))
    sh.wire((Ux - 43.18, Ysd + 7.62), "R37.1"); sh.tag("R37.2", 0)
    # -- speaker
    Jx = Ux + 45.72
    sh.heading(Jx - 12.7, Yc - 10.16, "Speaker J4")
    sh.place("J4", Jx, Yc, 0, f=(7.62, 1.27))
    sh.nc("J4.MP")
    sh.wire("U7.5", "J4.1")
    sh.wire("U7.8", (Jx - 10.16, Uy + 2.54), (Jx - 10.16, Yc + 2.54), "J4.2", v=True)
    sh.text(17.78, 160.02, "AUDIO_PWM (GPIO31, PWM7 B, 8 bits: 586 kHz) through two RC stages (-3 dB ~14 kHz); gain 2 x R35/R34.")
    sh.text(17.78, 163.83, "AMP_EN high turns the amplifier on (Q3 pulls SHUTDOWN low). 8 ohm speaker on J4 (PicoBlade), lying on the board beside the screen.")
    return sh


def debug():
    sh = Sheet("debug.kicad_sch")

    def three_pin(ref, x, y, title):
        """A 3-pin JST-SH debug connector, pins to the right: signal, GND, signal."""
        sh.heading(x - 7.62, y - 15.24, title)
        sh.place(ref, x, y, 180, f=(-5.08, 0))
        sh.nc(f"{ref}.MP")
        sh.tag(f"{ref}.3", 7.62)
        sh.wire(f"{ref}.2", (x + 10.16, y))
        sh.gnd((x + 10.16, y))
        end = sh.wire(f"{ref}.1", (x + 7.62, y + 2.54), (x + 7.62, y + 7.62), (x + 17.78, y + 7.62))
        sh.label(sh.parts[ref]["pins"]["1"], end, "right")

    three_pin("J5", 27.94, 50.8, "SWD connector J5")
    # -- BOOTSEL and RESET buttons, J7 (the same SWD on bare pads, for a pogo jig) between
    # them with its BOOTSEL pad wired straight to the BOOTSEL button
    by = 115.57
    for bx, r, sw, title in ((40.64, "R25", "SW18", "BOOTSEL button"), (152.4, "R26", "SW19", "RESET button")):
        sh.heading(bx - 12.7, by - 17.78, title)
        sh.place(r, bx, by, 0, f="r")
        sh.tag(f"{r}.1", G, d="up")
        sh.place(sw, bx, by + 3.81 + 5.08 + 10.16, 270, f="l")
        sh.wire(f"{r}.2", f"{sw}.1")
        sh.tag(f"{sw}.2", 0)
    yb = by + 3.81 + 2.54                            # the BOOTSEL node
    x = 83.82
    sh.heading(x - 22.86, by - 17.78, "Debug pads J7 (back side, 2.54 mm)")
    sh.place("J7", x, yb - 5.08, 0, f="r")
    sh.wire((40.64, yb), "J7.5")
    for n in ("4", "3", "2"):
        sh.tag(f"J7.{n}")
    sh.tag("J7.1")
    sh.tag("J7.6")
    sh.text(17.78, 165.1, "J5 SWD follows the Raspberry Pi 3-pin debug connector pinout (Debug Probe cable).")
    sh.text(17.78, 168.91, "SW18 BOOTSEL and SW19 RESET sit under pinholes in the lid. J7: power through its 3V3 pad only with the")
    sh.text(17.78, 172.72, "cell out and nothing on USB; BOOTSEL to GND at reset enters the USB boot loader, RUN to GND resets.")
    return sh


def usb_charger():
    sh = Sheet("usb_charger.kicad_sch")
    VB = 45.72                                   # the VBUS line across the top of the sheet
    # ---- USB-C receptacle, CC pull-downs and sense taps
    Jx, Jy = 30.48, 60.96
    sh.heading(17.78, 25.4, "USB-C receptacle + CC")
    sh.place("J1", Jx, Jy, 0, f=(0, -22.86))
    sh.nc("J1.A8"); sh.nc("J1.B8")
    for n in ("A9", "B4", "B9", "A12", "B1", "B12"):
        pass                                     # stacked on A4 / A1
    sh.wire("J1.A4", (VB_END := 139.7, VB))
    sh.rail("VBUS", sh.wire((58.42, VB), (58.42, VB - G)))
    # CC2 hangs off its own row nearest the connector, CC1 runs on past it
    for net, row, xs, pd, dnp, ser in (("CC2", 53.34, (55.88, 68.58, 83.82), "R2", "C43", "R4"),
                                      ("CC1", 50.8, (91.44, 104.14, 119.38), "R1", "C42", "R3")):
        pin = "J1.B5" if net == "CC2" else "J1.A5"
        sh.wire(pin, (xs[2], row))
        for x, ref in zip(xs, (pd, dnp, ser)):
            sh.place(ref, x, row + 3.81, 0, f=("l" if ref == ser else "r", G))
            sh.wire((x, row), f"{ref}.1")
        sh.tag(f"{pd}.2", 0); sh.tag(f"{dnp}.2", 0)
        sh.tag(f"{ser}.2", G)
    # D- (upper pair) and D+ (lower pair) joined and taken down, then right
    sh.wire("J1.A7", (50.8, 58.42)); sh.wire("J1.B7", (50.8, 60.96))
    sh.wire((50.8, 58.42), (50.8, 71.12), (88.9, 71.12))
    sh.label("USB_DM", (88.9, 71.12), "right")
    sh.wire("J1.A6", (48.26, 63.5)); sh.wire("J1.B6", (48.26, 66.04))
    sh.wire((48.26, 63.5), (48.26, 78.74), (88.9, 78.74))
    sh.label("USB_DP", (88.9, 78.74), "right")
    # ground: shield and GND pins together, with the GND power flag
    sh.wire("J1.SH", (22.86, 86.36), (38.1, 86.36))
    sh.wire("J1.A1", (30.48, 86.36))
    sh.gnd((38.1, 86.36))
    sh.flag("#FLG02", (34.29, 86.36))
    # ---- ESD array under D+/D- (IO1 = D+ is its upper pin, so D- crosses D+ once)
    Ex, Ey = 73.66, 101.6
    sh.heading(20.32, 99.06, "ESD (SRV05-4)")
    sh.place("U1", Ex, Ey, 0, f=(12.7, 13.97))
    sh.wire((58.42, 78.74), (58.42, Ey - 2.54), "U1.1")
    sh.wire((53.34, 71.12), (53.34, Ey + 2.54), "U1.3")
    sh.nc("U1.4"); sh.nc("U1.6")
    sh.tag("U1.5", G); sh.tag("U1.2", 0)
    # ---- VBUS input: TP4056's RC against plug-in ringing, and the bleeder
    sh.heading(119.38, 35.56, "VBUS input")
    sh.flag("#FLG01", (121.92, VB))
    sh.place("R38", 127.0, VB + 3.81, 0, f=("r", G))
    sh.wire((127.0, VB), "R38.1"); sh.tag("R38.2", 0)
    sh.place("R41", 139.7, VB + 3.81, 0, f=("l", G))
    sh.place("C1", 139.7, VB + 7.62 + 3.81, 0, f=("r", G))
    sh.wire((139.7, VB), "R41.1"); sh.tag("C1.2", 0)
    sh.label("VBUS_C1", sh.wire("R41.2", (142.24, VB + 7.62)), "right")
    # ---- charger: BQ24073 power path (2026-10-21, row 16), every pin labelled
    Ux, Uy = 190.5, 78.74
    sh.heading(160.02, 35.56, "Charger and power path (BQ24073)")
    sh.place("U2", Ux, Uy, 0, f=(0, -20.32))
    for n in ("13", "10", "11", "2", "3", "16", "12", "6", "5", "4", "15", "1", "7", "9", "8", "17"):
        sh.tag(f"U2.{n}", 5.08)
    sh.nc("U2.14")
    # its resistors, the thermistor and the caps, in a row under it
    yr = 132.08
    sh.heading(101.6, yr - 15.24, "Charge current, input mode, status, cell temperature")
    for i, ref in enumerate(("R5", "R64", "R43", "R44", "R9", "R10", "TH1", "C3", "C2")):
        x = 106.68 + i * 12.7
        sh.place(ref, x, yr, 0, f=("r", G))
        sh.tag(f"{ref}.1", G)
        sh.tag(f"{ref}.2", G)
    tp(sh, "TP7", sh.wire((sh.sym["TH1"]["x"] - 5.08, yr - 7.62), (sh.sym["TH1"]["x"] - 5.08, yr - 7.62)))
    sh.label("CHG_TEMP", (sh.sym["TH1"]["x"] - 5.08, yr - 7.62), "left")
    # ---- battery connector
    bx, by = 259.08, 129.54
    sh.heading(231.14, 119.38, "Battery connector J2")
    sh.place("J2", bx, by, 0, f="r")
    sh.nc("J2.MP")
    sh.place("JP5", bx - 20.32, by, 180)
    sh.wire("J2.1", "JP5.1")
    sh.rail("VBAT", sh.stub("JP5.2", G))
    sh.wire("J2.2", (bx - 7.62, by + 2.54), (bx - 7.62, by + 5.08), v=False)
    sh.gnd((bx - 7.62, by + 5.08))
    sh.text(17.78, 177.8, "BQ24073: OUT (VSYS) 4.4 V from USB with the system first, 475 mA in (USB500, EN1 high), 270 mA charge (R5),")
    sh.text(17.78, 181.61, "5 h timer, OVP 6.6 V, TS window 3.5-47 C (TH1 B3950 at the cell hole's edge). Without USB the cell feeds OUT.")
    sh.text(17.78, 185.42, "CC1/CC2_SENSE let firmware read the source's current offer (dim the backlight on a 500 mA port).")
    return sh


def tp(sh, ref, at, side="r", down=False):
    """Test point ref with its pin on `at` (on a wire), pad above (or below, at the end
    of a wire coming down), its name and net stacked beside the pad (side 'r' or 'l')."""
    at = sh.P(at)
    w = max(tw(ref), tw(sh.parts[ref]["value"])) / 2 + 1.3
    sh.place(ref, at[0], at[1], 180 if down else 0, f=(w if side == "r" else -w, 2.84 if down else -2.84))
    return at


def power():
    sh = Sheet("power.kicad_sch")
    # ---- 3V3 LDO
    Ux, Uy = 88.9, 45.72
    sh.heading(17.78, 25.4, "3V3 regulator (RT9013)")
    sh.place("U3", Ux, Uy, 0, f=(0, -10.16))
    vin = (45.72, Uy - 2.54)
    sh.wire(vin, "U3.1")
    sh.rail("VSYS", vin)
    tp(sh, "TP2", (55.88, Uy - 2.54))
    sh.place("C5", 68.58, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.wire((68.58, Uy - 2.54), "C5.1"); sh.tag("C5.2", 0)
    # CE: straight down to the latch's PWR_EN node, round the right end of its diode row
    sh.wire("U3.3", (Ux - 12.7, Uy), (Ux - 12.7, Uy + 12.7), (116.84, Uy + 12.7), (116.84, 104.14))
    sh.nc("U3.4")
    sh.tag("U3.2", 0)
    sh.place("C6", 104.14, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.place("C7", 114.3, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.place("JP1", 129.54, Uy - 2.54, 0)
    sh.wire("U3.5", "JP1.1")
    for c in ("C6", "C7"):
        sh.wire((sh.sym[c]["x"], Uy - 2.54), f"{c}.1"); sh.tag(f"{c}.2", 0)
    sh.wire("JP1.2", (160.02, Uy - 2.54))
    sh.flag("#FLG07", (135.89, Uy - 2.54))
    tp(sh, "TP4", (149.86, Uy - 2.54))
    sh.rail("+3V3", (160.02, Uy - 2.54))
    sh.text(119.38, Uy + 10.16, "JP1: cut at bring-up to check the")
    sh.text(119.38, Uy + 13.97, "regulator alone (3V3_LDO)")
    # ---- power latch: button, MCU hold, USB, wired-OR into the LDO's CE
    sh.heading(17.78, 72.39, "Power latch (button, MCU hold, USB)")
    Dx, Dy = 76.2, 88.9
    sh.place("D2", Dx, Dy, 0, f=(-6.35, -6.35))
    raw = (33.02, Dy)
    sh.wire(raw, "D2.1")
    sh.label("PWR_BTN_RAW", raw, "left")
    sh.place("R13", 40.64, Dy + 3.81, 0, f=("r", G))
    sh.wire((40.64, Dy), "R13.1"); sh.tag("R13.2", 0)
    hold = (99.06, Dy)
    sh.wire("D2.2", hold)
    sh.label("PWR_HOLD", hold, "right")
    sh.place("R42", 88.9, Dy + 3.81, 0, f=("r", G))
    sh.wire((88.9, Dy), "R42.1"); sh.tag("R42.2", 0)
    Ye = 104.14
    sh.wire("D2.3", (Dx, Ye), (118.11, Ye))
    sh.wire((Dx, Ye), (Dx - 7.62, Ye))
    tp(sh, "TP6", (Dx - 7.62, Ye), "l")
    sh.place("R15", 91.44, Ye + 3.81, 0, f=("r", G))
    sh.place("C47", 101.6, Ye + 3.81, 0, f=("r", G))
    for r in ("R15", "C47"):
        sh.wire((sh.sym[r]["x"], Ye), f"{r}.1"); sh.tag(f"{r}.2", 0)
    sh.place("D3", 121.92, Ye, 0)
    sh.place("R14", 140.97, Ye, 270)
    sh.wire("D3.2", "R14.2")
    v = sh.stub("R14.1", 15.24)
    tp(sh, "TP1", (149.86, Ye))
    sh.rail("VBUS", v)
    # ---- button sense: Q1 lets the MCU read the power button without seeing VSYS
    Qy = 134.62
    sh.heading(17.78, 121.92, "Button sense")
    Qx = 58.42
    sh.place("Q1", Qx, Qy, 0, f=("r", G))
    sh.wire((48.26, Dy), (48.26, Qy), "Q1.1")
    sh.tag("Q1.2", 0)
    sh.place("R16", Qx + 2.54, Qy - 8.89, 180, f=("l", 0))
    sh.tag("R16.2", 0)
    sh.wire("Q1.3", (Qx + 7.62, Qy - 5.08))
    sh.label("PWR_BTN_N", (Qx + 7.62, Qy - 5.08), "right")
    # ---- seat 2's Start (row 1, 2026-10-21): D8 ORs it into PWR_EN, Q7 reads it for GPIO2
    sh.heading(101.6, 121.92, "Seat 2 Start: power on and sense")
    for i, ref in enumerate(("D8", "R62", "Q7", "R63")):
        x = 109.22 + i * 17.78
        sh.place(ref, x, 139.7, 0 if ref != "D8" else 90, f=("r", G) if ref != "Q7" else "r")
        for n in (("1", "2") if ref != "Q7" else ("1", "2", "3")):
            sh.tag(f"{ref}.{n}", G)
    # ---- cell voltage divider
    bx, by = 172.72, 86.36
    sh.heading(160.02, 68.58, "Cell voltage sense")
    sh.place("R17", bx, by, 0, f=("l", -H))
    sh.place("R18", bx, by + 7.62, 0, f=("l", H))
    top = sh.stub("R17.1", G)
    sh.rail("VBAT", top)
    tp(sh, "TP3", sh.wire(top, (bx + 5.08, top[1])))
    sh.wire("R17.2", "R18.1"); sh.tag("R18.2", 0)
    sh.place("C8", bx + 10.16, by + 3.81 + 3.81, 0, f=("r", G))
    sh.wire((bx, by + 3.81), (bx + 20.32, by + 3.81))
    sh.wire((bx + 10.16, by + 3.81), "C8.1"); sh.tag("C8.2", 0)
    sh.label("VBAT_DIV", (bx + 20.32, by + 3.81), "right")
    # U10 buffers the divider for GPIO47 (the pin's leakage flows from its output, not through the divider)
    sh.place("U10", bx + 50.8, by + 3.81, 0, f=(0, -10.16))
    sh.tag("U10.3", d="left", net="VBAT_DIV")
    sh.tag("U10.4", d="left")
    sh.tag("U10.1", d="right")
    sh.tag("U10.5", d="up")
    sh.tag("U10.2", d="down")
    sh.place("C54", bx + 66.04, by + 7.62, 0, f=("r", G))
    sh.tag("C54.1"); sh.tag("C54.2", 0)
    # ---- ground pads for probe and scope clips
    sh.heading(195.58, 124.46, "Ground pads (probe clips)")
    for i, ref in enumerate(("TP5", "TP15", "TP16")):
        a = tp(sh, ref, (200.66 + i * 15.24, 139.7))
        sh.gnd(sh.wire(a, (a[0], a[1] + G)))
    sh.text(17.78, 177.8, "Power: either seat's Start contact, the MCU (PWR_HOLD) or USB raise PWR_EN. Releasing it with")
    sh.text(17.78, 181.61, "PWR_HOLD low turns the device off. A reset drops PWR_HOLD, so a crash turns it off.")
    sh.text(17.78, 185.42, "Q1 lets the MCU see the button (PWR_BTN_N low = pressed) without exposing a pin to VSYS.")
    return sh


def display():
    sh = Sheet("display.kicad_sch")
    # ---- J3, pins to the right: panel pin 1 (LED_A) at the top, J3 pin j = panel pin 51 - j
    Jx, Jy = 22.86, 92.71
    T = Jx + 5.08                                  # pin tips
    sh.place("J3", Jx, Jy, 180, f=(5.08, 71.12))
    yj = lambda j: round(Jy + 60.96 - (j - 1) * 2.54, 2)
    pins = sh.parts["J3"]["pins"]
    sh.nc("J3.MP")
    for j in range(1, 51):
        if pins[str(j)] is None:
            sh.nc(f"J3.{j}")

    def bus(js, x=T + 2.54):
        for j in js:
            sh.wire(f"J3.{j}", (x, yj(j)))
        sh.wire((x, yj(max(js))), (x, yj(min(js))))

    # backlight anode through JP2 from VSYS
    sh.place("JP2", T + 15.24, 21.59, 180)
    sh.wire("J3.50", (T + 7.62, yj(50)), (T + 7.62, 21.59), "JP2.2")
    sh.rail("VSYS", sh.stub("JP2.1", G))
    sh.tag("J3.49", 5.08); sh.tag("J3.48", 5.08)
    # panel logic supply pins
    for grp in ((44, 43, 42), (11, 10, 9)):
        bus(grp)
        sh.label("LCD_3V3", sh.wire((T + 2.54, yj(grp[1])), (T + 7.62, yj(grp[1]))), "right")
    sh.tag("J3.41", 5.08)
    # ground pins: panel 11-32 (RGB bus and DB0-17 held low), 43, 48-50
    bus(range(19, 41))
    sh.gnd(sh.wire((T + 2.54, yj(35)), (T + 5.08, yj(35))))
    sh.gnd(sh.wire("J3.8", (T + 5.08, yj(8))))
    bus((1, 2, 3))
    sh.gnd((T + 2.54, yj(1)))
    # SPI pins fan out upwards into 7.62 mm lanes
    lanes = [(18, 60.96), (17, 68.58), (16, 76.2), (15, 83.82), (14, 91.44), (13, 99.06), (12, 106.68)]
    for i, (j, ly) in enumerate(lanes):
        sh.wire(f"J3.{j}", (35.56 + i * 2.54, yj(j)), (35.56 + i * 2.54, ly), v=False)
    lane = {j: (35.56 + i * 2.54, ly) for i, (j, ly) in enumerate(lanes)}
    sh.heading(53.34, 54.61, "SPI series resistors, probe points")
    for j, net in ((18, "LCD_MISO"), (16, "LCD_3V3"), (12, "LCD_TE")):
        sh.label(net, sh.wire(lane[j], (55.88, lane[j][1])), "right")
    for j, tpr, r in ((17, "TP12", "R29"), (15, "TP11", "R28")):
        ly = lane[j][1]
        sh.place(r, 83.82, ly, 270)
        sh.wire(lane[j], f"{r}.2")
        tp(sh, tpr, (68.58, ly))
        sh.label(sh.parts[r]["pins"]["1"], sh.stub(f"{r}.1", 5.08), "right")
    for j, tpr, side in ((14, "TP14", -3.81), (13, "TP13", -3.81)):
        ly = lane[j][1]
        sh.label(sh.parts["J3"]["pins"][str(j)], sh.wire(lane[j], (66.04, ly)), "right")
        tp(sh, tpr, (58.42, ly), "l" if side < 0 else "r")
    # ---- panel logic supply: JP4 from 3V3, decoupling at the connector
    sy = 132.08
    sh.heading(66.04, 121.92, "Panel supply (LCD_3V3)")
    sh.place("JP4", 83.82, sy, 0)
    sh.rail("+3V3", sh.stub("JP4.1", 5.08))
    sh.wire("JP4.2", (114.3, sy))
    sh.flag("#FLG08", (93.98, sy))
    for x, c in ((101.6, "C29"), (111.76, "C30")):
        sh.place(c, x, sy + 3.81, 0, f=("r", G))
        sh.wire((x, sy), f"{c}.1"); sh.tag(f"{c}.2", 0)
    sh.label("LCD_3V3", sh.wire((114.3, sy), (119.38, sy)), "right")
    # ---- backlight: reference filter and two current sinks around U6 (MCP6002)
    Ux, Uy = 200.66, 80.01
    sh.place("U6", Ux, Uy, 0, f=(0, 21.59))
    sh.heading(Ux - 66.04, Uy - 10.16, "Backlight reference")
    ry = Uy + 5.08
    sh.place("R60", Ux - 50.8, ry, 90)
    pwm = (Ux - 66.04, ry)
    sh.wire(pwm, "R60.1")
    sh.label("BL_PWM", pwm, "left")
    sh.wire("R60.2", "U6.5")
    for x, ref in ((Ux - 40.64, "R61"), (Ux - 30.48, "C50")):
        sh.place(ref, x, ry + 3.81, 0, f=("l", G))
        sh.wire((x, ry), f"{ref}.1"); sh.tag(f"{ref}.2", 0)
    tp(sh, "TP10", (Ux - 25.4, ry))
    sh.label("BL_REF", sh.wire((Ux - 35.56, ry), (Ux - 35.56, ry - G)), "up")
    sh.label("BL_REF", sh.wire("U6.3", (Ux - 12.7, Uy - 7.62), (Ux - 12.7, Uy - 10.16)), "up")
    sh.place("C51", Ux - 22.86, Uy - 15.24, 0, f="l")
    sh.tag("C51.1", 0); sh.tag("C51.2", 0)
    sh.tag("U6.8", 0); sh.tag("U6.4", 0)
    for ch, half in ((2, "top"), (1, "bottom")):
        q, rs, rg, rpd, rf, cf = {1: ("Q5", "R52", "R54", "R56", "R58", "C48"),
                                  2: ("Q6", "R53", "R55", "R57", "R59", "C49")}[ch]
        out, inn = ("U6.1", "U6.2") if ch == 2 else ("U6.7", "U6.6")
        sgn = -1 if half == "top" else 1
        yo = sh.P(out)[1]
        drive = yo if half == "top" else Uy + 25.4            # gate drive row
        crow = Uy - 17.78 if half == "top" else Uy + 33.02     # C feedback row
        frow = Uy - 25.4 if half == "top" else Uy + 45.72      # R feedback row
        sh.heading(Ux + 15.24, (Uy - 33.02) if half == "top" else (Uy + 53.34),
                   f"Backlight sink ch{ch} (LED_K{ch})")
        # -IN up (or down) to the feedback rows
        sh.wire(inn, (Ux - 15.24, sh.P(inn)[1]), (Ux - 15.24, frow), v=False)
        # output riser to the C row and the drive row
        sh.wire(out, (Ux + 12.7, yo))
        sh.wire((Ux + 12.7, yo), (Ux + 12.7, crow))
        sh.place(cf, Ux - 7.62, crow, 270)
        sh.wire((Ux - 15.24, crow), f"{cf}.2"); sh.wire(f"{cf}.1", (Ux + 12.7, crow))
        sh.place(rg, Ux + 20.32, drive, 90)
        sh.wire((Ux + 12.7, drive), f"{rg}.1")
        sh.place(q, Ux + 40.64, drive, 0, f="r")
        sh.wire(f"{rg}.2", f"{q}.1")
        sh.place(rpd, Ux + 30.48, drive + 3.81, 0, f=("r", G))
        sh.wire((Ux + 30.48, drive), f"{rpd}.1"); sh.tag(f"{rpd}.2", 0)
        sh.label(f"LED_K{ch}", sh.stub(f"{q}.3", G), "up")
        node = sh.stub(f"{q}.2", G)
        sh.place(rs, Ux + 43.18, node[1] + 3.81, 0, f=("r", G))
        sh.tag(f"{rs}.2", 0)
        sh.wire(node, (Ux + 55.88, node[1]), (Ux + 55.88, frow), v=False)
        sh.place(rf, Ux + 15.24, frow, 270)
        sh.wire((Ux + 55.88, frow), f"{rf}.1"); sh.wire(f"{rf}.2", (Ux - 15.24, frow))
    sh.text(17.78, 175.26, "EastRising ER-TFT035IPS-6 (ILI9488, 4-wire SPI); its 50-pin FPC folds under the panel into J3 (ZIF). J3 pin j is panel pin 51 - j.")
    sh.text(17.78, 179.07, "Unused bus pins as the datasheet says: DB0-17 and RGB low, RD and IM2-0 high. Backlight: LED_A on VSYS; a current sink per")
    sh.text(17.78, 182.88, "cathode pin (U6 half, Q5/Q6, 2.2 ohm R52/R53). GPIO28 PWM, filtered by R60/R61/C50, sets BL_REF: 0.124 V (56 mA a pin)")
    sh.text(17.78, 186.69, "at 100%. Stability: sim/backlight.py. JP2 and JP4: cut to measure the LED or panel logic current.")
    return sh


def mcu():
    sh = Sheet("mcu.kicad_sch")
    Ux, Uy = 140.97, 96.52
    sh.place("U4", Ux, Uy, 0, f=(0, 0))
    pins = sh.parts["U4"]["pins"]
    import kisym
    geo = kisym.placed(sh.parts["U4"]["lib"], Ux, Uy, 0)
    # right side: GPIO0-39 straight out to their labels
    for num, g in geo.items():
        if g["out"] != "right":
            continue
        net = pins[num]
        if net is None:
            sh.nc(f"U4.{num}")
        else:
            sh.tag(f"U4.{num}", 5.08)
    # top: supply pins on two short buses, the regulator's own pins labelled
    yb = Uy - 60.96
    for dx in (-12.7, -10.16, -5.08, -2.54, 2.54):
        sh.wire((Ux + dx, Uy - 55.88), (Ux + dx, yb))
    sh.wire((Ux - 12.7, yb), (Ux + 2.54, yb))
    sh.rail("+3V3", sh.wire((Ux - 7.62, yb), (Ux - 7.62, yb - G)))
    for dx in (7.62, 12.7):
        sh.wire((Ux + dx, Uy - 55.88), (Ux + dx, yb))
    sh.wire((Ux + 7.62, yb), (Ux + 12.7, yb))
    sh.rail("+1V1", sh.wire((Ux + 10.16, yb), (Ux + 10.16, yb - G)))
    for num in ("61", "63"):
        sh.label(pins[num], sh.stub(f"U4.{num}", G), "up", vertical=True)
    sh.wire("U4.62", (Ux - 2.54, Uy + 60.96), (Ux, Uy + 60.96))
    sh.wire("U4.81", (Ux, Uy + 60.96))
    sh.gnd((Ux, Uy + 60.96))
    # left side
    L = Ux - 25.4
    sh.label("RUN", sh.wire("U4.35", (Ux - 35.56, Uy - 33.02)), "left")
    # USB series resistors on the D-/D+ pin rows, staggered so each has room for its name
    sh.heading(58.42, 59.69, "USB series resistors")
    yd, yp = sh.P("U4.66")[1], sh.P("U4.67")[1]
    sh.place("R22", 86.36, yd, 90, f=("at", 86.36, yd - 3.81))
    sh.place("R21", 96.52, yp, 90, f=("at", 96.52, yp + 3.81))
    sh.wire("U4.66", "R22.2"); sh.wire("U4.67", "R21.2")
    sh.wire("R22.1", (74.93, yd)); sh.wire("R21.1", (74.93, yp))
    sh.label("USB_DM", (74.93, yd), "left"); sh.label("USB_DP", (74.93, yp), "left")
    for n in ("71", "72", "74", "73", "70"):
        sh.nc(f"U4.{n}")
    qs = (43.18, Uy - 15.24)
    sh.wire("U4.75", qs)
    sh.label("QSPI_SS", qs, "left")
    sh.place("R23", 53.34, Uy - 15.24 - 3.81, 0, f=("r", -H))
    sh.tag("R23.1", 0)
    # 12 MHz crystal under its pins: XIN straight across, XOUT through R20
    xi, xo = sh.P("U4.30")[1], sh.P("U4.31")[1]
    yx = xo + 5.08                                   # the crystal's row
    sh.heading(50.8, xi - 5.08, "12 MHz crystal")
    sh.place("X1", 73.66, yx, 0, f=("at", 73.66, yx - 6.99))
    sh.place("C27", 64.77, yx + 3.81, 0, f=("l", G))
    sh.place("C28", 82.55, yx + 3.81, 0, f=("r", G))
    sh.place("R20", 93.98, xo, 270)
    sh.wire("X1.1", (60.96, yx), (60.96, xi), "U4.30")
    sh.wire("X1.3", (86.36, yx), (86.36, xo), "R20.2")
    sh.wire("R20.1", "U4.31")
    sh.wire((64.77, yx), "C27.1"); sh.tag("C27.2", 0)
    sh.wire((82.55, yx), "C28.1"); sh.tag("C28.2", 0)
    sh.wire("X1.2", (73.66, yx + 7.62)); sh.gnd((73.66, yx + 7.62))
    sh.label("SWCLK", sh.wire("U4.33", (Ux - 38.1, Uy + 22.86)), "left")
    sh.label("SWDIO", sh.wire("U4.34", (Ux - 38.1, Uy + 25.4)), "left")
    # GPIO40-47 (the ADC pins) fan out into 5.08 mm lanes, each with its label above
    adc = ["49", "52", "53", "54", "55", "56", "57", "58"]
    for k, num in enumerate(adc):
        if pins[num] is None:
            sh.nc(f"U4.{num}")
            continue
        y = Uy + 33.02 + 2.54 * k
        lane = Uy + 33.02 + 5.08 * k
        end = (Ux - 63.5, lane)
        if k == 0:
            sh.wire(f"U4.{num}", end)
        else:
            rx = L - 2.54 * (9 - k)
            sh.wire(f"U4.{num}", (rx, y), (rx, lane), end)
        sh.label(pins[num], end, "left")
    sh.text(198.12, 20.32, "Core from Raspberry Pi's RP2350B minimal design (MIT),")
    sh.text(198.12, 24.13, "fitted with an RP2354B: its 2 MB flash is inside the package,")
    sh.text(198.12, 27.94, "so only QSPI_SS (the BOOTSEL strap, R23 pull-up) is wired outside.")
    return sh


def mcu_power():
    sh = Sheet("mcu_power.kicad_sch")
    # which U4 pin each cap serves, from the routed board (the core's caps sit where Raspberry
    # Pi's minimal design puts them, each on a short track to its pin); lines read top down
    SERVES = {
        "C14": ["5 IOVDD"], "C16": ["15 IOVDD"], "C19": ["24 IOVDD"], "C12": ["29 IOVDD"],
        "C17": ["41 IOVDD"], "C15": ["50 IOVDD"], "C18": ["59 ADC_AVDD", "60 IOVDD"],
        "C11": ["68 USB_OTP_VDD", "69 QSPI_IOVDD"], "C13": ["76 IOVDD"],
        "C10": ["3V3 feed,", "no U4 pin"], "C23": ["bulk, by", "regulator"],
        "C20": ["10 DVDD"], "C21": ["32 DVDD"], "C22": ["51 DVDD"],
        "C24": ["64 VREG_VIN"], "C26": ["61 VREG_AVDD"], "C25": ["65 VREG_FB", "(output)"],
    }

    def serves(c, x, y):
        """The pin(s) cap c serves, centred on x, the last line just above y (its tap)."""
        lines = SERVES[c]
        for i, s in enumerate(lines):
            sh.text(x, y - 2.54 - 2.03 * (len(lines) - 1 - i), s, align="c")

    def cap_row(x0, y0, caps, rail, title):
        """Caps side by side between a supply wire on top and a ground wire below, each
        with the pin it serves over its tap."""
        sh.heading(x0 - 10.16, y0 - 12.7, title)
        xs = [x0 + 17.78 * i for i in range(len(caps))]
        sh.rail(rail, (x0 - 10.16, y0))
        sh.wire((x0 - 10.16, y0), (xs[-1], y0))
        for x, c in zip(xs, caps):
            sh.place(c, x, y0 + 6.35, 0, f=("r", 0))
            sh.wire((x, y0), f"{c}.1")
            sh.wire(f"{c}.2", (x, y0 + 12.7))
            serves(c, x, y0)
        sh.wire((xs[0], y0 + 12.7), (xs[-1], y0 + 12.7))
        sh.gnd((xs[len(xs) // 2], y0 + 12.7))
        return xs

    # in U4's pin order
    cap_row(30.48 + 5.08, 45.72, ["C14", "C16", "C19", "C12", "C17", "C15", "C18", "C11", "C13", "C10", "C23"], "+3V3",
            "+3V3 decoupling, by U4 pin: 100 nF at each IOVDD, ADC_AVDD, USB_OTP_VDD, QSPI_IOVDD; bulk C23")
    cap_row(30.48 + 5.08, 88.9, ["C20", "C21", "C22"], "+1V1", "DVDD decoupling (+1V1), by U4 pin")
    # ---- the core regulator: VREG_AVDD filter, VREG_VIN cap, inductor and output cap
    sh.heading(83.82, 76.2, "Core 1.1 V switcher: VREG_VIN cap, AVDD filter, inductor, output")
    y = 88.9
    sh.rail("+3V3", (88.9, y))
    sh.place("C24", 99.06, y + 3.81, 0, f=("r", G))
    sh.wire((88.9, y), (106.68, y)); sh.wire((99.06, y), "C24.1"); sh.tag("C24.2", 0)
    serves("C24", 99.06, y)
    sh.place("R19", 111.76, y, 90)
    sh.wire((106.68, y), "R19.1")
    sh.place("C26", 124.46, y + 3.81, 0, f=("r", G))
    sh.wire("R19.2", (142.24, y))
    sh.wire((124.46, y), "C26.1"); sh.tag("C26.2", 0)
    serves("C26", 124.46, y)
    sh.flag("#FLG06", (134.62, y))
    sh.label("VREG_AVDD", (142.24, y), "right")
    yl = 121.92
    lx = (88.9, yl)
    sh.place("L2", 101.6, yl, 270)
    sh.wire(lx, "L2.2")
    sh.label("VREG_LX", lx, "left")
    sh.wire("L2.1", (147.32, yl))
    sh.place("C25", 116.84, yl + 3.81, 0, f=("r", G))
    sh.wire((116.84, yl), "C25.1"); sh.tag("C25.2", 0)
    serves("C25", 116.84, yl)
    sh.flag("#FLG05", (127.0, yl))
    tp(sh, "TP9", (137.16, yl))
    sh.rail("+1V1", (147.32, yl))
    sh.text(17.78, 170.18, "RP2354B supply, as in Raspberry Pi's minimal design: a 100 nF beside every IOVDD, DVDD, ADC, OTP and QSPI pin")
    sh.text(17.78, 173.99, "(the number and name over each cap is the U4 pin it sits by on the board), 4.7 uF at the regulator's input")
    sh.text(17.78, 177.8, "and output, and L2 (a polarity-marked 3.3 uH inductor, fit as drawn). C10 is the 100 nF Raspberry Pi gave its")
    sh.text(17.78, 181.61, "external flash; the RP2354B has its flash inside, so C10 sits on the 3V3 feed to the core, by no U4 pin.")
    return sh


# ---- root sheet: the sheet symbols, what each sheet holds, mechanical parts
SHEET_NOTES = {
    "USB_Charger": ("USB-C receptacle + CC pull-downs", "ESD (SRV05-4), VBUS input",
                    "charger (TP4056), load sharing", "battery connector J2"),
    "Power": ("3V3 regulator (RT9013)", "power latch (button, MCU, USB)",
              "button sense, cell voltage sense", "ground pads"),
    "MCU": ("RP2354B (2 MB flash inside)", "USB series resistors",
            "12 MHz crystal", ""),
    "MCU_Power": ("+3V3 and DVDD decoupling", "core 1.1 V switcher: VREG_VIN cap,",
                  "AVDD filter, inductor, output", ""),
    "Debug": ("SWD connector J5", "debug pads J7",
              "BOOTSEL and RESET buttons", ""),
    "Display": ("panel connector J3 (50-pin ZIF)", "SPI series resistors, panel supply",
                "backlight reference", "backlight current sinks ch1, ch2"),
    "Audio": ("PWM low-pass filter, DC block", "amplifier (FM8002A) and supply",
              "shutdown control", "speaker plug J4"),
    "Buttons": ("seat 1 and seat 2 contacts", "Start/Select per seat (P1 Start",
                "is the power button),", "20 contacts, one GPIO each"),
}


def SHEET_BOX(i):
    """x, y, width, height of the i-th sheet symbol on the root sheet."""
    return (20.32 + (i % 4) * 66.04, 50.8 + (i // 4) * 50.8, 58.42, 33.02)


def root():
    import design
    sh = Sheet("battler.kicad_sch")
    sh.text(20.32, 20.32, "Two-player handheld: reference design (REF-A)", size=2.0)
    sh.text(20.32, 27.94, "RP2354B, 3.5in 320x480 IPS (ILI9488, SPI), 20 membrane contacts under DS Lite silicone,")
    sh.text(20.32, 31.75, "one Li-ion cell (in a hole in the board) on a BQ24073 charger, RT9013 3V3, PWM audio into an FM8002A.")
    sh.text(20.32, 35.56, "Every sheet is generated from board/design.py by tools/build_sch.py. Edit design.py, not the sheets.")
    for i, (f, name) in enumerate(design.SHEETS):
        x, y, w, h = SHEET_BOX(i)
        sh.sheets.append((f, name, x, y, w, h))
        for j, line in enumerate(SHEET_NOTES[name]):
            if line:
                sh.text(x + 2.54, y + 6.35 + j * 4.45, line, size=1.0)
    sh.text(20.32, 154.94, "Mechanical: locating holes at positions set by the case; fiducials for assembly.")
    for i in range(6):
        sh.place(f"H{i + 1}", 25.4 + i * 7.62, 165.1, 0)
    for i in range(3):
        sh.place(f"FID{i + 1}", 76.2 + i * 12.7, 165.1, 0)
    return sh


LAYOUTS = {
    "battler.kicad_sch": root,
    "mcu.kicad_sch": mcu,
    "mcu_power.kicad_sch": mcu_power,
    "display.kicad_sch": display,
    "power.kicad_sch": power,
    "usb_charger.kicad_sch": usb_charger,
    "debug.kicad_sch": debug,
    "audio.kicad_sch": audio,
    "buttons.kicad_sch": buttons,
}
