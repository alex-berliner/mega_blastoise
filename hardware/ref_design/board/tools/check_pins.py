#!/usr/bin/env python3
"""The datasheet gate: every assembled part's pins against its datasheet review.

    python3 tools/check_pins.py

board/pin_review/<part>.json holds one review per part (or per group of identical parts):
the datasheet it was checked against (URL, revision), and for every pin the datasheet's
name, type and requirement, a page/table citation and a quote, the net each ref had on that
pin when it was reviewed, and whether that was right ("ok") or a known issue ("issue"; it
passes only with a "waiver" saying why it is accepted). passives.json does the same for the
resistors and capacitors: value, LCSC number and voltage rating against the net.

It fails when:
  - an assembled part has no review, or has a pin the review doesn't cover;
  - a reviewed pin's net in design.py is not the net it was reviewed with (the wiring changed
    since the datasheet was read: review that pin again and update its entry);
  - a review entry has ok false and no waiver;
  - a resistor or capacitor's value or LCSC number differs from its review.
The reviews were built by reading each part's current datasheet (2026-10-05); the files
record where each fact came from, so a re-review starts from them.
"""
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, BOARD)
import design  # noqa: E402

REVIEW = os.path.join(BOARD, "pin_review")


def main():
    parts = {p["ref"]: p for p in design.PARTS if not p["ref"].startswith("#")}
    problems, waived = [], []
    reviewed = {}                                   # ref -> {pin: entry}
    for f in sorted(glob.glob(os.path.join(REVIEW, "*.json"))):
        if f.endswith("passives.json"):
            continue
        r = json.load(open(f))
        for e in r["pins"]:
            for ref, net in e["nets"].items():
                reviewed.setdefault(ref, {})[str(e["pin"])] = (e, net, os.path.basename(f))

    for ref, p in sorted(parts.items()):
        lib = p["lib"]
        if not p["bom"] or lib in ("Device:R", "Device:C"):
            continue                                # unassembled, or a passive (passives.json)
        if ref not in reviewed:
            problems.append(f"{ref} ({p['value']}): no datasheet review in pin_review/")
            continue
        rv = reviewed[ref]
        for pin, net in p["pins"].items():
            if pin not in rv:
                problems.append(f"{ref} pin {pin} ({net}): not in its review")
                continue
            e, rnet, src = rv[pin]
            if (net or None) != (rnet or None):
                problems.append(f"{ref} pin {pin}: now {net}, reviewed as {rnet} ({src}): review it again")
            if not e.get("ok", False):
                if e.get("waiver"):
                    waived.append(f"{ref} pin {pin}: {e['issue']}  WAIVED: {e['waiver']}")
                else:
                    problems.append(f"{ref} pin {pin}: {e.get('issue') or 'marked not ok'} ({src})")

    pas = json.load(open(os.path.join(REVIEW, "passives.json")))
    seen = set()
    for e in pas["passives"]:
        ref = e["ref"]
        seen.add(ref)
        p = parts.get(ref)
        if p is None:
            continue                                # removed since the review: harmless
        if p["bom"] and (p["value"] != e["value"] or p["lcsc"] != e["lcsc"]):
            problems.append(f"{ref}: now {p['value']} {p['lcsc']}, reviewed as {e['value']} {e['lcsc']}: review it again")
        if not e.get("ok", False):
            (waived if e.get("waiver") else problems).append(
                f"{ref}: {e.get('issue')}" + (f"  WAIVED: {e['waiver']}" if e.get("waiver") else ""))
    for ref, p in sorted(parts.items()):
        if p["bom"] and p["lib"] in ("Device:R", "Device:C") and ref not in seen:
            problems.append(f"{ref} ({p['value']} {p['lcsc']}): not in passives.json")

    for w in waived:
        print("  waived:", w)
    for pr in problems:
        print("  PROBLEM:", pr)
    n = sum(len(v) for v in reviewed.values())
    print(f"{len(reviewed)} parts, {n} reviewed pins, {len(seen)} passives; {len(waived)} waived")
    print("RESULT:", "PASS" if not problems else f"FAIL ({len(problems)})")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
