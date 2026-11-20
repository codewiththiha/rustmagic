#!/usr/bin/env python3
"""Checks that the generated icons actually contain a picture.

    python3 tools/package/check_icon.py dist/brand

A vector icon that a renderer only half understands comes out as a blank or
single-colour square, and a blank square is a valid PNG, so file size proves
nothing. This reads the pixels and insists on a range of colours; it also
insists the .ico and the 1024px PNG are there, because macOS and Windows build
their own formats from that PNG.
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

MINIMUM_COLOURS = 32


def main(argv: list[str]) -> int:
    directory = Path(argv[0] if argv else "dist/brand")
    problems = []

    png = directory / "rustmagic-256.png"
    if not png.is_file():
        problems.append(f"{png} is missing")
    else:
        colours = distinct_colours(png.read_bytes())
        if colours < MINIMUM_COLOURS:
            problems.append(
                f"{png.name} has only {colours} distinct bytes of pixel data; "
                "the renderer probably dropped the gradients"
            )
        else:
            print(f"{png.name}: {colours} distinct pixel bytes")

    for name in ("rustmagic.ico", "rustmagic-1024.png"):
        path = directory / name
        if not path.is_file() or path.stat().st_size == 0:
            problems.append(f"{name} is missing or empty")

    icon = directory / "rustmagic.ico"
    if icon.is_file() and icon.read_bytes()[:4] != b"\x00\x00\x01\x00":
        problems.append(f"{icon.name} is not an icon file")

    for problem in problems:
        print(f"✗ {problem}", file=sys.stderr)
    if problems:
        return 1
    print("the icons look like pictures")
    return 0


def distinct_colours(png: bytes) -> int:
    """Distinct bytes in the decompressed pixel stream, as a cheap variety test."""
    position = 8
    data = b""
    while position < len(png):
        length = struct.unpack(">I", png[position : position + 4])[0]
        kind = png[position + 4 : position + 8]
        if kind == b"IDAT":
            data += png[position + 8 : position + 8 + length]
        position += 12 + length
    return len(set(zlib.decompress(data)))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
