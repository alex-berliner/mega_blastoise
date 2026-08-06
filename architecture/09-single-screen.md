# Single-Screen Redesign

Target design for the next generation of the device: one shared color screen
split between two seats, cursor-driven controls, and open information.

This page is the reference every migration session reads first. It records
decisions, not aspirations — where a number appears here it was measured, and
where something is undecided it says so.

Visual spec: [`mega_blastoise_web/www/ui_flow.html`](../mega_blastoise_web/www/ui_flow.html)
(published at `/mega_blastoise/ui_flow.html`). Click any mockup to toggle a
true 240x320 pixel preview.

---

## 1. What changes and what does not

**Changes:** the display stack (one color panel instead of two mono OLEDs), the
input model (cursor and confirm instead of corner-button-equals-corner-move),
the information model (open, with a permanent foe HUD), and the physical
control layout (D-pad plus three buttons per seat).

**Does not change:** the battle engine integration, `BoardEvent` /
`BoardEffects`, `battle_runner`, `data_store`, `randbat`, and the choice-string
protocol. The engine sees the same interface it sees today.

**The rule that governs all of it** stays exactly as it is: all display logic
lives in core (`oled_ctl` + the renderer), all input semantics live in core
(`choice_collect`), and platforms do raw IO only. The web build and the
firmware must render identically from the same state. This is the property that
makes the migration tractable at all, so it takes priority over convenience in
every case.

---

## 2. Hardware target

| Item | Decision | Notes |
|---|---|---|
| Panel | 2.8" **IPS** ST7789, 240x320, SPI | IPS is mandatory: a TN panel shows inverted contrast to whichever seat views it from the far side |
| MCU | RP2350 (Pico 2) | Keeps embassy-rp, probe-rs, and defmt. See §7 for why not ESP32-S3 |
| Framebuffer | full RGB565, 240x320 = 153.6 KB | RP2350 has 520 KB. Trim the heap from its current 64 KB (measured peak is ~3 KB) |
| Sprites | 4-bit indexed + per-species 16-color palette, deflate | ~485 KB for all 386 species, front and back. See §5 |
| Buttons | 4x4 matrix, **one diode per switch** | 14 switches: D-pad (4) + A + B + ? per seat. Diodes give full NKRO and permanently kill the ghosting bug |
| D-pad | rocker part, not four discrete tacts | One hole, correct feel |
| LEDs | 1-2 status LEDs per seat | The 8-LED HP strips are dropped; HP lives on screen |
| Buzzer | **add to the board** | The current PCB has no buzzer footprint at all, so today's audio code plays to nothing |
| Power | spin both a AA and a LiPo variant | |

A 16 MB board (Pico Plus 2 W) is only worth it for WiFi — OTA sprite packs, or
the device serving its own web build. It is not needed for space.

---

## 3. Screen model

The panel lies flat between the seats. Two orientations exist:

**Landscape** — attract mode, the Gen picker, the lobby, and the options menu.
One person reads it upright; configuring is a one-person job, and attract mode
is aimed at bystanders standing beside the table.

**Split portrait** — everything from battle start onward. Two 240x160 halves,
and the far half is rendered rotated 180 degrees so both players read upright.

Orientation switches automatically on battle start and back on battle end.
Debug toggles for every orientation combination ship hidden in the release
build, with web UI buttons that send the same flip commands, because which
arrangement actually feels best is still an open question that needs play
testing rather than argument.

### Screens

Landscape: attract demo, Gen picker (Gen 1 / Gen 3), lobby with ready-up,
options menu.

Split portrait, per half: team reveal, move choice, party list, locked-in,
turn playback, forced switch with rival scouting, battle log, recap.

The `Screen` enum in `oled_ctl.rs` is the seam for all of this and survives the
migration. Variants tied to concealed mode (`ActionSelect`, `ConcealedMoves`,
`SwitchList`, `OpponentMon`, `ControlsSelect`) are deleted; new variants are
added for the options menu, the Gen picker, and the battle log.

---

## 4. Input model

Seven inputs per seat, placed with **rotational** symmetry so both seats get an
identical hand position — not mirrored, which is what the current PCB does.

- **D-pad** moves the cursor. Auto-repeat on hold.
- **A** confirms.
- **B** backs out. Every state has an explicit back path.
- **?** explains whatever the cursor is on: full description and secondary
  effects for a move, full stats for a party member, known information for the
  foe on the HUD.

This replaces the current position-encoded scheme, where a physical corner
button meant the move drawn at that corner, and where a stray tap committed a
choice with no way back.

`?` does not interrupt turn playback. Instead, pressing `?` on a UI element
opens the **battle log**: the whole battle, D-pad scrollable, capped at roughly
the last 50 lines, showing damage numbers and crit and effectiveness reasoning
rather than only the narration text. The log pauses only the reader — the other
player keeps playing, so nobody can stall the game by reading.

Debounce is required and does not currently exist anywhere in the firmware.

---

## 5. Sprite pipeline

Measured, not estimated. Sample: 31 real `pret/pokeemerald` 64x64 sprite
sources, plus 12 PokeAPI Emerald sprites for the color survey.

Gen 3 sprites are at most 64x64 and use at most 16 colors (median 15), so
4-bit indexed with a per-species palette is **lossless**.

Per 64x64 frame:

| Scheme | Size | vs raw |
|---|---|---|
| raw 4bpp | 2048 B | 100% |
| GBA LZ77 — what the Emerald ROM actually stores | 1019 B | 50% |
| deflate, GBA 8x8 tile order | 702 B | 34% |
| **deflate, linear scanline order** | **627 B** | **31%** |

Projected to 386 species, front and back plus palette: ROM parity would be
~780 KB; our scheme lands at **~485 KB**, about 38% under what the real
cartridge spends on the identical pixels.

Two findings that drive the implementation:

1. **Store linear scanline order, not GBA tile order.** Tiling costs 12%
   because it interleaves pixels eight rows apart and destroys horizontal runs.
   The GBA tiled only because its PPU required it; we blit in software.
2. **Use a 512-byte window (deflate `windowBits = 9`).** Measured *smaller*
   than a 32 KB window on this data, because sprites are under 4 KB so a large
   window finds no additional matches while costing more bits to encode. So the
   decompressor needs 512 bytes of RAM, not 32 KB.

Deflate beats the GBA's LZ77 because LZ77 has no entropy coding, and 16-color
sprite data has extremely skewed symbol frequencies, so Huffman pays heavily.
The GBA chose LZ77 because it was free in BIOS, decompressed in place with zero
auxiliary RAM, was roughly ten times faster to decode on a 16.78 MHz ARM7TDMI,
and because saving 295 KB on a 16 MB cart they had already bought bought them
nothing. Every one of those constraints is inverted for us.

**Art direction:** Gen 3 style everywhere, for every species, pegged to the
sprite sets Showdown maintains. A Gen 1 battle uses the same art as a Gen 3
battle. Gen 1's 146-species randbat pool is a subset of the 386, so this is one
sprite set, not two.

Animation is deferred but partly free: Emerald ships multi-frame front sprites
for some species already (2 of 33 sampled were 64x256, i.e. four frames).
Showdown's animation sets are the eventual target.

---

## 6. Game rules and flow

- **Open information.** Concealed mode is deleted. The move a player has locked
  in stays hidden until playback, but nothing else does.
- **Gen picker before the lobby.** Gen 1 or Gen 3, its own screen — it is the
  biggest choice and deserves the framing.
- **Gen 3 is stubbed initially**: the menu entry and all UI plumbing exist, but
  it routes to Gen 1 combat. It must be labeled "preview" until the engine
  lands, and the Gen 3 info fields (abilities, held items, natures, the
  Sp.Atk / Sp.Def split) must sit behind the same ruleset flag as the engine so
  the UI never advertises data the engine cannot fill.
- **Options menu**, lobby only, landscape: team size (3v3 / 6v6), text speed,
  sound, tutorial, turn timer. Either player can change settings; changes apply
  to both and are shown on both halves.
- **Teams are a fresh random draft** every battle, including rematches.
- **Turn timer**, optional, default 60 s, with a visible countdown over the
  final 10 s. Today a stalled player can freeze the game permanently.
- **Hold to fight the AI** in the lobby stays — it is how a lone visitor tries
  the device.
- **Landscape attract demo** stays; it is the only thing aimed at bystanders.
- Animation scope: sprite bob, HP bar drain, damage flash, faint fade. Per-move
  attack animations are out of scope for now.

---

## 7. Engine

The live engine is `gen1_battle/` — 3,615 lines of hand-written `no_std` Rust,
with 238 mon entries and 167 moves. Upstream `battler` is vendored and remains
a dependency, but core has **zero** `battler::` call sites today against 49
`gen1_battle::` ones. It stays for Gen 3 reference.

Gen 3 is not a data swap. It needs the Special stat split into Sp.Atk and
Sp.Def (which touches all of `combat.rs` and `state.rs`), 77 abilities as
behavioral hooks, roughly 60 held items, natures, the Gen 3 IV system, four
weathers, a 17-type chart, and 354 moves instead of 167. Realistically 2-3x the
current engine.

Upstream `battler` already implements this, with executable fxlang effect
scripts — `battle-data/data/abilities/gen3.json` carries all 76 Gen 3 abilities
with real callbacks (Intimidate iterates adjacent foes and applies `atk:-1`;
Levitate overrides `is_grounded`), plus `mons/gen3.json` and `moves/gen3.json`
as delta layers over gen1 and gen2.

**Before writing any Gen 3 engine code**, build core against upstream `battler`
targeting RP2350 and measure flash and RAM. It was presumably dropped because
it would not fit RP2040; on RP2350 with 4 MB that assumption may be stale. If
it fits, gens 1 through 9 come free and a large maintenance burden disappears.
That measurement decides the shape of the whole Gen 3 phase, and it is about a
day of work.

ESP32-S3 was considered and declined: it is Xtensa, so it needs the `espup`
toolchain fork rather than a plain target add, and the entire firmware IO layer
is embassy-rp (I2C, PIO, PWM, USB, Flex GPIO), so the port would be a full
rewrite of that crate plus the loss of a working probe-rs, defmt, and RTT
workflow. RP2350 delivers the RAM and flash without any of that risk.

---

## 8. Migration plan

Each stage lands as its own commit and is verifiable in the web build or the
host CLI **before** any firmware work. Iterating in a browser takes seconds;
iterating on flashed firmware does not.

Before starting: **tag the last commit that carries the mono renderer**, since
this branch deletes it.

Status as of 2026-08-06: stages 1 and 2 are done, stage 3 is done for the
foe HUD, stage 4 is done as an additive `cursor_nav` module (the old
collector is untouched), and stage 6 has a working client at
`mega_blastoise_web/www/device.html`. Stages 7 through 9 are untouched, and
the firmware still runs the mono path.

1. **New renderer beside the old one.** `render_half()` consuming the same
   `Screen` enum, drawing to a 240x160 `Rgb565` target, matching
   `ui_flow.html`. Plus a compose step that writes two halves into one 240x320
   buffer with the far half rotated 180. Old `display.rs` untouched; the two
   paths are selected by a cargo feature so they can be compared side by side
   in a browser.
2. **Sprite pipeline.** `build.rs` currently fetches color PNGs and throws the
   color away at a `luma > 100` threshold. Change the quantizer to emit 4-bit
   indexed plus palette, deflate each with a 512-byte window, and add the
   `no_std` inflate on the read side.
3. **Foe data plumbing.** The HUD needs the opposing active mon's name, level,
   HP percentage, and status inside `ChoiceCollector`. Small, but every choice
   screen depends on it.
4. **Input model.** New event set, cursor state, auto-repeat, and the deletion
   of concealed mode. The riskiest core change: the existing `choice_collect`
   test suite is the safety net, and every test it breaks must be deliberately
   rewritten rather than deleted.
5. **Options menu, Gen picker, battle log, turn timer.**
6. **Web build to full fidelity.** 240x320 canvas, on-screen D-pad and buttons
   per seat rotated to their player, direct tap on UI elements as a shortcut,
   WebAudio driven by the same note tables the buzzer uses, and the orientation
   debug toggles.
7. **Firmware: ST7789 over SPI with DMA**, replacing `subsystems/oled.rs` and
   `sh1106.rs`. One display task now, so the two-panel skew disappears.
8. **Firmware: buttons**, matrix with diodes, real debounce, D-pad repeat.
9. **Delete** the mono renderer, the SH1106 driver, the two-panel path, and the
   LED strip code. Fix the pacing findings from the ergonomics audit at the
   same time: a minimum dwell per narration event, the tutorial shown once per
   power-on rather than every battle, and the rematch dead time.

Gen 3 is a **separate phase after all of this ships**. A working single-screen
Gen 1 device is the milestone that de-risks everything else.

---

## 9. Open questions

- Which orientation arrangement actually feels best in play. Resolved by the
  shipped debug toggles and a real session, not by discussion.
- Font: whether to keep embedded-graphics `FONT_6X10` / `FONT_5X8` or bake an
  authentic Game Boy bitmap font. Expected to need iteration either way.
- Whether upstream `battler` fits on RP2350 (§7).
- Doubles are out of scope but are a plausible future direction for Gen 3.
