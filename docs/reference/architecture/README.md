# Architecture Docs

This directory contains the architecture documentation for the Mega Blastoise workspace. For build instructions see [README.md](../../../README.md); for software internals see [TECHNICAL.md](../TECHNICAL.md).

## Reading Order

1. [System Overview](01-system-overview.md)
2. [Core Battle Engine Flow](02-core-battle-flow.md)
3. [Firmware Runtime (RP2350)](03-firmware-runtime.md)
4. [Host Runtime and Test Harness](04-host-runtime.md)
5. [Events and Input Contracts](05-events-and-input.md)
6. [Memory and Debugging](06-memory-and-debugging.md)
7. [Design Principles and Extension Guide](07-design-principles.md)
8. [Feature Map: Core vs. Target-Specific](08-feature-map.md)
9. [Single-Screen Redesign](09-single-screen.md) — target design for the next hardware generation
10. [Production Board](../../living/10-production-board.md) — what changes when the device becomes a sellable product

## Quick Links

- Core crate: [`mega_blastoise_core`](../../../crates/mega_blastoise_core/)
- Firmware crate: [`mega_blastoise_fw`](../../../crates/mega_blastoise_fw/)
- Host/test crate: [`mega_blastoise_test`](../../../crates/mega_blastoise_test/)

## Where things live

| Path | What |
|---|---|
| [README.md](../../../README.md) | Build and run instructions |
| [CLAUDE.md](../../../CLAUDE.md) | The zero-drift rule and project rules for Claude Code |
| [docs/reference/architecture/](README.md) | Software architecture pages |
| [docs/archive/architecture/](09-single-screen.md) | Superseded plans: [09](09-single-screen.md) single-screen redesign, [10](../../living/10-production-board.md) production board |
| [docs/reference/TECHNICAL.md](../TECHNICAL.md) | Software internals overview |
| [docs/archive/v1/](../../archive/v1/DESIGN.md) | The v1 device: design, electronics, firmware interactions, original plan |
| [docs/reference/openscad-methodology.md](../openscad-methodology.md) | How the cases are modelled and checked |
| `crates/gen1_battle/`, `crates/gen3_battle/`, `crates/battle_types/` | Battle engines; Gen 1 spec |
| `crates/mega_blastoise_core/` | The shared game: loop, input, screens, renderer, generated data |
| `crates/mega_blastoise_fw/` | Firmware for the stripboard v2 |
| `crates/mega_blastoise_web/` | Browser build; `www/` holds the pages |
| `crates/mega_blastoise_test/` | Host CLI, tests, parity and fuzz suites, screen dumper |
| `battler/` | Git submodule: the upstream engine, now a data source |
| `patches/once_cell/` | A patched dependency for `no_std` targets |
| tools/showdown-parity/ | Node harness around the real Showdown |
| `scripts/` | The gate, UI probe, console, build and publish helpers |
| `firmware/bringup/` | Standalone bring-up crates |
| `site/` | The Pages front page |
| [hardware/README.md](../../../hardware/README.md) | Hardware index |
| [hardware/stripboard_v2/](../../../hardware/stripboard_v2/BUILD.md) | The built board: generator, build guide, assembly stages, models |
| [hardware/ref_design/](../../../hardware/ref_design/README.md) | The reference design: `board/`, `case/`, `docs/`, `datasheets/`, `tools/quotes/`, `viewer/` |
| `hardware/prod_board/`, [hardware/prod_case/](../../../hardware/prod_case/README.md) | Alex's own board and case (read only for others) |
| [hardware/membrane/](../../../hardware/membrane/README.md) | Membrane contact footprints and their generator and checker |
| [hardware/ds_lite/](../../../hardware/ds_lite/README.md) | DS Lite construction research |

## Glossary (software and the game)

Terms used in these documents, alphabetical.

- <a id="g-layers"></a>**2-layer, 4-layer**: How many copper layers a board has. Two are cheapest; REF-A fits on two because each pin faces where its net goes; its first 4-layer version's bare board cost about three times as much. More: [Wikipedia](https://en.wikipedia.org/wiki/Printed_circuit_board).
- <a id="g-3d-model"></a>**3D model**: A part's solid shape, which KiCad shows on the board and `check_fit.py` tests against the case. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#navigating_the_3d_view).
- <a id="g-ability"></a>**Ability**: A Pokemon's built-in passive effect, such as Levitate or Intimidate. Abilities arrived in Gen 3, so Gen 1 has none; gen3_battle models 75 of the 76. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Ability).
- <a id="g-adc"></a>**ADC**: Analogue-to-digital converter: measures a voltage as a number. REF-A reads the cell (ADC7) and the two USB-C CC lines (ADC6, ADC5); readings are relative to 3V3, the ADC's reference. More: [Wikipedia](https://en.wikipedia.org/wiki/Analog-to-digital_converter).
- <a id="g-async-task"></a>**Async task**: A function that can pause while it waits (for a timer, a DMA transfer, a pin) and let other tasks run, all on a single CPU with no operating system. The firmware is a set of Embassy tasks. More: [Embassy book](https://embassy.dev/book/).
- <a id="g-attract-demo"></a>**Attract demo**: The AI-versus-AI battle the device plays across both halves while idle, after 15 s in the lobby.
- <a id="g-battle-engine"></a>**Battle engine**: The crate that holds one generation's rules: `gen1_battle` or `gen3_battle`. It takes each side's choice and writes the turn as log lines; core does everything else.
- <a id="g-battler"></a>**`battler`**: The upstream open-source Rust Pokemon engine the project began on, checked out as a git submodule (Alex's fork). It ran out of heap on the RP2040, so it is now only a data source for core's build script. More: [GitHub](https://github.com/jackson-nestelroad/battler).
- <a id="g-blit"></a>**Blit**: To copy a block of pixels in one go. The web page blits each finished frame onto a canvas. More: [Wikipedia](https://en.wikipedia.org/wiki/Bit_blit).
- <a id="g-boardevent"></a>**`BoardEvent`**: A typed battle event (damage, faint, switch-in, status and so on) parsed from an engine's log.
- <a id="g-bootsel"></a>**BOOTSEL**: The RP2350's boot-select input: holding QSPI_SS low through a reset enters the USB boot loader instead of the firmware. On REF-A that is SW18 under a pinhole, or BOOT to GND on J7. More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-brown-out"></a>**Brown-out detector, supervisor**: A circuit that resets a chip when its supply sags, so it never runs at a voltage too low for its logic or flash. The RP2350's brown-out detector watches only the 1.1 V core, and REF-A has no supervisor on 3V3, which is why the unplug dip and the 3.5 V cutoff matter. More: [Wikipedia](https://en.wikipedia.org/wiki/Power-on_reset).
- <a id="g-build-script"></a>**Build script**: A Rust program (`build.rs`) that Cargo runs before compiling a crate. Core's generates the static data tables and sprites at compile time. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/build-scripts.html).
- <a id="g-button-matrix"></a>**Button matrix**: Buttons wired in rows and columns so a few pins read many buttons, at the cost of ghosting. v1 used one; stripboard v2 and REF-A give every button its own GPIO. More: [Wikipedia](https://en.wikipedia.org/wiki/Keyboard_matrix_circuit).
- <a id="g-ceramic-cap"></a>**Ceramic capacitor**: The common small surface-mount capacitor: cheap and low-loss, but it can ring with a cable's inductance at a live plug-in, and it loses capacitance under DC voltage. More: [Wikipedia](https://en.wikipedia.org/wiki/Ceramic_capacitor).
- <a id="g-choicecollector"></a>**`ChoiceCollector`**: Core's battle-input state machine: prompts, accept and reject, the waiting screen, take-back, the grace window and the typed-line grammar.
- <a id="g-class-ab"></a>**Class AB, class D**: Amplifier types. Class AB runs its output transistors linearly: simple and cheap, turning about half its battery draw into sound at high volume. Class D switches its output and reaches 80-90%. More: [Wikipedia](https://en.wikipedia.org/wiki/Power_amplifier_classes).
- <a id="g-clearance"></a>**Clearance**: The minimum gap between copper features of different nets. REF-A uses 0.15 mm, down to 0.1 mm inside the copied RP2350B core. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#clearance-and-constraint-resolution).
- <a id="g-comb"></a>**Comb**: One half of a membrane contact: two interleaved combs of bare copper (0.15 mm lines, 0.20 mm gaps) that a pill bridges when pressed.
- <a id="g-core"></a>**Core**: `mega_blastoise_core`, the `no_std` crate shared by every platform. It holds the game loop, the input rules, the screens and the renderer; see the zero-drift rule.
- <a id="g-coupon"></a>**Coupon**: A small test piece that checks one thing: printed case coupons in the fit kit, or a bare ENIG board coupon for contact resistance.
- <a id="g-cpsia"></a>**CPSIA**: The US Consumer Product Safety Improvement Act, which requires testing of children's products; marketing to adults avoids it. More: [Wikipedia](https://en.wikipedia.org/wiki/Consumer_Product_Safety_Improvement_Act).
- <a id="g-crate"></a>**Crate**: Rust's unit of code: one library or program. The repository holds the engines, core and one crate per platform. More: [Rust book](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html).
- <a id="g-crit"></a>**Critical hit (crit)**: A hit that does extra damage, at random. The battle log gives the reason for each one, and the fuzzer scripts them on both sides so the comparison is exact. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Critical_hit).
- <a id="g-damage-roll"></a>**Damage roll**: The random factor in each damage calculation (in Gen 3, a multiplier from 85% to 100%). The fuzzer fixes it on both sides so every branch compares exactly. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Damage).
- <a id="g-dc-block"></a>**DC block**: A series capacitor that passes an audio swing but not the steady voltage it rides on. C39 blocks the PWM's half-scale idle level.
- <a id="g-dc-bias"></a>**DC-bias derating**: Ceramic capacitors lose capacitance as the DC voltage across them rises, small high-value parts often by half or more, so a 22 uF part on a 5 V rail may act like much less. More: [Wikipedia](https://en.wikipedia.org/wiki/Ceramic_capacitor).
- <a id="g-db"></a>**Decibel (dB)**: A logarithmic ratio: every 20 dB is a factor of ten in voltage. The PWM carrier coming out 49 dB down means it is about 1/280 of its size. More: [Wikipedia](https://en.wikipedia.org/wiki/Decibel).
- <a id="g-defmt"></a>**defmt**: A compact logging format for embedded Rust: the device sends short codes and the host expands them into text. More: [defmt book](https://defmt.ferrous-systems.com/).
- <a id="g-deviceevent"></a>**`DeviceEvent`**: A press as a platform hands it to core, already classified as a tap or a 500 ms hold.
- <a id="g-devicerunner"></a>**`DeviceRunner` / `DevicePort`**: Core's game loop (attract, lobby, ready sequence, battle, recap), and the trait a platform implements for it. It moved into core on 2026-08-27.
- <a id="g-differential"></a>**Differential testing**: Running the same inputs through two implementations and comparing the outputs; here the engines against Showdown. More: [Wikipedia](https://en.wikipedia.org/wiki/Differential_testing).
- <a id="g-diode-test"></a>**Diode test**: A multimeter mode that pushes a small current through a part and shows the voltage across it; a Schottky reads about 0.2-0.3 V. More: [Wikipedia](https://en.wikipedia.org/wiki/Multimeter).
- <a id="g-dirty-tiles"></a>**Dirty tiles**: Redrawing only what changed: the firmware splits the frame into 16x16 tiles and sends only those whose pixels differ, which keeps SPI traffic down.
- <a id="g-dma"></a>**DMA**: Direct memory access: hardware that moves data (here pixels to the panel's SPI) while the CPU does other work. More: [Wikipedia](https://en.wikipedia.org/wiki/Direct_memory_access).
- <a id="g-embassy"></a>**Embassy**: An async framework for embedded Rust; `embassy-rp` is its layer for Raspberry Pi's chips. It lets the firmware run as cooperating tasks without an operating system. More: [embassy.dev](https://embassy.dev/).
- <a id="g-event-bus"></a>**Event bus**: A hub where code registers for events and is called back when they fire. gen3_battle avoids one: each pipeline stage asks the ability and item code directly. More: [Wikipedia](https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern).
- <a id="g-ev-iv-nature"></a>**EVs, IVs and natures**: Gen 3's hidden stat modifiers: IVs (individual values, fixed per Pokemon), EVs (effort values, earned in training; an EV spread is how they are shared out) and a nature, which raises one stat by 10% and lowers another. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Effort_values).
- <a id="g-exploded"></a>**Exploded view**: A 3D drawing with the parts pulled apart along the assembly direction so each one shows. REF-A's is published on Pages at `/assembly/`. More: [Wikipedia](https://en.wikipedia.org/wiki/Exploded-view_drawing).
- <a id="g-feature"></a>**Feature (Cargo)**: A named build option of a Rust crate. The firmware has `input-debug`, `mem-profile` and `trace`; embassy-rp picks the chip with `rp235xa` or `rp235xb`. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/features.html).
- <a id="g-firmware"></a>**Firmware**: The program stored in the device's flash and run by its microcontroller; here `mega_blastoise_fw`, core plus the RP2350's raw IO. More: [Wikipedia](https://en.wikipedia.org/wiki/Firmware).
- <a id="g-fit-kit"></a>**Fit kit**: Printed board mock, part dummies and case coupons to test real parts before boards are ordered. More: [fit kit README](../../../hardware/ref_design/case/fitkit/README.md).
- <a id="g-framebuffer"></a>**Framebuffer**: A copy of the whole screen image in the MCU's RAM. Not needed here: the panel keeps its own frame memory, so the firmware sends only what changes. More: [Wikipedia](https://en.wikipedia.org/wiki/Framebuffer).
- <a id="g-fuzz"></a>**Fuzz testing**: Comparing the engines with Showdown on seeded random battles (`showdown_fuzz`), with hit, crit and damage roll scripted on both sides. The fuzz pool is the set of moves it draws from; a sweep runs many seeds, 2000 before an engine change is pushed. More: [Wikipedia](https://en.wikipedia.org/wiki/Fuzzing).
- <a id="g-the-gate"></a>**The gate (`check.sh`)**: `scripts/check.sh`, run before every code commit: host tests, a firmware `cargo check`, the WASM build and the headless UI probe. More: [check.sh](../../../scripts/check.sh).
- <a id="g-gate"></a>**Gate, drain and source**: A MOSFET's three terminals. The gate's voltage, measured from the source, sets how well current flows between drain and source; the gate itself draws almost no steady current. More: [Wikipedia](https://en.wikipedia.org/wiki/MOSFET).
- <a id="g-gen"></a>**Gen 1, Gen 3**: Generations of the Pokemon games, each with its own battle rules. Gen 1 is Red, Blue and Yellow (1996 to 1999); Gen 3 is Ruby, Sapphire, Emerald, FireRed and LeafGreen (2002 to 2004). More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Generation).
- <a id="g-ghosting"></a>**Ghosting**: A button matrix reporting a press that didn't happen, when several held buttons close a path through another position. v1's matrix ghosted presses across players. More: [Wikipedia](https://en.wikipedia.org/wiki/Keyboard_matrix_circuit).
- <a id="g-submodule"></a>**Git submodule**: A git repository checked out inside another at a pinned commit; `battler/` is one. More: [Pro Git](https://git-scm.com/book/en/v2/Git-Tools-Submodules).
- <a id="g-github-pages"></a>**GitHub Pages**: GitHub's static web hosting. `.github/workflows/pages.yml` publishes the front page, the game under `/battler/` and the assembly viewers on every push to `main`. More: [GitHub Pages](https://pages.github.com/).
- <a id="g-gpio"></a>**GPIO**: General-purpose input/output: a chip pin that firmware can read or drive. Every REF-A button has its own. More: [Wikipedia](https://en.wikipedia.org/wiki/General-purpose_input/output).
- <a id="g-grace"></a>**Grace window**: The 1 s core waits after both seats have locked in before the turn fires, so a last-moment take-back still counts.
- <a id="g-half"></a>**Half**: One seat's part of the panel; the far half is drawn rotated 180 degrees so both players read their own half upright.
- <a id="g-heap"></a>**Heap**: Memory handed out at run time for data whose size isn't known in advance. `battler` exhausted the RP2040's 128 KB heap; the in-house engines avoid heap allocation on the hot path. More: [Wikipedia](https://en.wikipedia.org/wiki/Memory_management).
- <a id="g-held-item"></a>**Held item**: An item a Pokemon carries into battle, such as Leftovers. gen3_battle models them, but the device's Gen 3 draft doesn't hand them out yet. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Held_item).
- <a id="g-host"></a>**Host**: The developer's computer, as against the device. Host tests, the host CLI and the screen dumper run there with no hardware.
- <a id="g-hot-path"></a>**Hot path**: The code that runs most often, where speed and memory matter most; for the engines, every damage calculation. More: [Wikipedia](https://en.wikipedia.org/wiki/Hot_spot_(computer_programming)).
- <a id="g-hp"></a>**HP**: Hit points: how much damage a Pokemon can take before it faints. Each seat's HP plate shows its own mon's. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/HP).
- <a id="g-hud"></a>**HUD**: Heads-up display: the HP plate and status drawn over the battle scene. More: [Wikipedia](https://en.wikipedia.org/wiki/HUD_(video_games)).
- <a id="g-i2s"></a>**I2S**: A three-wire digital audio bus. The first production plan had an I2S amplifier; REF-A makes audio from a PWM pin instead, which is cheaper. More: [Wikipedia](https://en.wikipedia.org/wiki/I2S).
- <a id="g-injection"></a>**Injection moulding**: Making plastic parts by forcing molten plastic into a steel mould: dear to tool, cheap per part, the usual route for a case at volume. More: [Wikipedia](https://en.wikipedia.org/wiki/Injection_moulding).
- <a id="g-input-buffer"></a>**Input buffer**: The digital input stage behind a GPIO pad. It is turned off on analogue pins, and pins with it on are the ones erratum E9 affects.
- <a id="g-iovdd"></a>**IOVDD, DVDD**: RP2350 supply pin names: IOVDD feeds the 3.3 V I/O and DVDD the 1.1 V digital core. VREG_VIN, VREG_AVDD and VREG_LX are the internal switcher's input, filtered analogue supply and inductor pin. More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-ips"></a>**IPS panel**: In-plane switching LCD: colour and contrast hold from steep angles. Needed because each seat views the shared panel from its own side. More: [Wikipedia](https://en.wikipedia.org/wiki/IPS_panel).
- <a id="g-jig"></a>**Jig**: A fixture that holds a board against test pins, such as pogo pins on J7. More: [Wikipedia](https://en.wikipedia.org/wiki/Jig_(tool)).
- <a id="g-latch"></a>**Latch (soft power, PWR_HOLD)**: REF-A's soft power circuit: pressing seat 1's Start powers the regulator up, the MCU then holds itself on through PWR_HOLD, and dropping PWR_HOLD turns the device off once Start is released. A reset or crash drops the hold.
- <a id="g-ldo"></a>**LDO**: Low-dropout linear regulator: makes a lower, fixed voltage by dropping the difference across a transistor as heat. Simple, quiet and cheap; REF-A's ME6211 makes 3V3 from VSYS. More: [Wikipedia](https://en.wikipedia.org/wiki/Low-dropout_regulator).
- <a id="g-linear-charger"></a>**Linear charger**: A charger that drops the difference between USB's 5 V and the cell as heat in the chip, like an LDO: cheap and simple but warm (about 0.65 W in REF-A's TP4056 while charging). More: [Wikipedia](https://en.wikipedia.org/wiki/Battery_charger).
- <a id="g-memory-map"></a>**Memory map**: The table that tells the linker where flash and RAM sit and how big they are. REF-A's firmware needs one with 2048K of flash. More: [Wikipedia](https://en.wikipedia.org/wiki/Memory_map).
- <a id="g-mit"></a>**MIT licence**: A permissive open-source licence: reuse is allowed if the copyright notice and licence text go along. The copied RP2350B core is under it. More: [Wikipedia](https://en.wikipedia.org/wiki/MIT_License).
- <a id="g-mixer"></a>**Mixer**: Code that adds several sounds into one output stream. None exists yet; when written it goes in core, so the browser sounds like the device.
- <a id="g-mon"></a>**Mon**: Short for a Pokemon, as the code and the UI text use it.
- <a id="g-monotonic"></a>**Monotonic clock**: A clock that only counts up and is never adjusted, used for timing (the 500 ms hold, the 1 s grace). Each platform supplies one. More: [Rust docs](https://doc.rust-lang.org/std/time/struct.Instant.html).
- <a id="g-fet"></a>**MOSFET (FET)**: A transistor whose gate voltage controls the current between drain and source. N-channel ones (2N7002, AO3400A) turn on with the gate above the source; P-channel ones (AO3401A) with the gate below it. More: [Wikipedia](https://en.wikipedia.org/wiki/MOSFET).
- <a id="g-nightly"></a>**Nightly Rust**: Rust's daily build, which allows unstable features. The project pins one in `rust-toolchain.toml` so builds repeat. More: [rustup book](https://rust-lang.github.io/rustup/concepts/channels.html).
- <a id="g-no-std"></a>**`no_std`**: A Rust crate that doesn't use the standard library (no operating system, files or threads), so it runs on a bare microcontroller. Core and the engines are `no_std`. More: [Embedded Rust book](https://docs.rust-embedded.org/book/intro/no-std.html).
- <a id="g-node"></a>**Node**: Node.js, JavaScript outside a browser. The parity harness runs the real Showdown under it. More: [nodejs.org](https://nodejs.org/).
- <a id="g-oledcontroller"></a>**`OledController` / `Screen`**: Core's per-seat screen state machine and its list of screens. The name dates from the two-OLED device.
- <a id="g-on-resistance"></a>**On-resistance**: A switched-on MOSFET's resistance from drain to source, which falls as the gate drive rises. Q2's is at most 85 mOhm with 2.5 V of gate drive. More: [Wikipedia](https://en.wikipedia.org/wiki/Power_MOSFET).
- <a id="g-open-information"></a>**Open information**: The rule that each player sees the rival's active mon, its HP and status; only a locked-in move stays hidden until playback. More: Section 2.
- <a id="g-openocd"></a>**OpenOCD**: An open-source on-chip debugger, an alternative to probe-rs for SWD. More: [GitHub](https://github.com/openocd-org/openocd).
- <a id="g-palette"></a>**Palette**: The short list of colours an image's pixels index into. Each Gen 3 sprite is stored at 4 bits a pixel (16 colours) with its own palette. More: [Wikipedia](https://en.wikipedia.org/wiki/Indexed_color).
- <a id="g-panic"></a>**Panic**: A Rust program stopping on an unrecoverable error. On the Pico 2 a panic lights GP25 and its message survives a reset. More: [Rust book](https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html).
- <a id="g-parity"></a>**Parity tests**: Differential tests of the engines against the real Showdown on fixed, hand-written scenarios (`showdown_parity`). The fuzzer does the same with seeded random ones. More: tools/showdown-parity.
- <a id="g-escape"></a>**Pin escape (fan-out)**: Routing a track out from between a chip's pins to where there is room. REF-A copies Raspberry Pi's escapes along with the core layout.
- <a id="g-pin-review"></a>**Pin review**: REF-A's check of every assembled part's every pin against its current datasheet (`check_pins.py`, results in `board/pin_review/`), a gate since 2026-10-05.
- <a id="g-platform"></a>**Platform**: The firmware, web or host crate: raw IO around core (press classification, frames, log lines, a clock).
- <a id="g-pokeapi"></a>**PokeAPI**: An open Pokemon data project whose sprite repository supplies the Gen 3 (Emerald) art. More: [GitHub](https://github.com/PokeAPI/sprites).
- <a id="g-ppm"></a>**PPM**: A minimal uncompressed image format, easy to write from a host program; the `screens` dumper writes it. More: [Wikipedia](https://en.wikipedia.org/wiki/Netpbm).
- <a id="g-probe-rs"></a>**probe-rs**: A Rust tool that flashes and debugs chips over SWD and shows their RTT and defmt output. More: [probe.rs](https://probe.rs/).
- <a id="g-randbat"></a>**Random battle (randbat)**: Showdown's format in which each player gets a random team drawn from curated sets. The device drafts a fresh team every battle, from the randbat data for Gen 1 and the vendored sets for Gen 3. More: [pkmn randbats](https://pkmn.github.io/randbats/).
- <a id="g-residual"></a>**Residual order**: The fixed order in which end-of-turn effects (weather damage, Leftovers, poison and so on) resolve. gen3_battle follows Showdown's.
- <a id="g-rtt"></a>**RTT**: Real-time transfer: a debug channel through the SWD probe that carries log output at little cost to the running program. More: [SEGGER](https://www.segger.com/products/debug-probes/j-link/technology/about-real-time-transfer/).
- <a id="g-saturation"></a>**Saturation current**: The current at which an inductor's core saturates and its inductance collapses. L2 must handle at least 1.5 A. More: [Wikipedia](https://en.wikipedia.org/wiki/Saturation_(magnetic)).
- <a id="g-schematic-parity"></a>**Schematic parity**: KiCad's check that the board and the schematic agree: the same parts, footprints and nets. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#schematic_parity_drc_checks).
- <a id="g-seat"></a>**Seat**: One player's end of the device: seat 1 and seat 2 (P1 and P2 in code).
- <a id="g-seed"></a>**Seed**: The number that starts a pseudo-random sequence, so a random battle or test replays exactly when given the same seed. More: [Wikipedia](https://en.wikipedia.org/wiki/Random_seed).
- <a id="g-showdown"></a>**Showdown**: Pokemon Showdown, the open-source battle simulator used as the reference. Its source is read before a mechanic is implemented, and it wins every disagreement. More: [GitHub](https://github.com/smogon/pokemon-showdown).
- <a id="g-singles"></a>**Singles battle**: One Pokemon active on each side at a time. Doubles, with two a side, are out of scope. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Single_Battle).
- <a id="g-special-split"></a>**Special split**: Gen 1 has a single Special stat; from Gen 2 on it is split into Special Attack and Special Defense, and gen3_battle models the split. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Stat).
- <a id="g-sprite"></a>**Sprite**: A small picture drawn over the scene, here each Pokemon's. The sprite bob is its idle up-and-down bounce. More: [Wikipedia](https://en.wikipedia.org/wiki/Sprite_(computer_graphics)).
- <a id="g-standard-process"></a>**Standard process**: A fab's cheapest set of limits (smallest hole, track and gap); going below them costs more. REF-A keeps every hole 0.3 mm or larger to stay on JLC's. More: [JLCPCB](https://jlcpcb.com/capabilities/pcb-capabilities).
- <a id="g-strap"></a>**Strap**: A resistor that fixes a pin's level so the chip or the firmware reads it at start-up. QSPI_SS is the BOOTSEL strap; R48 to R51 set REF-A's 4-bit board revision.
- <a id="g-swd"></a>**SWD**: Serial Wire Debug: ARM's two-wire debug port (SWCLK, SWDIO) for loading firmware, pausing the chip and reading its memory. More: [Wikipedia](https://en.wikipedia.org/wiki/JTAG#Serial_Wire_Debug).
- <a id="g-threshold"></a>**Threshold voltage**: The gate-to-source voltage at which a MOSFET starts to conduct. More: [Wikipedia](https://en.wikipedia.org/wiki/Threshold_voltage).
- <a id="g-tn"></a>**TN panel**: Twisted nematic LCD: cheap, but its contrast inverts when viewed from the wrong side, so the far seat would see a negative. More: [Wikipedia](https://en.wikipedia.org/wiki/Twisted_nematic_field_effect).
- <a id="g-tp4056"></a>**TP4056**: A cheap linear Li-ion charger chip that charges to 4.2 V. Stripboard v2 uses a protected TP4056 module; REF-A uses the bare chip with load sharing around it. Its pins: CHRG and STDBY report charging and done, TEMP takes the thermistor, CE enables it.
- <a id="g-trait"></a>**Trait**: A Rust interface: a set of functions a type promises to provide. Each platform implements core's `DevicePort` trait. More: [Rust book](https://doc.rust-lang.org/book/ch10-02-traits.html).
- <a id="g-type"></a>**Type**: Each Pokemon and move has a type (Fire, Water and so on): 15 in Gen 1, 17 in Gen 3. The type chart sets how effective each attacking type is against each defending one; `battle_types` holds the shared list. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Type).
- <a id="g-un383"></a>**UN38.3**: The UN transport test standard for lithium cells; the cell supplier provides the test documents. More: [Wikipedia](https://en.wikipedia.org/wiki/UN38.3).
- <a id="g-usb-cdc"></a>**USB CDC**: The USB class for a serial port; the firmware's console shows up on the computer as one. More: [Wikipedia](https://en.wikipedia.org/wiki/USB_communications_device_class).
- <a id="g-usb-c"></a>**USB-C**: The reversible USB connector. REF-A's J1 carries charging, firmware loading and the console, on USB 2.0 data; its SBU (sideband) pins are left open. More: [Wikipedia](https://en.wikipedia.org/wiki/USB-C).
- <a id="g-validation-plan"></a>**Validation plan**: `validation_plan.md` (2026-10-05): a joint plan from two models' cross-examination, with a 13-item go/no-go gate to pass before boards are ordered. More: [validation_plan.md](../../living/validation_plan.md).
- <a id="g-vendored"></a>**Vendored**: Copied into this repository from an outside source rather than fetched when needed, such as the Gen 3 random battle sets and the membrane footprints.
- <a id="g-vid-pid"></a>**VID and PID**: The vendor and product IDs a USB device reports. The firmware uses placeholders; Raspberry Pi gives out product IDs under its vendor ID for products on its chips. More: [Raspberry Pi usb-pid](https://github.com/raspberrypi/usb-pid).
- <a id="g-wasm"></a>**WASM**: WebAssembly: a compact code format that browsers run at near-native speed. The web build is core compiled to it. More: [webassembly.org](https://webassembly.org/).
- <a id="g-watchdog"></a>**Watchdog**: A hardware timer the firmware must keep restarting; if the firmware hangs, the watchdog resets the chip, which on REF-A turns the device off on battery. More: [Wikipedia](https://en.wikipedia.org/wiki/Watchdog_timer).
- <a id="g-weather"></a>**Weather**: Gen 3's field conditions (rain, sun, sandstorm, hail), which change damage and act at the end of each turn. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Weather).
- <a id="g-workspace"></a>**Workspace (Cargo)**: A set of crates built together. The web build is its own workspace so its WASM settings stay apart. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/workspaces.html).
- <a id="g-ws2812b"></a>**WS2812B**: An RGB LED with its own controller, chained on one data line into addressable strips; v1 showed HP and party on them. More: [Adafruit](https://learn.adafruit.com/adafruit-neopixel-uberguide).
- <a id="g-zero-drift"></a>**Zero-drift rule**: Firmware and web must look and behave identically: all decisions live in core, and platforms do raw IO only. When they disagree, the device is the source of truth. More: Section 5.3.
