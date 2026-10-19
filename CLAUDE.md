## ZERO-DRIFT RULE: shared display & input semantics (MANDATORY)

The pico firmware and the web client MUST be indistinguishable in everything
the player sees and does during a battle. This is enforced structurally:

- `crates/mega_blastoise_core/src/oled_ctl.rs` — the ONLY place that decides what a
  screen shows (`OledController` + `oled_cmds_for_event`).
- `crates/mega_blastoise_core/src/choice_collect.rs` — the ONLY place that decides
  how input behaves (`ChoiceCollector`: prompts, accept/reject + messages,
  waiting screen, tap/type-to-unready, the 1 s both-ready grace, invalid
  flashes, long-press detail views, typed-line grammar).

Platforms (mega_blastoise_fw, mega_blastoise_web) may contain ONLY raw IO:
press classification (tap vs 500 ms hold), typed-line transport, rendering
`OledCmd`s, printing collector `Effect` lines, and a monotonic-ms clock for
`tick()`. If a change makes you write battle UX behavior in fw- or web-only
code, STOP — put it in core and drive it from both platforms. When behavior
between targets conflicts, the pico is the source of truth.

---

## Project

A two-player handheld Pokemon battler: one shared screen, a DS Lite D-pad, A/B/X/Y,
Start and Select at each player's end. Gen 1 and Gen 3 battle engines in `crates/`
(`no_std`), the same core driving the firmware and the browser build.

- **Firmware** (`crates/mega_blastoise_fw`) targets the stripboard v2 board (Pico 2,
  RP2350, ST7796 panel): a plain `cargo build` there builds it.
- **Browser build** (`crates/mega_blastoise_web`, its own wasm workspace) ships to GitHub
  Pages under `/battler/`; `site/` is the Pages front page.
- **Hardware** is indexed in `hardware/README.md`. `hardware/prod_board` and
  `hardware/prod_case` are Alex's own learning project: read them, and change them only
  when he asks. `hardware/ref_design` is Claude's reference design: its KiCad files are
  generated, so edit `board/design.py`, `board/placement.py` or `case/case.scad` and run
  its README's pipeline.
- **The gate** before committing code: `scripts/check.sh` (host tests, firmware check,
  wasm build, headless UI probe).
- The v1 device (RP2040, two OLEDs) is gone from the tree: tag `v1-archive`, notes in
  `docs/archive/v1/`.
