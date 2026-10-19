# Back-of-device label

65 x 145 mm instruction graphic for the back of the console: title, control
legend, how-to-play steps, and Gen 1 rules. Sized to the stripboard footprint
(~61 x 140 mm board, so the label runs edge to edge with a hair of overhang;
trim to the frame line when cutting).

## Files

- `back_label_template.svg` - editable source; fonts referenced as `@@..@@` tokens
- `back_label.svg` - built output, fonts embedded, self-contained
- `back_label.png` - 600 dpi raster (1535 x 3425 px)
- `back_label.pdf` - vector, exact 65 x 145 mm page for printing
- `luckiestguy.ttf`, `baloo2.ttf` - display + body fonts (Google Fonts: Apache 2.0 / OFL)

## Rebuild

```
python3 gen_label.py
```

Edit `back_label_template.svg`, never `back_label.svg`. The script embeds the
fonts, then renders PNG and PDF with headless chromium.

## Printing

Print `back_label.pdf` at 100% scale (no fit-to-page) and cut along the
rounded frame line.
