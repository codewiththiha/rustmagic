#!/bin/sh
# Runs the IBus engine against a private session bus and drives it with a C
# client that decodes the replies through libibus.
#
#   tools/smoke/ibus.sh            # builds and checks the engine
#
# Needs: a session bus (dbus-daemon), libibus and its headers. Ubuntu and Arch
# both ship them; CI installs them before calling this.
set -e
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"

cargo build --quiet -p rustmagic-ibus
cc tools/smoke/ibus-engine.c $(pkg-config --cflags --libs ibus-1.0) -o target/ibus-smoke

state=$(mktemp -d)
export XDG_STATE_HOME="$state"
export XDG_CACHE_HOME="$state/cache"

target/debug/rustmagic-ibus &
engine=$!
trap 'kill $engine 2>/dev/null || true; rm -rf "$state"' EXIT INT TERM

# The engine needs a moment to own its bus name.
for _ in $(seq 1 50); do
    if busctl --address="$DBUS_SESSION_BUS_ADDRESS" --user list 2>/dev/null | grep -q rustmagic; then
        break
    fi
    sleep 0.1
done

target/ibus-smoke org.freedesktop.IBus.RustMagic rustmagic
