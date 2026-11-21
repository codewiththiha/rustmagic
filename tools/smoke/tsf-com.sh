#!/bin/sh
# Builds the Windows text service and drives its COM surface under Wine.
#
#   tools/smoke/tsf-com.sh
#
# The same sources the Windows job builds with MSVC, cross-compiled with
# mingw-w64 and loaded by Wine. It proves what can be proved without Windows:
# the DLL is loadable, exports the four entry points COM looks for, creates a
# service whose QueryInterface answers to the published TSF identifiers, counts
# its references correctly, and — through DllRegisterServer — writes a CLSID that
# COM can create it from.
#
# What it cannot prove is a keystroke reaching a document, which needs a real
# TSF manager and a focused window; the Windows CI job covers the activation
# path and INSTALL.txt lists the rest as human checks.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

cc=${MINGW_CXX:-x86_64-w64-mingw32-g++}
wine=${WINE:-$(command -v wine64 || command -v wine || echo /usr/lib/wine/wine64)}
if [ ! -x "$wine" ] && ! command -v "$wine" >/dev/null 2>&1; then
    echo "tsf-com.sh needs Wine: WINEDEBUG=-all wine64 <exe>" >&2
    echo "install it with: apt-get install -y wine64" >&2
    exit 1
fi

out=${TSF_BUILD_DIR:-build/tsf}
mkdir -p "$out"

# The engine, cross-built for the Windows ABI the DLL links against.
cargo build -p km-ffi --release --target x86_64-pc-windows-gnu

"$cc" -std=c++17 -O2 -Wall -Wextra -Wno-delete-non-virtual-dtor \
    -shared -static -static-libgcc -static-libstdc++ \
    -Wl,-s -Wl,--gc-sections -Wl,--kill-at \
    -I crates/km-ffi/include \
    -o "$out/RustMagicTSF.dll" \
    platform/windows/tsf/class_factory.cpp \
    platform/windows/tsf/display_attribute.cpp \
    platform/windows/tsf/dllmain.cpp \
    platform/windows/tsf/engine.cpp \
    platform/windows/tsf/text_service.cpp \
    platform/windows/tsf/tsf_guids.cpp \
    target/x86_64-pc-windows-gnu/release/libkm_ffi.a \
    -lole32 -loleaut32 -luuid -ladvapi32 -lkernel32 -luser32 \
    -lws2_32 -luserenv -lbcrypt -lntdll -lsynchronization -lmsvcrt \
    platform/windows/tsf/tsf.def

"$cc" -std=c++17 -O1 -Wall -Wextra -o "$out/tsf-com.exe" tools/smoke/tsf-com.cpp \
    -lole32 -luuid

win_dll=$(cd "$out" && pwd -W 2>/dev/null || echo "$(pwd)/RustMagicTSF.dll")
win_exe=$(cd "$out" && pwd -W 2>/dev/null || echo "$(pwd)/tsf-com.exe")

export WINEPREFIX=${WINEPREFIX:-build/wine}
export WINEDEBUG=-all
# Wine wants a runtime directory even when nothing needs a display.
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-$WINEPREFIX/run}
mkdir -p "$WINEPREFIX" "$XDG_RUNTIME_DIR"

"$wine" "${win_exe}" --register "${win_dll}"
status=$?
"$wine" "${win_exe}" --unregister "${win_dll}" || status=1

exit $status
