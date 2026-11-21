# Linux: IBus, Arch, and everything around it

On Windows and macOS a keyboard layout plugs into the system's text service. On
Linux the equivalent is an **input method**: the desktop talks to IBus, and IBus
talks to an engine.

RustMagic's engine is `apps/rustmagic-ibus`. It is a plain Rust process that
speaks the IBus D-Bus protocol directly — no glib, no GObject subclassing, no
C bindings to libibus. That is why it can be driven and tested by anything that
can make a D-Bus call.

## How the pieces fit

```
keyboard ──▶ desktop ──▶ IBus daemon ──▶ rustmagic-ibus ──▶ km-engine
                              │                 │
                              │                 └─ CommitText / UpdatePreeditText
                              └─ reads /usr/share/ibus/component/rustmagic.xml
```

* `rustmagic.xml` tells IBus the engine exists: its bus name, the program that
  serves it, and how to label it in the input-method menu.
* IBus starts `rustmagic-ibus --ibus` when the engine is selected, and asks its
  factory object for an engine.
* Key events arrive as `(keyval, keycode, state)`. The engine maps the keysym to
  a virtual key and a character, runs `km-engine`, and answers with a commit or a
  preedit update.

## Install

### Arch Linux

From a release, into an empty directory, with nothing else to fetch:

```sh
version=0.2.0
base=https://github.com/codewiththiha/rustmagic/releases/download/v$version
curl -LO "$base/PKGBUILD"
curl -LO "$base/rustmagic.install"
curl -LO "$base/rustmagic-$version.tar.gz"
makepkg -si
```

The three files keep the names `makepkg` looks for. If the release is downloaded
through a browser, it may have saved them as `PKGBUILD.1` or `PKGBUILD.txt`;
rename them back before running, or `makepkg` will answer `PKGBUILD does not
exist`. The built package is also published, so `sudo pacman -U
rustmagic-<version>-1-x86_64.pkg.tar.zst` skips the compile entirely.

From a clone of the branch, `PKGBUILD-git` is the rolling one:

```sh
git clone https://github.com/codewiththiha/rustmagic
cd rustmagic/packaging/arch
makepkg -si            # or: makepkg -si -p PKGBUILD-git
```

The package installs `rustmagic`, `rustmagic-cli`, `rustmagic-ibus`, the IBus
component file, a desktop entry and icons. Its `.install` script refreshes the
IBus cache so the engine appears without a reboot.

Then:

```sh
rustmagic                      # install a keyboard once
ibus restart                   # pick up the component
```

and add **RustMagic** as an IBus input method.

On KDE Plasma Wayland, selecting RustMagic in an IBus list is not by itself
sufficient: set System Settings → Input & Output → Keyboard → Virtual Keyboard
to **IBus Wayland**, apply it, then add RustMagic in IBus input-method settings.
RustMagic exposes one IBus engine with a **Keyboard** submenu for the layouts
installed in its library; selecting one there also updates the active keyboard
in the RustMagic app. Its Settings page can limit that menu to the active layout,
all layouts, or a selected subset. KDE's IBus support has application-specific
limits—test in LibreOffice Writer or another IBus-aware editor before diagnosing
a layout bug; some KDE applications such as Konsole may not work with IBus
engines. Keyman's KDE instructions document the same IBus Wayland setup and
application caveat: <https://help.keyman.com/kb/101>.

Dependencies are `ibus`, `glibc` and `gcc-libs`; the build needs `rust` and
`cargo`. The window adds no toolkit of its own — it draws with egui over
OpenGL — so no GTK or Qt development packages are needed to build it.

### Any other distribution

```sh
cargo build --release -p rustmagic -p rustmagic-cli -p rustmagic-ibus
sudo install -Dm755 target/release/rustmagic-ibus /usr/bin/rustmagic-ibus
sudo install -Dm644 assets/ibus/rustmagic.xml \
     /usr/share/ibus/component/rustmagic.xml
ibus write-cache && ibus restart
```

For a per-user install, put the component file in
`~/.config/ibus/component/rustmagic.xml` and correct the `<exec>` path.

Under Wayland, IBus needs `GTK_IM_MODULE=ibus`, `QT_IM_MODULE=ibus` and
`XMODIFIERS=@im=ibus` in the session environment; most desktops set these
already. Under X11 the engine also works through the XIM bridge that
`ibus-daemon --xim` provides.

## Hotkeys

A keyboard's `@HOTKEY` switches to it while the engine is active — the same
behaviour the Windows build has. The engine watches every key event for a
matching hotkey before it hands the key to the layout, and it needs the keyboard
in the library, so install keyboards with `rustmagic` first.

## Verifying an install

Three commands, from least to most involved:

```sh
rustmagic-cli type "mingalaba"        # the engine, without IBus at all
ibus list-engine | grep -i rustmagic  # IBus has read the component file
dbus-run-session -- tools/smoke/ibus.sh   # the full protocol, in a private bus
```

The last one is what CI runs: it starts the engine on a throwaway session bus,
asks its factory for an engine, sends `a` and space, and decodes the reply with
**libibus itself**. If libibus can read the payload, a real panel can.

## Troubleshooting

**The engine is not in the input-method list.** IBus caches components. Run
`ibus write-cache`, then `ibus restart`; check that the component file is where
IBus looks (`/usr/share/ibus/component` or `~/.config/ibus/component`) and that
its `<exec>` path exists and is executable.

**It is listed but typing does nothing.** Check that the engine process is
running (`busctl --user list | grep rustmagic`) and that the library has a
keyboard (`rustmagic-cli list`). An empty library means the engine has nothing to
apply, and it will pass every key through.

**Text appears twice, or a key vanishes.** That is the preedit/commit boundary.
The engine commits when a syllable ends — space, Enter, Tab, Escape, a modifier,
or focus loss — and otherwise keeps the composition in the preedit. A layout that
maps `ANY` (which matches any printable character) should also map the keys it
wants to end on; otherwise the layout decides, not the desktop.

**Debugging.** Run the engine in a terminal to see its complaints:

```sh
RUST_BACKTRACE=1 rustmagic-ibus --ibus
```

and watch the bus traffic with
`dbus-monitor --session "interface='org.freedesktop.IBus.Engine'"`.

## What is verified, and what is not

Verified in this repository's tests and smoke scripts:

* the component file is valid XML and IBus 1.5 lists the engine from it;
* the service owns its bus name, serves a factory, and answers `CreateEngine`;
* `ProcessKeyEvent` drives the real keyboard files and returns the right
  handled/not-handled answer;
* `CommitText` and `UpdatePreeditText` carry payloads libibus decodes into the
  text the layout produced;
* everything builds and the package metadata is valid on Arch's toolchain (CI).

Not verified here, because it needs a live desktop session: the panel-side switch
(`SetGlobalEngine`) completing against a real GNOME/KDE session. The pieces on
either side of that step are tested; the step itself is the desktop's.
