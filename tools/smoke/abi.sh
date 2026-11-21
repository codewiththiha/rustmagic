#!/bin/sh
# Compiles tools/smoke/abi.c against the built C ABI and runs it.
#
#   tools/smoke/abi.sh
#
# On Linux this links the shared object; macOS and Windows do the same with
# their own extension. CI runs it on every push, so a change to the ABI that
# breaks a C caller fails before it ships.
set -e
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"

cargo build --quiet -p km-ffi
lib_dir="$root/target/debug"
cc -I crates/km-ffi/include tools/smoke/abi.c \
   -L"$lib_dir" -lkm_ffi -o target/abi-smoke
# Point the loader at the freshly built library, so the example runs from a
# checkout without installing anything.
case "$(uname -s)" in
    Darwin) export DYLD_LIBRARY_PATH="$lib_dir" ;;
    *)      export LD_LIBRARY_PATH="$lib_dir" ;;
esac
target/abi-smoke fixtures/basic.km2
