#!/bin/sh
# Installs RustMagic's input method for the current user.
#
#   sh install-ime.sh
#
# The work is the bundle's own: `RustMagicIME --install` copies itself into
# ~/Library/Input Methods, registers it with Text Input Source Services so it
# appears without a logout, and prints the one step that is the system's to
# allow. This script exists so a checked-out tree and a .dmg behave the same.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
bundle="$here/RustMagicIME.app"
if [ ! -d "$bundle" ]; then
    echo "RustMagicIME.app is not next to this script" >&2
    exit 1
fi

exec "$bundle/Contents/MacOS/RustMagicIME" --install
