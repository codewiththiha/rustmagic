#!/bin/sh
# Installs RustMagic's input method for the current user.
#
#   sh install-ime.sh
#
# An input method lives in ~/Library/Input Methods, which needs no administrator
# rights and no installer: macOS scans that folder and offers what it finds in
# System Settings > Keyboard > Input Sources. Running this from the .dmg, from
# the unzipped release, or from a build tree all do the same thing.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
bundle="$here/RustMagicIME.app"
if [ ! -d "$bundle" ]; then
    echo "RustMagicIME.app is not next to this script" >&2
    exit 1
fi

destination="$HOME/Library/Input Methods"
mkdir -p "$destination"

# The system keeps the old bundle open while it is the active input source, so
# replacing it means taking it out first.
if [ -d "$destination/RustMagicIME.app" ]; then
    rm -rf "$destination/RustMagicIME.app"
fi
cp -R "$bundle" "$destination/"

# Nudge the agent that lists input methods, so it appears without a log out.
killall -HUP TextInputMenuAgent 2>/dev/null || true

echo "installed $destination/RustMagicIME.app"
echo
echo "Add it once: System Settings > Keyboard > Text Input > Input Sources > +,"
echo "choose Burmese (or English), and pick RustMagic. Then switch to it with"
echo "Control+Space or the input menu in the menu bar."
