#!/bin/sh
# Builds RustMagic-<version>.dmg around a finished RustMagic.app.
#
#   tools/package/dmg.sh <version> <app-bundle> <output.dmg> [extra...]
#
# Anything after the output path is copied next to the app in the volume — the
# input method, for instance, which macOS wants in ~/Library/Input Methods rather
# than /Applications.
#
# A .dmg is what a Mac user expects to download: open it, drag the app to
# Applications, eject. The zip stays around for people who automate, but nobody
# should have to unzip a program to install it.
set -e

version=${1:?usage: dmg.sh <version> <app-bundle> <output.dmg>}
app=${2:?}
out=${3:?}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
stage="$work/RustMagic"
mkdir -p "$stage"
cp -R "$app" "$stage/"
shift 3
for extra in "$@"; do
    cp -R "$extra" "$stage/"
done
ln -s /Applications "$stage/Applications"

# No Finder scripting: on a headless runner it fails more often than it works,
# and a plain volume with the app and an Applications link is enough to drag.
hdiutil create \
    -volname "RustMagic $version" \
    -srcfolder "$stage" \
    -fs HFS+ \
    -format UDZO \
    -imagekey zlib-level=9 \
    -ov \
    "$out"

shasum -a 256 "$out" > "$out.sha256"
echo "$out"
