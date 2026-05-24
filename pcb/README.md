# pcb/ — code-defined PCB design

Source-of-truth for the mega_blastoise board wiring as an atopile
project. Mirrors what's described in `../ELECTRONICS.md` but in a form
that a tool can render into a KiCad schematic, BOM, and netlist.

This is the "PCB version" of the design. The current build path is
stripboard + hand wiring (see memory `feedback_stripboard_preference`);
this directory exists so the connectivity is captured as code and so a
fabbed board is a `ato build` away if/when that path makes sense.

## Tool choice

[atopile](https://atopile.io) — purpose-built, code-first DSL for
hardware, designed around the same parametric / source-as-truth
philosophy as `cad/`. Outputs a KiCad 7+ project.

Alternatives considered:

- **SKiDL** (Python library): more mature but writes netlists, not
  full KiCad projects. Pick this if you want to drive layout in
  Python from existing KiCad libraries.
- **KiCad Python API**: scripts existing KiCad — best for layout
  automation, not for from-scratch design.

## Layout

```
pcb/
├── README.md              (this file)
├── ato.yaml               (atopile project config)
└── elec/src/
    └── mega_blastoise.ato (top-level design)
```

## Build

```bash
# one-time
pip install atopile
cd pcb
ato install               # pull standard part library

# every time
ato build                 # emits KiCad project to build/default/
```

Open `build/default/<project>.kicad_pro` in KiCad to inspect the
schematic and lay out the board.

## What's modelled

- Raspberry Pi Pico (treated as a daughterboard, all 26 usable GPIO
  exposed)
- 2x SSD1306 OLED breakouts on independent I2C buses (GP16/17 and
  GP18/19), each with a 100 nF decoupling cap
- 14x tactile buttons arranged as a 4x4 matrix on GP6..GP13
- WS2812B NeoPixel strip header on GP20 with a 330R series resistor on
  data and a 470 uF bulk cap across V5/GND at the strip entry
- Piezo buzzer on GP21
- Power stage: JST-PH LiPo connector + TP4056 charge module + MT3608
  boost (preset to 5 V) + SPDT slide switch -> V5 rail
- Reset button across RUN -> GND (see memory
  `project_reset_button`)
- Error LED stub on GP22 (see memory `project_error_led`) — needs an
  `LED` part imported before `ato build` will succeed

## Known gaps before fabbing

The `.ato` file uses local `component` stubs for the bigger
parts (Pico, OLEDs, TP4056, MT3608, slide switch, JST, headers,
buttons, buzzer). Each one declares only pins and a footprint
hint. Before `ato build` will produce a fabricable board you'll
need to either:

1. swap each stub for a registry-backed `component` (`ato install`
   then `from ... import ...`), or
2. fill in the `footprint` / `mpn` fields and add KiCad symbols
   under `elec/footprints/`.

The connectivity is fully expressed; the work above is part-library
plumbing, not design work.
