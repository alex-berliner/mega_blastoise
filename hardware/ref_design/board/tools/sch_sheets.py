"""Hand layouts of the reference schematic's sheets, block by block.

Each function returns a schdraw.Sheet: every part of the sheet placed (origin,
rotation, where its Reference/Value go), wired to its neighbours, and labelled
only where a net leaves its block. Coordinates are mm on the A4 page (x right,
y down), on KiCad's 1.27 mm grid. The parts, values and nets all come from
design.py; this file only says where things go and which pins a wire joins.
"""
from schdraw import Sheet, G

H = 1.27


def buttons():
    sh = Sheet("buttons.kicad_sch")
    for col, (seat, first) in enumerate((("Seat 1 (player 1)", 1), ("Seat 2 (player 2)", 9))):
        x0 = 35.56 + col * 110.49           # switch origins
        y0 = 35.56
        sh.heading(x0 - 17.78, y0 - 12.7, f"{seat}: membrane contacts")
        bus_x = x0 - 12.7
        gnd_rows = []
        for i in range(8):
            ref = f"SW{first + i}"
            y = y0 + i * 15.24
            sh.place(ref, x0, y, 180, f=(0, H))
            sh.tag(f"{ref}.1", length=5.08)
            if sh.parts[ref]["pins"]["2"] == "GND":
                sh.wire(f"{ref}.2", (bus_x, y))
                gnd_rows.append(y)
            else:                                   # P1 Y: the power button, on VSYS
                end = sh.stub(f"{ref}.2", 5.08)
                sh.rail("VSYS", end, "left")
        sh.wire((bus_x, gnd_rows[0]), (bus_x, gnd_rows[-1]))
        sh.gnd((bus_x, gnd_rows[-1]))
    sh.text(17.78, 158.75, "Seat 1's Y contact is also the power button: it joins VSYS to PWR_BTN_RAW (Power sheet),")
    sh.text(17.78, 162.56, "and the MCU reads it through Q1 as PWR_BTN_N.")
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
    sh.place("C39", 78.74, Yc, 90, f=(0, -8.89))
    sh.place("R34", 93.98, Yc, 90)
    a = (20.32, Yc)
    sh.wire(a, "R32.1")
    sh.label("AUDIO_PWM", a, "left", hook="up")
    sh.wire("R32.2", "R33.1"); sh.wire((44.45, Yc), "C37.1"); sh.tag("C37.2", 0)
    sh.wire("R33.2", "C39.1"); sh.wire((67.31, Yc), "C38.1"); sh.tag("C38.2", 0)
    sh.wire("C39.2", "R34.1")
    sh.heading(73.66, Yc - 17.78, "DC block, Ri")
    node = (101.6, Yc)
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
    sh.wire((Ux, Yv), "U7.6")
    sh.wire((Ux + 7.62, Yv), "C34.1"); sh.tag("C34.2", 0)
    sh.wire((Ux + 17.78, Yv), "C35.1"); sh.tag("C35.2", 0)
    sh.flag("#FLG09", (Ux + 12.7, Yv))
    sh.label("AMP_VDD", sh.wire((Ux + 2.54, Yv), (Ux + 2.54, Yv - G)), "up")
    sh.tag("U7.7", G)
    # -- bypass: +IN and BYPASS both on AMP_BYP, C40 to ground
    sh.tag("U7.3", 10.16)
    sh.place("C40", Ux - 17.78, Uy + 2.54 + 3.81, 0, f=("l", G))
    by = (Ux - 27.94, Uy + 2.54)
    sh.wire("U7.2", by)
    sh.label("AMP_BYP", by, "left")
    sh.wire((Ux - 17.78, Uy + 2.54), "C40.1"); sh.tag("C40.2", 0)
    # -- shutdown: R36 holds SHUTDOWN high (off), Q3 pulls it low when AMP_EN is high
    Ysd = Uy + 17.78
    sh.heading(Ux - 66.04, Ysd + 25.4, "Shutdown control")
    sh.wire("U7.1", (Ux - 12.7, Uy + 5.08), (Ux - 12.7, Ysd))
    sh.place("R36", Ux - 35.56, Ysd - 3.81, 180, f=("l", -G))
    sh.place("Q3", Ux - 25.4 - 2.54, Ysd + 7.62, 0, f="r")
    sl = (Ux - 55.88, Ysd)
    sh.wire((Ux - 12.7, Ysd), sl)
    sh.wire((Ux - 35.56, Ysd), "R36.1")
    sh.tag("R36.2")
    sh.wire((Ux - 25.4, Ysd), "Q3.3")
    sh.tag("Q3.2", 0)
    sh.label("AMP_SHDN", sl, "left", hook="up")
    gl = (Ux - 63.5, Ysd + 7.62)
    sh.wire("Q3.1", gl)
    sh.label("AMP_EN", gl, "left", hook="up")
    sh.place("R37", Ux - 43.18, Ysd + 7.62 + 3.81, 0, f=("r", G))
    sh.wire((Ux - 43.18, Ysd + 7.62), "R37.1"); sh.tag("R37.2", 0)
    # -- speaker
    Jx = Ux + 45.72
    sh.heading(Jx - 12.7, Yc - 10.16, "Speaker J4")
    sh.place("J4", Jx, Yc, 0, f=(7.62, 1.27))
    sh.wire("U7.5", "J4.1")
    sh.wire("U7.8", (Jx - 10.16, Uy + 2.54), (Jx - 10.16, Yc + 2.54), "J4.2", v=True)
    sh.text(17.78, 160.02, "AUDIO_PWM (GPIO8, PWM4 A, 8 bits: 586 kHz) through two RC stages (-3 dB ~14 kHz); gain 2 x R35/R34.")
    sh.text(17.78, 163.83, "AMP_EN high turns the amplifier on (Q3 pulls SHUTDOWN low). 8 ohm 1 W speaker on the J4 pads, from below.")
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
    three_pin("J6", 91.44, 50.8, "UART connector J6")
    # -- J7: the same SWD on bare pads (pogo jig)
    x, y = 165.1, 55.88
    sh.heading(x - 7.62, y - 20.32, "Debug pads J7 (back side, 2.54 mm)")
    sh.place("J7", x, y, 180, f="l")
    t = x + 5.08
    sh.wire("J7.6", (t + 2.54, y - 7.62), (t + 2.54, y - 12.7), (t + 7.62, y - 12.7), v=False)
    sh.gnd((t + 7.62, y - 12.7))
    for n in ("5", "4", "3", "2"):
        sh.tag(f"J7.{n}", 5.08)
    sh.rail("+3V3", sh.stub("J7.1", 5.08), "right")
    # -- BOOTSEL and RESET buttons
    for i, (r, sw, title) in enumerate((("R25", "SW18", "BOOTSEL button"), ("R26", "SW19", "RESET button"))):
        bx, by = 40.64 + i * 63.5, 115.57
        sh.heading(bx - 12.7, by - 17.78, title)
        sh.place(r, bx, by, 0, f="r")
        sh.tag(f"{r}.1", G, d="up")
        sh.place(sw, bx, by + 3.81 + 5.08 + 10.16, 270, f=(-3.81, 0))
        sh.wire(f"{r}.2", f"{sw}.1")
        sh.tag(f"{sw}.2", 0)
        if sh.parts[r]["pins"]["2"] == "BOOTSEL":
            end = sh.wire((bx, by + 3.81 + 2.54), (bx + 10.16, by + 3.81 + 2.54))
            sh.label("BOOTSEL", end, "right")
    sh.text(17.78, 165.1, "J5 SWD and J6 UART follow the Raspberry Pi 3-pin debug connector pinout (Debug Probe cable).")
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
    sh.wire("J1.A4", (VB_END := 217.17, VB))
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
    sh.wire((50.8, 58.42), (50.8, 71.12), (63.5, 71.12))
    sh.label("USB_DM", (63.5, 71.12), "right")
    sh.wire("J1.A6", (48.26, 63.5)); sh.wire("J1.B6", (48.26, 66.04))
    sh.wire((48.26, 63.5), (48.26, 78.74), (63.5, 78.74))
    sh.label("USB_DP", (63.5, 78.74), "right")
    # ground: shield and GND pins together, with the GND power flag
    sh.wire("J1.SH", (22.86, 86.36), (38.1, 86.36))
    sh.wire("J1.A1", (30.48, 86.36))
    sh.gnd((38.1, 86.36))
    sh.flag("#FLG02", (34.29, 86.36))
    # ---- ESD array on D+/D-
    Ex, Ey = 63.5, 127.0
    sh.heading(35.56, 104.14, "ESD (SRV05-4)")
    sh.place("U1", Ex, Ey, 0, f=(16.51, 10.16))
    sh.tag("U1.1", 7.62, hook="up"); sh.tag("U1.3", 7.62, hook="down")
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
    # ---- charger
    Ux, Uy = 180.34, 71.12
    sh.heading(160.02, 35.56, "Charger (TP4056)")
    sh.place("U2", Ux, Uy, 0, f=(12.7, -15.24))
    sh.wire((Ux, VB), "U2.4")
    sh.wire("U2.3", (Ux, Uy + 15.24)); sh.wire("U2.9", (Ux - 2.54, Uy + 15.24), (Ux, Uy + 15.24))
    sh.gnd((Ux, Uy + 15.24))
    # CE: R43 pulls it to VBUS, Q4 (CHG_OFF) pulls it low
    sh.place("R43", 160.02, VB + 3.81 + 7.62, 0, f=("l", 0))
    sh.wire((160.02, VB), "R43.1"); sh.wire("R43.2", (160.02, Uy - 5.08), "U2.8")
    sh.place("Q4", 157.48, Uy, 0, f="l")
    sh.wire((160.02, Uy - 5.08), "Q4.3"); sh.tag("Q4.2", 0)
    sh.place("R44", 139.7, Uy + 3.81, 0, f=("l", G))
    sh.wire("Q4.1", (116.84, Uy))
    sh.label("CHG_OFF", (116.84, Uy), "left", hook="up")
    sh.wire((139.7, Uy), "R44.1"); sh.tag("R44.2", 0)
    # status outputs: open drain, pulled up to 3V3 for the MCU
    yc, yd = 91.44, 101.6
    sh.wire("U2.7", (165.1, Uy), (165.1, yc), (121.92, yc))
    sh.label("CHG_STAT_N", (121.92, yc), "left", hook="up")
    sh.place("R10", 147.32, yc - 3.81, 180, f=("r", -H))
    sh.wire((147.32, yc), "R10.1"); sh.tag("R10.2", 0)
    sh.wire("U2.6", (167.64, Uy + 2.54), (167.64, yd), (96.52, yd))
    sh.label("CHG_DONE_N", (96.52, yd), "left", hook="up")
    sh.place("R9", 111.76, yd - 3.81, 180, f=("r", -H))
    sh.wire((111.76, yd), "R9.1"); sh.tag("R9.2", 0)
    # PROG and TEMP
    sh.place("R5", Ux + 12.7, Uy + 2.54 + 3.81 + 12.7, 0, f=("r", G))
    sh.wire("U2.2", "R5.1"); sh.tag("R5.2", 0)
    yt = 99.06
    sh.heading(187.96, 111.76, "Cell temperature window")
    sh.wire("U2.1", (Ux + 21.59, Uy), (Ux + 21.59, yt), (251.46, yt))
    sh.place("R39", 210.82, yt - 3.81, 0, f=("l", -H))
    sh.wire((210.82, yt), "R39.2"); sh.tag("R39.1", 0)
    for x, ref in ((220.98, "R40"), (233.68, "TH1"), (251.46, "C41")):
        sh.place(ref, x, yt + 3.81, 0, f=("r", G))
        sh.wire((x, yt), f"{ref}.1"); sh.tag(f"{ref}.2", 0)
    # BAT out to the load sharing switch and the cell
    YB = 66.04
    QX = 236.22                                  # Q2, D5, D7 column
    sh.wire("U2.5", (QX - 5.08, YB))
    sh.place("C3", 207.01, YB + 3.81, 0, f=("r", G))
    sh.wire((207.01, YB), "C3.1"); sh.tag("C3.2", 0)
    sh.rail("VBAT", sh.wire((214.63, YB), (214.63, YB - G)))
    # ---- load sharing: USB through R47/D5, the cell through Q2 (D7 across it)
    sh.heading(215.9, 35.56, "Load sharing")
    sh.place("R47", QX - 15.24, VB, 90)
    sh.place("D5", QX, VB, 180)
    sh.wire((VB_END, VB), "R47.1"); sh.wire("R47.2", "D5.2")
    XS = QX + 15.24                              # VSYS node
    sh.wire("D5.1", (XS, VB), (XS, YB))
    sh.place("D7", QX, 55.88, 180)
    sh.wire((QX - 17.78, YB), (QX - 17.78, 55.88), "D7.2"); sh.wire("D7.1", (XS, 55.88))
    sh.place("Q2", QX, YB + 2.54, 90, f=(-15.24, 5.08))
    sh.wire("Q2.2", (XS, YB))
    yg = 78.74
    sh.place("D6", QX + 7.62, yg, 0)
    sh.wire("Q2.1", (QX, yg), "D6.1")
    sh.rail("VBUS", sh.stub("D6.2", G))
    sh.place("R6", QX - 5.08, yg + 3.81, 0, f=("l", G))
    sh.wire((QX, yg), (QX - 5.08, yg), "R6.1"); sh.tag("R6.2", 0)
    sh.wire((QX, yg), (QX, yg + 10.16))
    sh.label("LS_GATE", (QX, yg + 10.16), "down")
    # VSYS out
    sh.rail("VSYS", sh.wire((XS, VB), (XS, VB - G)))
    sh.wire((XS, VB), (QX + 38.1, VB))
    sh.flag("#FLG03", (XS + 3.81, VB))
    sh.place("C9", XS + 12.7, VB + 3.81, 0, f=("r", G)); sh.wire((XS + 12.7, VB), "C9.1"); sh.tag("C9.2", 0)
    sh.place("C2", XS + 22.86, VB + 3.81, 0, f=("r", G)); sh.wire((XS + 22.86, VB), "C2.1"); sh.tag("C2.2", 0)
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
    sh.text(17.78, 177.8, "TP4056 charges at 333-364 mA (R5), from 1 to 46 C (TH1 over the cell). USB feeds the system through D5;")
    sh.text(17.78, 181.61, "without USB the cell does, through Q2; on an unplug D7 carries VSYS until R38 has drained VBUS and Q2 is on.")
    sh.text(17.78, 185.42, "CC1/CC2_SENSE let firmware read the source's current offer (dim the backlight on a 500 mA port).")
    return sh


def tp(sh, ref, at, f=(3.81, -H)):
    """Test point ref with its pin on `at` (on a wire), pad above."""
    at = sh.P(at)
    sh.place(ref, at[0], at[1], 0, f=f)
    return at


def power():
    sh = Sheet("power.kicad_sch")
    # ---- 3V3 LDO
    Ux, Uy = 88.9, 45.72
    sh.heading(17.78, 25.4, "3V3 regulator (ME6211)")
    sh.place("U3", Ux, Uy, 0, f=(0, -10.16))
    vin = (45.72, Uy - 2.54)
    sh.wire(vin, "U3.1")
    sh.rail("VSYS", vin)
    tp(sh, "TP2", (55.88, Uy - 2.54))
    sh.place("C5", 68.58, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.wire((68.58, Uy - 2.54), "C5.1"); sh.tag("C5.2", 0)
    end = sh.wire("U3.3", (Ux - 12.7, Uy), (Ux - 12.7, Uy + 12.7))
    sh.label("PWR_EN", end, "down")
    sh.nc("U3.4")
    sh.tag("U3.2", 0)
    sh.place("C6", 104.14, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.place("C7", 114.3, Uy - 2.54 + 3.81, 0, f=("r", G))
    sh.place("JP1", 129.54, Uy - 2.54, 0)
    sh.wire("U3.5", "JP1.1")
    for c in ("C6", "C7"):
        sh.wire((sh.sym[c]["x"], Uy - 2.54), f"{c}.1"); sh.tag(f"{c}.2", 0)
    sh.wire("JP1.2", (160.02, Uy - 2.54))
    sh.flag("#FLG07", (139.7, Uy - 2.54))
    tp(sh, "TP4", (147.32, Uy - 2.54))
    sh.rail("+3V3", (160.02, Uy - 2.54))
    sh.text(119.38, Uy + 10.16, "JP1: cut at bring-up to check the")
    sh.text(119.38, Uy + 13.97, "regulator alone (3V3_LDO)")
    # ---- power latch: button, MCU hold, USB, wired-OR into the LDO's CE
    sh.heading(17.78, 72.39, "Power latch (button, MCU hold, USB)")
    Dx, Dy = 76.2, 88.9
    sh.place("D2", Dx, Dy, 0, f=(-6.35, -6.35))
    raw = (20.32, Dy)
    sh.wire(raw, "D2.1")
    sh.label("PWR_BTN_RAW", raw, "left", hook="up")
    sh.place("R13", 30.48, Dy + 3.81, 0, f=("r", G))
    sh.wire((30.48, Dy), "R13.1"); sh.tag("R13.2", 0)
    hold = (99.06, Dy)
    sh.wire("D2.2", hold)
    sh.label("PWR_HOLD", hold, "right")
    sh.place("R42", 88.9, Dy + 3.81, 0, f=("r", G))
    sh.wire((88.9, Dy), "R42.1"); sh.tag("R42.2", 0)
    Ye = 104.14
    sh.wire("D2.3", (Dx, Ye), (118.11, Ye))
    sh.wire((Dx, Ye), (Dx - 7.62, Ye))
    tp(sh, "TP6", (Dx - 7.62, Ye), f=(-3.81, -H))
    sh.label("PWR_EN", sh.wire((Dx, Ye), (Dx, Ye + 7.62)), "down")
    sh.place("R15", 91.44, Ye + 3.81, 0, f=("r", G))
    sh.place("C47", 101.6, Ye + 3.81, 0, f=("r", G))
    for r in ("R15", "C47"):
        sh.wire((sh.sym[r]["x"], Ye), f"{r}.1"); sh.tag(f"{r}.2", 0)
    sh.place("D3", 121.92, Ye, 0)
    sh.place("R14", 140.97, Ye, 270)
    sh.wire("D3.2", "R14.2")
    v = sh.stub("R14.1", 12.7)
    tp(sh, "TP1", (148.59, Ye))
    sh.rail("VBUS", v)
    # ---- button sense: Q1 lets the MCU read the power button without seeing VSYS
    Qy = 134.62
    sh.heading(17.78, 121.92, "Button sense")
    sh.place("Q1", 50.8, Qy, 0, f=("r", G))
    sh.wire((40.64, Dy), (40.64, Qy), "Q1.1")
    sh.tag("Q1.2", 0)
    sh.place("R16", 53.34, Qy - 8.89, 180, f=("l", 0))
    sh.tag("R16.2", 0)
    sh.wire("Q1.3", (58.42, Qy - 5.08))
    sh.label("PWR_BTN_N", (58.42, Qy - 5.08), "right")
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
    sh.label("VBAT_SENSE", (bx + 20.32, by + 3.81), "right")
    # ---- probe points for the signals on other sheets, and ground pads
    sh.heading(195.58, 124.46, "Test points")
    for i, (ref, net) in enumerate((("TP7", "LS_GATE"), ("TP8", "AMP_SHDN"))):
        a = tp(sh, ref, (200.66, 137.16 + i * 10.16))
        sh.label(net, sh.wire(a, (205.74, a[1])), "right")
    for i, ref in enumerate(("TP5", "TP15", "TP16")):
        a = tp(sh, ref, (241.3 + i * 12.7, 137.16))
        sh.gnd(sh.wire(a, (a[0], a[1] + G)))
    sh.text(17.78, 177.8, "Power: the button (seat 1's Y contact), the MCU (PWR_HOLD) or USB raise PWR_EN. Releasing it with")
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
    for j, tpr, side in ((14, "TP14", -3.81), (13, "TP13", 3.81)):
        ly = lane[j][1]
        sh.label(sh.parts["J3"]["pins"][str(j)], sh.wire(lane[j], (66.04, ly)), "right")
        tp(sh, tpr, (58.42, ly), f=(side, -H))
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
    hook = (Ux - 66.04, ry)
    sh.wire(hook, "R60.1")
    sh.label("BL_PWM", hook, "left", hook="up")
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
    sh.text(17.78, 182.88, "cathode pin (U6 half, Q5/Q6, 2.2 ohm R52/R53). GPIO44 PWM, filtered by R60/R61/C50, sets BL_REF: 0.124 V (56 mA a pin)")
    sh.text(17.78, 186.69, "at 100%. Stability: sim/backlight.py. JP2 and JP4: cut to measure the LED or panel logic current.")
    return sh


def mcu():
    sh = Sheet("mcu.kicad_sch")
    Ux, Uy = 140.97, 96.52
    sh.place("U4", Ux, Uy, 0, f=(0, 0))
    pins = sh.parts["U4"]["pins"]
    import kisym
    geo = kisym.placed("MCU_RaspberryPi:RP2350B", Ux, Uy, 0)
    # right side: GPIO0-39 straight out to their labels
    for num, g in geo.items():
        if g["out"] != "right" or (pins[num] or "").startswith("REV"):
            continue
        net = pins[num]
        if net is None:
            sh.nc(f"U4.{num}")
        else:
            sh.tag(f"U4.{num}", 5.08)
    # board revision straps, wired: GPIO22-25 to a 1k each (a 0 bit) or nothing (a 1)
    sh.heading(190.5, 93.98, "Board revision straps")
    for i in range(4):
        r = f"R{48 + i}"
        num = {0: "22", 1: "23", 2: "25", 3: "26"}[i]
        x = 223.52 - i * 10.16
        y = sh.P(f"U4.{num}")[1]
        sh.place(r, x, y + 3.81, 0, f="r")
        sh.wire(f"U4.{num}", (x, y), f"{r}.1")
        sh.tag(f"{r}.2", 0)
    sh.text(190.5, 135.89, "A 1k to GND makes a bit 0; a bit left unfitted")
    sh.text(190.5, 139.7, "reads 1 through the pin's pull-up (GPIO22-25).")
    sh.text(190.5, 143.51, "REV = 1: R48 not fitted.")
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
        sh.label(pins[num], sh.stub(f"U4.{num}", G), "up", hook="vertical")
    sh.wire("U4.62", (Ux - 2.54, Uy + 60.96), (Ux, Uy + 60.96))
    sh.wire("U4.81", (Ux, Uy + 60.96))
    sh.gnd((Ux, Uy + 60.96))
    # left side
    L = Ux - 25.4
    sh.label("RUN", sh.wire("U4.35", (Ux - 35.56, Uy - 33.02)), "left", hook="up")
    sh.tag("U4.66"); sh.tag("U4.67")
    for n in ("71", "72", "74", "73", "70"):
        sh.nc(f"U4.{n}")
    qs = (Ux - 66.04, Uy - 15.24)
    sh.wire("U4.75", qs)
    sh.label("QSPI_SS", qs, "left", hook="up")
    sh.place("R23", Ux - 50.8, Uy - 15.24 - 3.81, 0, f=("r", -H))
    sh.tag("R23.1", 0)
    sh.tag("U4.30"); sh.tag("U4.31")
    sh.label("SWCLK", sh.wire("U4.33", (Ux - 38.1, Uy + 22.86)), "left", hook="up")
    sh.label("SWDIO", sh.wire("U4.34", (Ux - 38.1, Uy + 25.4)), "left", hook="down")
    # GPIO40-47 (the ADC pins) fan out into 5.08 mm lanes, each with its label above
    adc = ["49", "52", "53", "54", "55", "56", "57", "58"]
    for k, num in enumerate(adc):
        y = Uy + 33.02 + 2.54 * k
        lane = Uy + 33.02 + 5.08 * k
        end = (Ux - 63.5, lane)
        if k == 0:
            sh.wire(f"U4.{num}", end)
        else:
            rx = L - 2.54 * (9 - k)
            sh.wire(f"U4.{num}", (rx, y), (rx, lane), end)
        sh.label(pins[num], end, "left", hook="up")
    # ---- USB series resistors
    sh.heading(20.32, 43.18, "USB series resistors")
    for i, (r, glob) in enumerate((("R22", "USB_DM"), ("R21", "USB_DP"))):
        y = 55.88 + i * 12.7
        sh.place(r, 45.72, y, 90)
        a = (25.4, y)
        sh.wire(a, f"{r}.1")
        sh.label(glob, a, "left", hook="up")
        sh.label(sh.parts[r]["pins"]["2"], sh.stub(f"{r}.2", 5.08), "right")
    # ---- crystal
    yc = 99.06
    sh.heading(20.32, 83.82, "12 MHz crystal")
    sh.place("X1", 48.26, yc, 0, f=(0, -10.16))
    sh.place("C27", 35.56, yc + 3.81, 0, f=("l", G))
    sh.place("C28", 58.42, yc + 3.81, 0, f=("r", G))
    sh.place("R20", 68.58, yc, 270)
    xin = (25.4, yc)
    sh.wire(xin, "X1.1")
    sh.label("XIN", xin, "left")
    sh.wire("X1.3", "R20.2")
    sh.label("XOUT", sh.stub("R20.1", 5.08), "right")
    sh.wire((35.56, yc), "C27.1"); sh.tag("C27.2", 0)
    sh.wire((58.42, yc), "C28.1"); sh.tag("C28.2", 0)
    sh.wire("X1.2", (48.26, yc + 7.62)); sh.gnd((48.26, yc + 7.62))
    sh.text(198.12, 20.32, "Core from Raspberry Pi's RP2350B minimal design (MIT),")
    sh.text(198.12, 24.13, "fitted with an RP2354B: its 2 MB flash is inside the package,")
    sh.text(198.12, 27.94, "so only QSPI_SS (the BOOTSEL strap, R23 pull-up) is wired outside.")
    return sh


def mcu_power():
    sh = Sheet("mcu_power.kicad_sch")

    def cap_row(x0, y0, caps, rail, title):
        """Caps side by side between a supply wire on top and a ground wire below."""
        sh.heading(x0 - 5.08, y0 - 12.7, title)
        xs = [x0 + 10.16 * i for i in range(len(caps))]
        sh.rail(rail, (x0 - 5.08, y0))
        sh.wire((x0 - 5.08, y0), (xs[-1], y0))
        for x, c in zip(xs, caps):
            sh.place(c, x, y0 + 6.35, 0, f=("r", 0))
            sh.wire((x, y0), f"{c}.1")
            sh.wire(f"{c}.2", (x, y0 + 12.7))
        sh.wire((xs[0], y0 + 12.7), (xs[-1], y0 + 12.7))
        sh.gnd((xs[len(xs) // 2], y0 + 12.7))
        return xs

    cap_row(30.48, 45.72, ["C10", "C11", "C12", "C13", "C14", "C15", "C16", "C17", "C18", "C19", "C23"], "+3V3",
            "+3V3 decoupling: IOVDD, ADC_AVDD, USB_OTP_VDD, QSPI_IOVDD (100 nF each), bulk C23")
    cap_row(30.48, 88.9, ["C20", "C21", "C22"], "+1V1", "DVDD decoupling (+1V1)")
    # ---- the core regulator: VREG_AVDD filter, VREG_VIN cap, inductor and output cap
    sh.heading(83.82, 76.2, "Core 1.1 V switcher: input, AVDD filter, inductor, output")
    y = 88.9
    sh.rail("+3V3", (88.9, y))
    sh.place("C24", 96.52, y + 3.81, 0, f=("r", G))
    sh.wire((88.9, y), (106.68, y)); sh.wire((96.52, y), "C24.1"); sh.tag("C24.2", 0)
    sh.text(91.44, y + 13.97, "VREG_VIN")
    sh.place("R19", 111.76, y, 90)
    sh.wire((106.68, y), "R19.1")
    sh.place("C26", 124.46, y + 3.81, 0, f=("r", G))
    sh.wire("R19.2", (134.62, y))
    sh.wire((124.46, y), "C26.1"); sh.tag("C26.2", 0)
    sh.flag("#FLG06", (119.38, y))
    sh.label("VREG_AVDD", (134.62, y), "right")
    yl = 121.92
    lx = (88.9, yl)
    sh.place("L2", 101.6, yl, 270, f=(0, 0))
    sh.wire(lx, "L2.2")
    sh.label("VREG_LX", lx, "left", hook="up")
    sh.wire("L2.1", (147.32, yl))
    sh.place("C25", 116.84, yl + 3.81, 0, f=("r", G))
    sh.wire((116.84, yl), "C25.1"); sh.tag("C25.2", 0)
    sh.flag("#FLG05", (127.0, yl))
    tp(sh, "TP9", (137.16, yl))
    sh.rail("+1V1", (147.32, yl))
    sh.text(17.78, 177.8, "RP2354B supply, as in Raspberry Pi's minimal design: 100 nF at every IOVDD, DVDD, ADC, OTP and QSPI pin,")
    sh.text(17.78, 181.61, "4.7 uF at the regulator's input and output, and L2 (a polarity-marked 3.3 uH inductor, fit as drawn).")
    return sh


# ---- root sheet: the sheet symbols, what each sheet holds, mechanical parts
SHEET_NOTES = {
    "USB_Charger": ("USB-C receptacle + CC pull-downs", "ESD (SRV05-4), VBUS input",
                    "charger (TP4056), load sharing", "battery connector J2"),
    "Power": ("3V3 regulator (ME6211)", "power latch (button, MCU, USB)",
              "button sense, cell voltage sense", "test points"),
    "MCU": ("RP2354B (2 MB flash inside)", "USB series resistors",
            "12 MHz crystal", "board revision straps"),
    "MCU_Power": ("+3V3 and DVDD decoupling", "core 1.1 V switcher:",
                  "AVDD filter, inductor, output", ""),
    "Debug": ("SWD connector J5", "UART connector J6", "debug pads J7",
              "BOOTSEL and RESET buttons"),
    "Display": ("panel connector J3 (50-pin ZIF)", "SPI series resistors, panel supply",
                "backlight reference", "backlight current sinks ch1, ch2"),
    "Audio": ("PWM low-pass filter, DC block", "amplifier (FM8002A) and supply",
              "shutdown control", "speaker pads J4"),
    "Buttons": ("seat 1 contacts (P1 Y is the", "power button), seat 2 contacts",
                "16 membrane contacts,", "one GPIO each"),
}


def SHEET_BOX(i):
    """x, y, width, height of the i-th sheet symbol on the root sheet."""
    return (20.32 + (i % 4) * 66.04, 50.8 + (i // 4) * 50.8, 58.42, 33.02)


def root():
    import design
    sh = Sheet("battler.kicad_sch")
    sh.text(20.32, 20.32, "Two-player handheld: reference design (REF-A)", size=2.0)
    sh.text(20.32, 27.94, "RP2354B, 3.5in 320x480 IPS (ILI9488, SPI), 16 membrane contacts under DS Lite silicone,")
    sh.text(20.32, 31.75, "PWM audio into an FM8002A, one Li-ion cell on a TP4056 charger with load sharing, ME6211 3V3.")
    sh.text(20.32, 35.56, "Every sheet is generated from board/design.py by tools/build_sch.py. Edit design.py, not the sheets.")
    for i, (f, name) in enumerate(design.SHEETS):
        x, y, w, h = SHEET_BOX(i)
        sh.heading(x + 1.27, y - 3.81, name)
        sh.text(x + 2.54, y + 5.08, f, size=1.0)
        for j, line in enumerate(SHEET_NOTES[name]):
            if line:
                sh.text(x + 2.54, y + 11.43 + j * 4.45, line, size=1.0)
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
