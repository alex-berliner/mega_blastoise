# DS Lite research packet

Everything we found about how the Nintendo DS Lite (USG-001) is built and why its bottom half
looks and feels the way it does, gathered so our case can feel like holding one.

- `reports/DS Lite construction dossier.md`: the dossier. Start here. It ends with
  "Measure your own unit" (scale and caliper checks on a genuine DS Lite) and
  "What this means for our case" (recommendations against `ref_design/case/case.scad`).
- `research_notes/DS Lite construction dossier/`: the raw notes behind it, one file per topic,
  every number with its source link and confidence label:
  - `exterior_geometry.md`: outline, thickness, edge profile, face and edge features
  - `materials_finish.md`: plastic, two-shot clear-over-colour gloss, markings, finishing prints
  - `controls_tactile.md`: D-pad, ABXY, START/SELECT, L/R, power and volume, clearances
  - `internals_mass.md`: parts, screws, stack-up, hinge, mass and balance
  - `design_ergonomics.md`: Nintendo's design intent, reviews, how it is held
  - `primary_sources.md`: patents, FCC filing, scans, CAD models and their licences

No genuine shell has ever been scanned or measured in public: shell numbers come from a scan of an
aftermarket shell, Nintendo's design patent, a scanned genuine mainboard and replica models. The
dossier labels which is which. Results of the owner's own measurements belong in the dossier's
tables once taken.

Downloads (scans, FCC photos, manuals) were kept out of the repo; the notes link every source.

## Membranes and the DS Lite research

The membrane decision is documented in [hardware/membrane/README.md](../membrane/README.md):
the donors surveyed (SNES, DS Lite, others), the DS Lite geometry measured from a breakout board's
gerbers, the contact pattern, licence traps in other projects' footprints, and an open question:
fine gold combs need ENIG or hard gold, while carbon ink (more durable) needs a coarser pattern.

[hardware/ds_lite/](README.md) is a research packet on how the DS Lite is built and
why it feels as it does, every number sourced and labelled by how it was obtained. Its key points
for this project: density (a DS Lite works out to about 1.03 g/cm3, so about 160 g for a case this
size), a 13.5 mm lower half, rounded edges, and the seam on the face. Its recommendations are
inputs, not decisions. Alex owns a genuine DS Lite for scale and caliper checks; the dossier's
"Measure your own unit" section lists them.
