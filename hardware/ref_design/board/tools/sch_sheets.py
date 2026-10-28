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
    # the third register: both Starts, both Selects and the charger's two status lines
    sh.heading(248.92, 70.0, "Third register: Start/Select and charger status")
    sh.place("U11", 284.48, 104.14, 0, f=(0, -15.24))
    for n in ("1", "2", "15", "8", "16", "10", "9", "11", "12", "13", "14", "3", "4", "5", "6"):
        sh.tag(f"U11.{n}", 5.08)
    sh.nc("U11.7")
    sh.place("C62", 304.8, 104.14, 0, f=("r", G))
    sh.tag("C62.1", G); sh.tag("C62.2", G)
    for k, r in enumerate(("R86", "R87")):
        sh.place(r, 256.54 + k * 12.7, 160.02, 0, f=("r", G))
        sh.tag(f"{r}.1", G); sh.tag(f"{r}.2", G)
    # the clock's series resistor (on the board by the module pin)
    sh.heading(116.84, 101.6, "SR_CLK source termination")
    sh.place("R65", 129.54, 111.76, 90)
    sh.tag("R65.1", 5.08)
    sh.tag("R65.2", 5.08)
    sh.text(17.78, 165.1, "D-pad and face buttons: bare ENIG combs under DS Lite silicone, read through a 74HC165 per seat (chained: U11 to U9 to U8, U9's")
    sh.text(17.78, 168.91, "Q7 into U8's DS; SR_PL low loads, 16 SR_CLK rises shift P1 Y..UP then P2 Y..UP out of SR_DATA, low = pressed; 8 more give U11).")
    sh.text(17.78, 172.72, "Start/Select: 4 mm metal domes (HYP 600-415S-260), read by U11 (24-clock read: the extra 8 bits: Start/Select/charger); both Starts sit on VSYS (Power sheet).")
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
    tp(sh, "TP9", (31.75, Yc))
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
    x = 96.52
    sh.heading(x - 22.86, 40.64, "Debug pads J7 (back side, 2.54 mm)")
    sh.place("J7", x, 55.88, 0, f="r")
    for n in ("1", "2", "3", "4"):
        sh.tag(f"J7.{n}")
    sh.text(17.78, 165.1, "J7: 3V3, EN, BOOT (IO0), GND on bare pads for a pogo jig or a clip. Power it through 3V3 only with the cell out and nothing on USB;")
    sh.text(17.78, 168.91, "EN to GND resets; IO0 to GND at reset enters the ROM's download mode (USB Serial/JTAG on the board's USB-C, or UART0 pads, not brought out).")
    sh.text(17.78, 172.72, "BOOT and RESET buttons are not on this board (no room for their pinholes beside the microSD slot, 2026-10-28).")
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
    sh.heading(17.78, 25.4, "3V3 regulator (TLV75733P)")
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
    # panel logic supply pins (VDDI x2, VCI, IM0/IM1, RD high)
    for grp in ((44, 43), (11, 10, 9)):
        bus(grp)
        sh.label("LCD_3V3", sh.wire((T + 2.54, yj(grp[1])), (T + 7.62, yj(grp[1]))), "right")
    sh.tag("J3.16", 5.08)
    # ground pins: panel 11-24 (RGB sync and DB17-DB8 held low), 9 (IM2), 34, 38, 43, 48-50
    bus(range(27, 41))
    sh.gnd(sh.wire((T + 2.54, yj(33)), (T + 5.08, yj(33))))
    for j in (42, 17, 13, 8):                    # between signal pins: the ground symbol hangs right on the stub
        sh.gnd(sh.stub(f"J3.{j}", 2.54, "right"))
    bus((1, 2, 3))
    sh.gnd((T + 2.54, yj(1)))
    # RST, TE, DC and the panel side of the series resistors
    for j in (41, 12, 14, 15, *range(19, 27)):
        sh.tag(f"J3.{j}", 7.62)
    # ---- 33 ohm in series on D0-D7 and WR, by the module (source termination), and the probe pads
    sh.heading(66.04, 140.97, "8080 bus: 33 ohm series resistors, probe points")
    for i, r in enumerate([f"R{70 + k}" for k in range(8)] + ["R78"]):
        sh.place(r, 66.04 + i * 12.7, 154.94, 0, f=("r", G))
        sh.tag(f"{r}.1")
        sh.tag(f"{r}.2")
    for i, t in enumerate(("TP11", "TP12", "TP13", "TP14")):
        sh.place(t, 190.5 + i * 7.62, 154.94, 0, f=("r", G))
        sh.tag(f"{t}.1")

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
    sh.text(17.78, 175.26, "EastRising ER-TFT035IPS-6 (ILI9488, 8-bit 8080 bus, IM = 011); its 50-pin FPC folds under the panel into J3 (ZIF). J3 pin j is panel pin 51 - j.")
    sh.text(17.78, 179.07, "Unused bus pins as the datasheet says: DB8-17 and RGB low, RD high, CSX low, SDA low, SDO open. Backlight: LED_A on VSYS; a current sink per")
    sh.text(17.78, 182.88, "cathode pin (U6 half, Q5/Q6, 2.2 ohm R52/R53). GPIO28 PWM, filtered by R60/R61/C50, sets BL_REF: 0.124 V (56 mA a pin)")
    sh.text(17.78, 186.69, "at 100%. Stability: sim/backlight.py. JP2 and JP4: cut to measure the LED or panel logic current.")
    return sh


def mcu():
    sh = Sheet("mcu.kicad_sch")
    Ux, Uy = 139.7, 101.6
    sh.place("U4", Ux, Uy, 0, f=(0, 0))
    pins = sh.parts["U4"]["pins"]
    sh.heading(Ux - 20.32, Uy - 38.1, "ESP32-S3-WROOM-1U-N16R8 (16 MB flash, 8 MB octal PSRAM, U.FL antenna connector)")
    for num, net in pins.items():
        if num in ("1", "2", "40", "41"):
            continue
        if net is None:
            sh.nc(f"U4.{num}")
        else:
            sh.tag(f"U4.{num}")
    sh.tag("U4.2", G)
    sh.tag("U4.1", G)
    # -- supply: 22 uF and 100 nF at the 3V3 pin
    sh.heading(17.78, 25.4, "3V3 at the module")
    sh.place("C10", 30.48, 45.72, 0, f=("r", G))
    sh.place("C11", 45.72, 45.72, 0, f=("r", G))
    for c in ("C10", "C11"):
        sh.tag(f"{c}.1", G)
        sh.tag(f"{c}.2", G)
    # -- EN: 10k to 3V3, 1 uF to ground; RESET (Debug sheet) pulls it low
    sh.heading(17.78, 63.5, "EN: power-up delay, never floating")
    sh.place("R23", 30.48, 83.82, 0, f=("r", G))
    sh.place("C12", 45.72, 83.82, 0, f=("r", G))
    sh.tag("R23.1", G)
    sh.tag("R23.2", G)
    sh.tag("C12.1", G)
    sh.tag("C12.2", G)
    # -- USB: 22 ohm in series close to the module, spare pads for capacitors
    sh.heading(17.78, 116.84, "USB D+/D- (module IO20/IO19)")
    sh.place("R21", 30.48, 134.62, 0, f=("r", G))
    sh.place("R22", 45.72, 134.62, 0, f=("r", G))
    sh.place("C13", 60.96, 134.62, 0, f=("r", G))
    sh.place("C14", 76.2, 134.62, 0, f=("r", G))
    for r in ("R21", "R22", "C13", "C14"):
        sh.tag(f"{r}.1", G)
        sh.tag(f"{r}.2", G)
    sh.text(198.12, 20.32, "ESP32-S3-WROOM-1U-N16R8 (Espressif, C3013946): Wi-Fi on, its antenna a flex antenna on a U.FL cable")
    sh.text(198.12, 24.13, "(no antenna on the board). IO35-IO37 are the octal PSRAM's. Strapping pins: IO0 BOOT, IO3 SR_DATA (JTAG source,")
    sh.text(198.12, 27.94, "ignored), IO45 AMP_EN (low at reset: R37) and IO46 CHG_OFF (low: R44). IO43/IO44 are UART0: LCD_D5/D6 toggle with")
    sh.text(198.12, 31.75, "the ROM's boot log while LCD_WR stays still. USB D-/D+ on IO19/IO20 (series 22 ohm, caps to ground not fitted).")
    return sh


def storage():
    sh = Sheet("storage.kicad_sch")
    Jx, Jy = 139.7, 96.52
    sh.heading(Jx - 38.1, 40.64, "microSD slot (push-push, card detect)")
    sh.place("J8", Jx, Jy, 0, f=(0, 0))
    for num in ("1", "2", "3", "4", "5", "7", "8", "9"):
        sh.tag(f"J8.{num}")
    for num in ("6", "10"):                      # next to signal pins: the ground symbol hangs right on the stub
        sh.gnd(sh.stub(f"J8.{num}", 2.54, "left"))
    sh.tag("J8.SH", G)
    # -- pull-ups on CMD, D0-D3 and the detect line
    sh.heading(17.78, 40.64, "Pull-ups (10k)")
    for i, r in enumerate(("R79", "R80", "R81", "R82", "R83", "R84")):
        x = 25.4 + i * 12.7
        sh.place(r, x, 63.5, 0, f=("r", G))
        sh.tag(f"{r}.1", G)
        sh.tag(f"{r}.2", G)
    # -- the card's supply through JP6, decoupling by the slot
    sh.heading(17.78, 91.44, "Card supply (SD_VDD)")
    sh.place("JP6", 30.48, 106.68, 0)
    sh.tag("JP6.1", G)
    sh.wire("JP6.2", (50.8, 106.68))
    sh.flag("#FLG10", (40.64, 106.68))
    sh.label("SD_VDD", sh.wire((50.8, 106.68), (55.88, 106.68)), "right")
    sh.place("C60", 53.34, 109.22, 0, f=("r", G))
    sh.place("C61", 66.04, 109.22, 0, f=("r", G))
    for c in ("C60", "C61"):
        sh.tag(f"{c}.1", G)
        sh.tag(f"{c}.2", G)
    sh.text(17.78, 165.1, "Hirose DM3AT-SF-PEJM5: push-push, 1.85 mm tall, its card-detect switch closes to ground with a card in (SD_CD low).")
    sh.text(17.78, 168.91, "4-bit SDMMC through the module's GPIO matrix: CMD IO11, CLK IO12, D0-D3 IO13-IO16, CD IO17. JP6: cut it to read the card's")
    sh.text(17.78, 172.72, "current or to power it from outside.")
    return sh


# ---- root sheet: the sheet symbols, what each sheet holds, mechanical parts
SHEET_NOTES = {
    "USB_Charger": ("USB-C receptacle + CC pull-downs", "ESD (SRV05-4), VBUS input",
                    "charger (TP4056), load sharing", "battery connector J2"),
    "Power": ("3V3 regulator (TLV75733P)", "power latch (button, MCU, USB)",
              "button sense, cell voltage sense", "ground pads"),
    "MCU": ("ESP32-S3-WROOM-1U-N16R8", "3V3 decoupling, EN delay",
            "USB series resistors", "(flex antenna on a U.FL cable)"),
    "Storage": ("microSD slot J8 (push-push)", "pull-ups, card supply JP6",
                "4-bit SDMMC + card detect", ""),
    "Debug": ("BOOT and RESET buttons", "debug pads J7 (3V3, EN, IO0, GND)",
              "", ""),
    "Display": ("panel connector J3 (50-pin ZIF)", "8080 bus series resistors, panel supply",
                "backlight reference", "backlight current sinks ch1, ch2"),
    "Audio": ("PWM low-pass filter, DC block", "amplifier (FM8002A) and supply",
              "shutdown control", "speaker plug J4"),
    "Buttons": ("seat 1 and seat 2 contacts", "Start/Select per seat (P1 Start",
                "is the power button),", "20 contacts + 2 Start/Select on 3 shift registers"),
}


def SHEET_BOX(i):
    """x, y, width, height of the i-th sheet symbol on the root sheet."""
    return (20.32 + (i % 4) * 66.04, 50.8 + (i // 4) * 50.8, 58.42, 33.02)


def root():
    import design
    sh = Sheet("battler.kicad_sch")
    sh.text(20.32, 20.32, "Two-player handheld: reference design (REF-A)", size=2.0)
    sh.text(20.32, 27.94, "ESP32-S3-WROOM-1U (Wi-Fi), 3.5in 320x480 IPS (ILI9488, 8080 bus), 20 membrane contacts under DS Lite silicone,")
    sh.text(20.32, 31.75, "one Li-ion cell (in a hole in the board) on a BQ24073 charger, TLV75733P 3V3, PWM audio into an FM8002A.")
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
    "storage.kicad_sch": storage,
    "display.kicad_sch": display,
    "power.kicad_sch": power,
    "usb_charger.kicad_sch": usb_charger,
    "debug.kicad_sch": debug,
    "audio.kicad_sch": audio,
    "buttons.kicad_sch": buttons,
}
