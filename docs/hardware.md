# Hardware

## PN532 NFC readers

Two PN532 modules on separate I²C buses (`I2C0` + `I2C1`), default address **0x24** each:

| Bus | SCL (GP) | SDA (GP) |
|-----|----------|----------|
| I2C0 (reader 0) | GP17 | GP16 |
| I2C1 (reader 1) | GP19 | GP18 |

## Breadboard power

PN532 modules draw significant current when the RF field is active. Powering both from the Pico's 3V3 pin can brown out the regulator or cause flaky I²C.

**Recommended setup:**

1. **External supply** — a small DC brick into a buck module exposing stable 5 V / 3.3 V at enough current for both readers plus headroom.
2. **Power readers from that rail**, not Pico 3V3:
   - Most PN532 breakouts accept 5 V on VIN and regulate down on-board.
   - If the breakout is 3.3 V only, use the external supply's 3.3 V output.
3. **Common ground** — tie supply GND, Pico GND, and both reader GNDs on one net (breadboard negative rail).
4. **Pico during development** — easiest to power the Pico over USB while the external supply feeds only the NFC modules (with common GND).
5. **I²C levels** — RP2040 GPIO is 3.3 V logic; most PN532 breakouts are 3.3 V I²C tolerant. 5 V-only I²C modules would need level shifting.
