#!/bin/sh
# Turns the one brand SVG into the raster files the operating systems demand.
#
#   tools/package/icons.sh <output-directory>
#
# A .ico and a .icns are formats, not choices: Windows reads the first, macOS
# reads the second, and neither accepts SVG. Everything here is generated from
# packaging/brand/rustmagic.svg so the repository keeps one source of truth and
# no committed bitmaps.
#
# Rasterising needs a real SVG renderer: ImageMagick's built-in one drops
# gradients and masks, which turns this icon into a blank square. librsvg
# (`rsvg-convert`, Debian's librsvg2-bin) is the one to install. ImageMagick is
# then only used to pack the .ico. The macOS job turns the PNG into a .icns with
# iconutil, because only Apple's tools write that format properly.
set -e

out=${1:?usage: icons.sh <output-directory>}
mkdir -p "$out"

svg=packaging/brand/rustmagic.svg
if ! command -v rsvg-convert >/dev/null 2>&1; then
    echo "icons.sh needs rsvg-convert: apt-get install librsvg2-bin" >&2
    exit 1
fi

raster() {
    rsvg-convert --background-color none --width "$1" --height "$1" "$svg" -o "$2"
}

raster 1024 "$out/rustmagic-1024.png"
raster 512 "$out/rustmagic-512.png"
raster 256 "$out/rustmagic-256.png"
raster 128 "$out/rustmagic-128.png"
raster 64 "$out/rustmagic-64.png"
raster 48 "$out/rustmagic-48.png"
raster 32 "$out/rustmagic-32.png"

# Windows: one file, every size a shell asks for.
convert "$out/rustmagic-256.png" "$out/rustmagic-128.png" "$out/rustmagic-64.png" \
        "$out/rustmagic-48.png" "$out/rustmagic-32.png" \
        -define icon:auto-resize=256,128,64,48,32,16 \
        "$out/rustmagic.ico"

ls -l "$out"
