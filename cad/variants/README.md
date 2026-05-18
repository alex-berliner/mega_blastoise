# CAD variants

Each `*.scad` here is a complete, standalone enclosure design. Work happens
in these files. **`cad/board.scad` is just a copy of whichever variant is
currently being shown** — the "now showing" / display slot. Don't hand-edit
`board.scad`; edit the variant, then it gets copied up.

Workflow: edit `variants/<name>.scad` → validate via the METHODOLOGY
render+assert loop → `cp variants/<name>.scad ../board.scad` to make it the
active view.

## Variants

| File | Display | Notes |
|------|---------|-------|
| `dual-oled-128x64.scad` | 2× 128×64 mono SSD1306 (I²C), one per player | Web-mirrored layout: OLED + 4 corner move btns + 3-switch row + 1 HP LED + 3 party LEDs per player; centred buzzer. ~71×133×13.5 mm. The committed baseline. |
| `single-st7789.scad` | 1× ST7789 240×240 colour TFT (SPI), shared central | Per-player button block (4 move + 3 switch) + LEDs flanking the screen; buzzer beside display. Colour → sprites. Shared screen = upright for one player only (suits a Stadium-style shared battle view). ~68×121×13.5 mm. |
| `single-st7789-nfc.scad` | ST7789 (as above) + NFC coin select | Drops the 3 switch buttons; per player an octagonal recess (≈24.8 mm, 0.9 mm floor so NFC reads through) + thumb notch for a flat NFC "Pokémon coin". 4 move btns stay. Assumes a per-player PN532-class reader (ghosted; NFC is electronics-out-of-scope today). ~68×149×13.5 mm. |

| `nintendo-style.scad` | ST7789 + amiibo-style NFC | The Yokoi/Iwata take: NFC antenna behind the screen (no slot, nothing to lose; buttons kept so base game needs no accessory), screws hidden on the bottom (clean play face), rounded chamfered shell, raised molded-key bezels, speaker grille, sized for hand-feel. ~70×123×14 mm. |

Currently active in `../board.scad`: **nintendo-style**.

All variants share the METHODOLOGY render+assert loop and the
`part`/`cut`/`check`/`show_ghosts` selectors. STLs are gitignored
(regenerate from whichever variant).
