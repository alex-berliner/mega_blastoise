#!/usr/bin/env bash
# Build the test firmware and emit an RP2350 UF2.
#
# elf2uf2-rs 2.2.0 is RP2040-only: it writes the right blocks but stamps every
# one with family id 0xe48bff56 (RP2040), which an RP2350 in BOOTSEL rejects.
# The payload and load addresses are already correct, so the fix is to restamp
# the family as 0xe48bff59 (RP2350, Arm secure). picotool would do this in one
# step, but it is not packaged for this distro.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

ELF=target/thumbv8m.main-none-eabihf/release/rp2350-blink
OUT=${1:-rp2350-blink.uf2}

cargo build --release
elf2uf2-rs "$ELF" "$OUT"

python3 - "$OUT" <<'PY'
import struct, sys

RP2350_ARM_S = 0xE48BFF59
UF2_MAGIC0, UF2_MAGIC1, UF2_MAGIC_END = 0x0A324655, 0x9E5D5157, 0x0AB16F30
FLAG_FAMILY_ID = 0x2000

path = sys.argv[1]
data = bytearray(open(path, 'rb').read())
assert len(data) % 512 == 0, 'not a UF2: length is not a multiple of 512'

for off in range(0, len(data), 512):
    block = data[off:off + 512]
    m0, m1, flags = struct.unpack('<3I', block[:12])
    end, = struct.unpack('<I', block[-4:])
    assert (m0, m1, end) == (UF2_MAGIC0, UF2_MAGIC1, UF2_MAGIC_END), f'bad magic at block {off // 512}'
    assert flags & FLAG_FAMILY_ID, f'block {off // 512} carries no family id'
    struct.pack_into('<I', data, off + 28, RP2350_ARM_S)

open(path, 'wb').write(data)
print(f'{path}: {len(data) // 512} blocks, family 0x{RP2350_ARM_S:08x} (rp2350-arm-s)')
PY
