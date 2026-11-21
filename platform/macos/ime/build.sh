#!/bin/sh
# Builds RustMagicIME.app — the macOS input method.
#
#   platform/macos/ime/build.sh [output-directory]
#
# Produces build/macos/RustMagicIME.app, a bundle that can be dropped into
# ~/Library/Input Methods and selected in System Settings > Keyboard > Input
# Sources. It carries the Swift input method, the engine's C ABI library and the
# bundled keyboards, so it works with no other part of RustMagic installed.
#
# Needs the Xcode command line tools (swiftc) and cargo. UNIVERSAL=1 builds both
# architectures, which is what the release ships.
set -eu

root=$(cd "$(dirname "$0")/../../.." && pwd)
here="$root/platform/macos/ime"
out=${1:-"$root/build/macos"}
cd "$root"

name=RustMagicIME
app="$out/$name.app"
contents="$app/Contents"
shipped="$out/libkm_ffi.dylib"

# --- the engine -------------------------------------------------------------

mkdir -p "$out"
if [ "${UNIVERSAL:-0}" = "1" ]; then
    for target in aarch64-apple-darwin x86_64-apple-darwin; do
        cargo build -p km-ffi --release --target "$target"
    done
    # Both slices in one library: the bundle has to run on Intel and Apple
    # silicon alike, for the same reason the app is universal.
    lipo -create -output "$shipped" \
        target/aarch64-apple-darwin/release/libkm_ffi.dylib \
        target/x86_64-apple-darwin/release/libkm_ffi.dylib
else
    cargo build -p km-ffi --release
    install -m755 target/release/libkm_ffi.dylib "$shipped"
fi

# --- the bundle -------------------------------------------------------------

rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Frameworks" "$contents/Resources/keyboards" \
         "$contents/Resources/include"

# No -L/-l here: the library is linked by path below, so the link cannot pick up
# a stale copy from somewhere else on the machine.
swiftc -O -swift-version 5 \
    -module-name RustMagicIM \
    -import-objc-header "$here/Sources/Bridging.h" \
    -I crates/km-ffi/include \
    -framework Cocoa -framework InputMethodKit -framework Carbon \
    -Xlinker -rpath -Xlinker "@executable_path/../Frameworks" \
    -Xlinker "$shipped" \
    -o "$contents/MacOS/$name" \
    "$here"/Sources/*.swift

install -m644 "$here/Info.plist" "$contents/Info.plist"
printf 'APPL????' > "$contents/PkgInfo"
install -m644 crates/km-ffi/include/rustmagic.h "$contents/Resources/include/rustmagic.h"
install -m644 "$root"/keyboards/bundled/*.km2 "$contents/Resources/keyboards/"

# The engine lives inside the bundle: an input method runs inside other people's
# applications, so it may not assume anything else is installed.
install -m755 "$shipped" "$contents/Frameworks/libkm_ffi.dylib"
install_name_tool -id "@rpath/libkm_ffi.dylib" "$contents/Frameworks/libkm_ffi.dylib"
# Whatever absolute path cargo left in the executable is rewritten to the copy
# inside the bundle, or the app would only run on the machine that built it.
for dependency in $(otool -L "$contents/MacOS/$name" | awk 'NR>1 {print $1}' | grep 'libkm_ffi'); do
    install_name_tool -change "$dependency" "@rpath/libkm_ffi.dylib" "$contents/MacOS/$name"
done

# The icon comes from the brand SVG through icons.sh and iconutil, done by the
# release job; without it the bundle simply has none.
if [ -f "$out/RustMagic.icns" ]; then
    install -m644 "$out/RustMagic.icns" "$contents/Resources/RustMagic.icns"
    /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string RustMagic" \
        "$contents/Info.plist" 2>/dev/null || true
fi

# macOS refuses to load an unsigned input method, so even a local build gets a
# signature — ad hoc here, Developer ID and notarisation in the release.
codesign --force --deep --sign - "$app"

echo "built $app"
otool -L "$contents/MacOS/$name" | sed -n '2,$p'
