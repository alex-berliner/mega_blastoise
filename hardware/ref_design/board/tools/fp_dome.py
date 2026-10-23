"""The Start/Select contact under a 4 mm metal snap dome (2026-10-21, row 4 of Alex's list):
HYP 600-415S-260 (LCSC C256258), round, 4.0 mm, 0.20 mm high, 260 gf, hand-placed under a
6 mm Kapton dot. Written into lib/battler.pretty as Dome_4mm_Round.

    python3 tools/fp_dome.py

- pad 1, the centre: 1.6 mm round, closed by the dome's apex (the signal)
- pad 2, the ring: 3.4 to 4.6 mm across (r 1.7 to 2.3), under the dome's rim at r 1.95-2.0
  with +-0.3 mm of hand-placement error (GND, or VSYS on the power contacts)
- a 0.5 mm unplated vent at r 1.25 in the 0.9 mm gap, 0.2 mm from both pads: a dome sealed
  under tape otherwise pumps air and feels mushy
- one 4.8 mm mask opening over all of it; courtyard 5.0 mm round
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
NAME = "Dome_4mm_Round"

TEXT = f'''(footprint "{NAME}"
	(version 20260206)
	(generator "fp_dome.py")
	(layer "F.Cu")
	(descr "Contact under a 4.0 mm round metal snap dome (HYP 600-415S-260): 1.6 mm centre pad, 3.4-4.6 mm ring, 0.5 mm vent. The dome is hand-placed, not a placed part.")
	(tags "metal dome snap dome tact contact")
	(property "Reference" "SW**" (at 0 -3.2 0) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))
	(property "Value" "{NAME}" (at 0 3.2 0) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))
	(property "Datasheet" "https://datasheet.lcsc.com/datasheet/pdf/5c1bc660c97aa5f5ab86d1c45a623c9a.pdf?productCode=C256258" (at 0 0 0) (layer "F.Fab") (hide yes) (effects (font (size 1.27 1.27))))
	(property "Description" "4 mm metal dome contact" (at 0 0 0) (layer "F.Fab") (hide yes) (effects (font (size 1.27 1.27))))
	(attr smd exclude_from_pos_files exclude_from_bom allow_soldermask_bridges)
	(fp_circle (center 0 0) (end 2.4 0) (stroke (width 0) (type solid)) (fill yes) (layer "F.Mask"))
	(fp_circle (center 0 0) (end 2.5 0) (stroke (width 0.05) (type solid)) (fill no) (layer "F.CrtYd"))
	(fp_circle (center 0 0) (end 2.0 0) (stroke (width 0.1) (type solid)) (fill no) (layer "F.Fab"))
	(pad "1" smd circle (at 0 0) (size 1.6 1.6) (layers "F.Cu" "F.Mask"))
	(pad "2" smd custom (at 2.0 0) (size 0.6 0.6) (layers "F.Cu" "F.Mask")
		(options (clearance outline) (anchor circle))
		(primitives (gr_circle (center -2.0 0) (end 0 0) (width 0.6) (fill no))))
	(pad "" np_thru_hole circle (at 0 1.25) (size 0.5 0.5) (drill 0.5) (layers "*.Cu" "*.Mask"))
)
'''

if __name__ == "__main__":
    path = os.path.join(LIB, NAME + ".kicad_mod")
    open(path, "w").write(TEXT)
    import pcbnew
    io = pcbnew.PCB_IO_MGR.FindPlugin(pcbnew.PCB_IO_MGR.KICAD_SEXP)
    fp = io.FootprintLoad(LIB, NAME)
    print(NAME, [(p.GetNumber(), p.GetShape()) for p in fp.Pads()])
    io.FootprintSave(LIB, fp)
    print("wrote", path)
