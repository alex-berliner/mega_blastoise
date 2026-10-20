"""Bring the routed board in line with a rebuilt schematic, without rebuilding it.

    python3 tools/sync_pcb.py            # from hardware/ref_design/board/
    python3 tools/sync_pcb.py --check    # report what would change, write nothing

What KiCad's "Update PCB from Schematic" does for a schematic whose circuit is
unchanged: each footprint relinked to its symbol (the symbol's UUID path), its
fields (LCSC, MPN, Manufacturer, BOM, Datasheet, Description) copied, a net
KiCad now names differently renamed (a net that moved onto one sheet becomes
/Sheet/NAME), and a no-connect pad put on its pin's own unconnected-(...) net
(netlist.pad_nets). The copper is untouched. Refuses if the connectivity differs
(a net split or joined, a pad on another net): that needs tools/build_pcb.py.

Then run tools/check_pcb.py (DRC with schematic parity).
"""
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
sys.path.insert(0, HERE)

import netlist  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
FIELDS = ("LCSC", "MPN", "Manufacturer", "BOM", "Datasheet", "Description")


def main(path=PCB):
    comps, nets, pins = netlist.read(pins=True)
    board = pcbnew.LoadBoard(path)
    fps = {fp.GetReference(): fp for fp in board.GetFootprints()}
    if set(fps) != set(comps):
        sys.exit(f"parts differ: board only {sorted(set(fps) - set(comps))}, "
                 f"schematic only {sorted(set(comps) - set(fps))}: rebuild with tools/build_pcb.py")
    rename, connect, relinked, fields = {}, [], 0, 0
    for ref, fp in fps.items():
        c = comps[ref]
        if fp.GetPath().AsString() != c["path"]:
            fp.SetPath(pcbnew.KIID_PATH(c["path"]))
            relinked += 1
        for key in FIELDS:
            if key in c["fields"] and (not fp.HasField(key) or fp.GetFieldText(key) != c["fields"][key]):
                fp.SetField(key, c["fields"][key])
                fields += 1
        pads = list(fp.Pads())
        for pad, new in zip(pads, netlist.pad_nets(ref, [p.GetNumber() for p in pads], nets, pins)):
            have = pad.GetNetname()
            if have == new:
                continue
            if not have and new.startswith("unconnected-("):
                connect.append((pad, new, pins[(ref, pad.GetNumber())]))    # a no-connect pin's own net
            elif not have or not new or rename.setdefault(have, new) != new:
                sys.exit(f"{ref} pad {pad.GetNumber()}: board net {have!r}, schematic {new!r}: "
                         "the circuit changed, rebuild with tools/build_pcb.py")
    # a rename must be one board net to one schematic net, and not onto a net the board has
    if len(set(rename.values())) != len(rename) or any(board.FindNet(n) for n in rename.values()):
        sys.exit(f"net renames don't map one to one: {rename}")
    for old, new in sorted(rename.items()):
        board.FindNet(old).SetNetname(new)
    for pad, name, (function, pintype) in connect:
        ni = board.FindNet(name)
        if not ni:
            ni = pcbnew.NETINFO_ITEM(board, name)
            board.Add(ni)
        pad.SetNet(ni)
        pad.SetPinFunction(function)
        pad.SetPinType(pintype)
    print(f"{relinked} footprints relinked, {fields} fields updated, {len(connect)} no-connect pads on their "
          "unconnected-(...) nets, nets renamed: "
          + (", ".join(f"{a} -> {b}" for a, b in sorted(rename.items())) or "none"))
    if "--check" in sys.argv:
        return
    pcbnew.SaveBoard(path, board)
    print("wrote", path)


if __name__ == "__main__":
    main(*[a for a in sys.argv[1:] if not a.startswith("--")])
