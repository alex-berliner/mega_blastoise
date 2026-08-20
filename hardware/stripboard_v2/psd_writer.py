#!/usr/bin/env python3
"""Minimal layered-PSD writer: enough of the format for named RGBA layers.

Written by hand because nothing in the toolchain writes layered PSD (PIL only
reads them) and GIMP's batch mode cannot write files in this environment. The
subset implemented here is: 8-bit RGB, one alpha per layer, PackBits rows,
per-layer name and visibility. That is all a build reference needs.

Format reference: Adobe Photoshop File Formats Specification, sections
"File Header", "Layer and Mask Information", "Image Data".
"""

import struct


def packbits(data: bytes) -> bytes:
    """PackBits RLE, the compression PSD calls method 1."""
    out = bytearray()
    i, n = 0, len(data)
    while i < n:
        # look for a run of three or more
        run = 1
        while i + run < n and run < 128 and data[i + run] == data[i]:
            run += 1
        if run >= 3:
            out.append(257 - run)
            out.append(data[i])
            i += run
            continue
        # otherwise gather a literal block, stopping before a real run
        start = i
        lit = 0
        while i < n and lit < 128:
            if i + 2 < n and data[i] == data[i + 1] == data[i + 2]:
                break
            i += 1
            lit += 1
        out.append(lit - 1)
        out += data[start:i]
    return bytes(out)


def _rle_channel(plane: bytes, w: int, h: int):
    """Compress one channel plane row by row; returns (counts, body)."""
    counts, body = bytearray(), bytearray()
    for y in range(h):
        row = packbits(plane[y * w:(y + 1) * w])
        counts += struct.pack(">H", len(row))
        body += row
    return bytes(counts), bytes(body)


def _pascal4(s: str) -> bytes:
    """Pascal string padded so the whole field is a multiple of 4 bytes."""
    b = s.encode("latin-1", "replace")[:255]
    out = bytes([len(b)]) + b
    pad = (-len(out)) % 4
    return out + b"\x00" * pad


def _lsct(kind: int) -> bytes:
    """Section-divider tag: 1 = open folder, 2 = closed, 3 = folder end.

    PSD encodes a layer group as a run of records: a hidden "end" marker
    below the children and the named folder record above them.
    """
    return b"8BIM" + b"lsct" + struct.pack(">I", 4) + struct.pack(">I", kind)


def _layer_record(name, visible, channels, extra_tag=b""):
    """One layer record plus its channel blobs, ready to concatenate."""
    rec = bytearray()
    (w, h, planes) = channels
    rec += struct.pack(">iiii", 0, 0, h, w)
    rec += struct.pack(">H", 4)

    blobs = []
    if planes is None:                      # a divider: no pixels at all
        for cid in (-1, 0, 1, 2):
            blob = struct.pack(">H", 0)     # raw, zero rows
            blobs.append(blob)
            rec += struct.pack(">hI", cid, len(blob))
    else:
        r, g, b, a = planes
        for cid, plane in ((-1, a), (0, r), (1, g), (2, b)):
            counts, body = _rle_channel(plane, w, h)
            blob = struct.pack(">H", 1) + counts + body
            blobs.append(blob)
            rec += struct.pack(">hI", cid, len(blob))

    rec += b"8BIM" + b"norm"
    rec += bytes([255, 0, 0 if visible else 2, 0])

    name_field = _pascal4(name)
    uni = name.encode("utf-16-be")
    luni = (b"8BIM" + b"luni" + struct.pack(">I", 4 + len(uni))
            + struct.pack(">I", len(name)) + uni)
    if len(luni) % 2:
        luni += b"\x00"
    extra = struct.pack(">I", 0) + struct.pack(">I", 0) + name_field + luni + extra_tag
    rec += struct.pack(">I", len(extra)) + extra
    return bytes(rec), b"".join(blobs)


def write_psd(path, width, height, nodes, merged_rgb):
    """nodes: the stack, bottom to top. Each entry is either

        ("layer", name, visible, (r, g, b, a))
        ("group", name, visible, [child nodes])

    where each plane is width*height bytes. merged_rgb is (r, g, b) for the
    flattened composite.
    """
    f = open(path, "wb")

    # --- file header ---
    f.write(b"8BPS")
    f.write(struct.pack(">H", 1))          # version 1
    f.write(b"\x00" * 6)                   # reserved
    f.write(struct.pack(">H", 3))          # channels in the composite
    f.write(struct.pack(">I", height))
    f.write(struct.pack(">I", width))
    f.write(struct.pack(">H", 8))          # bits per channel
    f.write(struct.pack(">H", 3))          # colour mode: RGB

    f.write(struct.pack(">I", 0))          # colour mode data: none
    f.write(struct.pack(">I", 0))          # image resources: none

    # --- layer records, then their channel data ---
    records, chan_blobs = bytearray(), bytearray()
    n_records = 0

    def emit(node):
        nonlocal n_records
        kind = node[0]
        if kind == "layer":
            _, name, visible, planes = node
            rec, blob = _layer_record(name, visible, (width, height, planes))
            records.extend(rec)
            chan_blobs.extend(blob)
            n_records += 1
            return
        _, name, visible, children = node
        # the closing marker sits below the children in file order
        rec, blob = _layer_record("</Layer group>", True,
                                  (width, height, None), _lsct(3))
        records.extend(rec)
        chan_blobs.extend(blob)
        n_records += 1
        for child in children:
            emit(child)
        rec, blob = _layer_record(name, visible, (width, height, None), _lsct(1))
        records.extend(rec)
        chan_blobs.extend(blob)
        n_records += 1

    for node in nodes:
        emit(node)

    layer_info = struct.pack(">h", n_records) + bytes(records) + bytes(chan_blobs)
    if len(layer_info) % 2:
        layer_info += b"\x00"
    layer_and_mask = struct.pack(">I", len(layer_info)) + layer_info
    layer_and_mask += struct.pack(">I", 0)      # global layer mask: none
    f.write(struct.pack(">I", len(layer_and_mask)))
    f.write(layer_and_mask)

    # --- flattened composite ---
    counts, bodies = bytearray(), bytearray()
    for plane in merged_rgb:
        c, bdy = _rle_channel(plane, width, height)
        counts += c
        bodies += bdy
    f.write(struct.pack(">H", 1))
    f.write(bytes(counts))
    f.write(bytes(bodies))
    f.close()
