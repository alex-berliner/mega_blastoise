# Mega Blastoise — Project Overview

A self-contained two-player Pokémon Generation 1 battle game that runs entirely on a Raspberry Pi Pico. No phone, no PC, no internet. Two players sit across a portable board, pick a team, and battle in 3v3 matches with full Gen 1 mechanics — type effectiveness, status effects, damage rolls, critical hits — all driven by an open-source battle engine.

The first version is designed for **demoing at conventions** (Open Sauce 2026). That shapes most of the design choices: fast setup, no fragile parts, intuitive enough that two strangers can sit down and play their first match within 15 seconds of approaching the board.

---

## Why This Exists

The goal is a physical artifact that feels like a purpose-built game device — not an emulator, not a phone app. The board should be something you can hand to anyone and play without setup. All game logic runs on a $4 microcontroller.

Gen 1 Pokémon is the right scope: well-understood rules, no abilities or items to implement, small enough to fit entirely in flash memory, and nostalgic enough that people immediately get it. The presentation reference is **Pokémon Stadium** — dramatic per-hit feedback, HP bars draining LED-by-LED, attack flashes, victory fanfares.

---

## What Playing It Looks Like

A complete game session, end to end:

**1. Idle.** When no one's playing, the board is in attract mode. The buzzer chimes a quick battle jingle every 30 seconds, all 24 LEDs pulse softly, and the OLED displays alternate between "PLAY" and "ME". Hard to ignore.

**2. Sit down.** Two players sit on opposite sides of the board. The first player to press any button kicks off a **mystery draft**: each player's three Pokémon are revealed one at a time on their personal OLED display, with sprite art, name, and a fanfare beep per reveal. ~15 seconds of theatre.

**3. Battle starts.** "Trainers ready!" — both OLEDs swap to your active Pokémon's portrait, HP bars light up to full green, the buzzer plays a ready-up chord. Move buttons pulse under your fingers.

**4. Pick a move.** Your OLED shows your active Pokémon and a numbered list of its four moves. Press a button to commit. **Hold a button** to inspect the move first — name, type, power, accuracy, secondary effects. Both players pick simultaneously; whoever's faster acts first.

**5. Moves resolve.** The attacker's LED strip flashes the move's type color. The defender's HP bar drains LED-by-LED with a single buzzer hit. Numbers update. Snappy — about 4 seconds per turn.

**6. Status effects** show up as a small sprite tag on the OLED, a colored status LED, and — to teach what each status actually does — a brief tint of the affected player's HP bar in the status's color.

**7. A Pokémon faints.** Their party-slot LED fades to black. A short "down" tone plays. The OLED shows a grayed-out sprite, then prompts you to pick a replacement from your remaining team. Meanwhile your opponent's OLED shows your remaining Pokémon's HP and types — a tactical timeout that gives both players a moment to think.

**8. The battle ends.** When the last Pokémon falls, both OLEDs show a short stat recap (damage dealt, crits landed, turns elapsed) before the winner's side erupts in a victory fanfare and the loser's side dims.

**9. Replay.** "Press any button to play again." Ten seconds of inactivity returns the board to attract mode for the next pair of strangers.

---

## Hardware at a Glance

| What | Why |
|------|-----|
| Raspberry Pi Pico (RP2040) | Runs everything — $4, capable enough, great LED hardware |
| WS2812B NeoPixel strip (×24) | HP bars, party status, attack flashes, faint animations |
| 2× 128×64 monochrome OLED | One per player — sprites, move lists, tooltips, MVP recap |
| Tactile buttons (×14) | 4 move + 3 party buttons per player |
| Piezo buzzer | Audio cues — every action has a sound |
| LiPo battery + boost converter | Untethered, runs all day on a charge |

For physical layout, wiring, and GPIO assignment, see [ELECTRONICS.md](./ELECTRONICS.md).

---

## What's Working / What's Left

| Area | Status |
|------|--------|
| Battle engine — full Gen 1 rules | Working |
| Play over USB serial (laptop) | Working |
| Button matrix input | Not started |
| OLED sprite + UI rendering | Not started |
| LED HP bars and animations | Not started |
| Buzzer audio cues | Not started |
| Preset team rosters + mystery draft | Not started |
| Battery + power management | Not started |
| Physical board / enclosure | Not started |

The game is fully playable today over a USB cable: flash the firmware, open a serial terminal, and play a complete battle by typing move choices. Everything physical is still ahead.

---

## Out of Scope (for now)

Earlier design iterations explored larger 6v6 teams, NFC card scanning for team selection, physical Pokémon standees with embedded chips, and per-Pokémon move cards. Those are interesting and may resurface in a v2 home edition, but they're poor fits for a convention demo. Quick games, no loose pieces, zero learning curve — that's the v1 brief.

Also out of scope: AI opponent (the demo is two-player only), full 151-Pokémon roster, sound chip / proper audio, wireless multiplayer.

---

## More Detail

- **Electronics design** — [ELECTRONICS.md](./ELECTRONICS.md)
- **Software internals** — [TECHNICAL.md](./TECHNICAL.md)
- **Full spec and architecture** — `specs.md`, `architecture/`
