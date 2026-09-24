"""Land patterns for the copied RP2350B core's passives, taken from Raspberry Pi's
minimal design (RP-010329-CA, MIT licence) into lib/battler.pretty.

    RP2350B_REF=/path/RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb python3 tools/lib_from_ref.py

The core's tracks and small zones were drawn around these pads: the VREG_LX
copper runs between the two pads of C24 and of C25, and the 3V3 pour under
the chip keeps its shape only with them. KiCad's own 0402 pads are larger and
sit closer together, so the copied copper no longer fits between them.
"""
import os
import subprocess
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
C_MODEL = "${KICAD10_3DMODEL_DIR}/Capacitor_SMD.3dshapes/C_0402_1005Metric.step"
R_MODEL = "${KICAD10_3DMODEL_DIR}/Resistor_SMD.3dshapes/R_0402_1005Metric.step"
COPIES = {
    # reference footprint: (name here, what differs from KiCad's own, model)
    "generic_capc1005x50": ("C_0402_1005Metric_RPi", "0.55 x 0.55 mm pads at 0.95 mm pitch", C_MODEL),
    "generic_capc1005x50_wide": ("C_0402_1005Metric_RPi_Wide",
                                 "0.47 x 0.55 mm pads at 1.03 mm pitch, 0.56 mm between them for a track", C_MODEL),
    "generic_resc1005x35": ("R_0402_1005Metric_RPi", "0.55 x 0.55 mm pads at 0.95 mm pitch", R_MODEL),
}


def copy(kind):
    """One footprint per process, and every item fetched before the first Remove():
    after that the bindings hand back bare pointers."""
    name, what, model = COPIES[kind]
    io = pcbnew.GetPluginForPath(LIB)
    ref = pcbnew.LoadBoard(os.environ["RP2350B_REF"])
    fp = next(f for f in ref.GetFootprints()
              if str(f.GetFPID().GetLibItemName()) == kind and not f.IsFlipped())
    print("saving", name, "from", fp.GetReference())
    new = pcbnew.FOOTPRINT(fp)
    new.SetOrientationDegrees(0)
    new.SetPosition(pcbnew.VECTOR2I(0, 0))
    pads = list(new.Pads())
    fields = list(new.GetFields())
    court_old = [g for g in new.GraphicalItems() if g.GetLayer() == pcbnew.F_CrtYd]
    for pad in pads:
        pad.SetNetCode(0)
    # Raspberry Pi's courtyard is the body outline; ours is the pads + 0.15 mm, as KiCad's
    cx = round(max(abs(pcbnew.ToMM(p.GetFPRelativePosition().x)) + pcbnew.ToMM(p.GetSize(pcbnew.F_Cu).x) / 2
                   for p in pads) + 0.15, 2)
    cy = round(max(abs(pcbnew.ToMM(p.GetFPRelativePosition().y)) + pcbnew.ToMM(p.GetSize(pcbnew.F_Cu).y) / 2
                   for p in pads) + 0.15, 2)
    court = pcbnew.PCB_SHAPE(new, pcbnew.SHAPE_T_RECTANGLE)
    court.SetStart(pcbnew.VECTOR2I(pcbnew.FromMM(-cx), pcbnew.FromMM(-cy)))
    court.SetEnd(pcbnew.VECTOR2I(pcbnew.FromMM(cx), pcbnew.FromMM(cy)))
    court.SetLayer(pcbnew.F_CrtYd)
    court.SetWidth(pcbnew.FromMM(0.05))
    new.Add(court)
    court.thisown = False
    new.SetFPID(pcbnew.LIB_ID("battler", name))
    new.SetReference("REF**")
    new.SetValue(name)
    new.SetSheetname("")
    new.SetSheetfile("")
    new.SetFilters("")
    # the instance taken may be one Raspberry Pi marked do-not-populate (their R1 is)
    new.SetDNP(False)
    new.SetExcludedFromBOM(False)
    new.SetExcludedFromPosFiles(False)
    new.SetLibDescription(f"0402 land pattern from Raspberry Pi's RP2350B minimal design ({kind}): {what}")
    new.SetKeywords("capacitor resistor 0402 1005 RP2350")
    for g in court_old:
        new.Remove(g)
    # the part's own fields (Murata numbers, Raspberry Pi's part database) stay behind
    for f in fields:
        if f.GetName() in ("Datasheet", "Description"):
            f.SetText("")
        elif f.GetName() not in ("Reference", "Value", "Footprint"):
            new.Remove(f)
    io.FootprintSave(LIB, new)


def main():
    if not os.environ.get("RP2350B_REF"):
        sys.exit("set RP2350B_REF to Raspberry Pi's RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb")
    for kind, (name, _, model) in COPIES.items():
        subprocess.run([sys.executable, __file__, "--copy", kind], check=True)
        subprocess.run([sys.executable, __file__, "--model", name, model], check=True)


def set_model(name, model):
    fp = pcbnew.FootprintLoad(LIB, name)
    fp.Models().clear()
    m = pcbnew.FP_3DMODEL()
    m.m_Filename = model
    fp.Add3DModel(m)
    pcbnew.FootprintSave(LIB, fp)


if __name__ == "__main__":
    if sys.argv[1:2] == ["--copy"]:
        copy(sys.argv[2])
    elif sys.argv[1:2] == ["--model"]:
        set_model(*sys.argv[2:4])
    else:
        main()
