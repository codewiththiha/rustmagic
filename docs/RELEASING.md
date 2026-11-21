# Releasing

A release is a tag. `.github/workflows/release.yml` builds every platform from
that tag, checks each one, and publishes the results as one GitHub release.
Nothing is uploaded to a package registry, and nothing is signed yet.

## Cutting one

```sh
cargo xtask release 0.2.0    # bump, gate, commit, tag — but do not push
git push origin main --tags  # publish: this is the step that ships
```

`cargo xtask release` refuses to run on a dirty tree, bumps the version in
`Cargo.toml`, `packaging/arch/PKGBUILD`, `packaging/macos/Info.plist` and
`assets/ibus/rustmagic.xml`, runs `cargo xtask check`, and creates an annotated
tag `v0.2.0`. Pushing the tag is deliberately a separate, human action.

The version in the tag is the version in the artifacts: the workflow strips the
leading `v` and uses what is left for every file name and for
`CFBundleShortVersionString`. A tag that does not match the manifests is a bug,
which is why the bump and the tag come from the same command.

## What each platform gets

Every platform gets something a person can double-click or hand to their package
manager. The archives are still built, because automation wants them and because
they are the smallest thing to inspect, but they are not the headline download.

| Job | Runner | Files | Verified by |
| --- | --- | --- | --- |
| `brand` | ubuntu-latest | the icon set as an artifact, not a release asset | `tools/package/check_icon.py` — the rasterised icon must have more than one colour in it |
| `linux` | ubuntu-22.04 | `rustmagic_<v>_amd64.deb`, `rustmagic-<v>-linux-x86_64.tar.gz` (+ `.sha256`) | `cargo test`, then `tools/smoke/ibus.sh` over a real D-Bus session, then a `dpkg-deb -c` of the `.deb` |
| `rpm` | ubuntu-latest in a `fedora:latest` container | `rustmagic-<v>-1.x86_64.rpm` | `rpmbuild` exits zero and `rpm -qlp` lists the binaries and the five keyboards |
| `macos` | macos-latest | `RustMagic-<v>-universal.dmg` (+ `.sha256`), `rustmagic-<v>-macos-universal.zip` | `cargo test`, `lipo -archs` on each binary, then `hdiutil verify` on the disk image |
| `windows` | windows-latest | `RustMagic-<v>-Setup-x64.exe`, `RustMagic-<v>-x64.msi`, `rustmagic-<v>-windows-x86_64.zip` (+ `.sha256`) | `cargo test`, then Inno Setup and WiX both compile the installers |
| `arch` | `archlinux:latest` container | `rustmagic-<v>-1-x86_64.pkg.tar.zst`, `PKGBUILD`, `rustmagic.install`, `rustmagic-<v>.SRCINFO`, `rustmagic-<v>.tar.gz` | `makepkg` builds as a non-root user, after the same tests |

`brand` runs first because the icon is one file that four jobs need: the `.deb`,
the `.rpm`, the `.dmg` and both Windows installers all take their icons from it.
The source of truth is `packaging/brand/rustmagic.svg`; nothing rasterised lives
in the tree. `tools/package/icons.sh` turns it into a 1024 PNG, the PNG sizes
Windows wants, a `.ico`, and the PNG set `iconutil` needs on macOS, using
`rsvg-convert` (ImageMagick's SVG renderer silently drops the gradients and
produces a blank square).

The other six jobs run in parallel; `publish` waits for all of them and asserts
that each expected file exists before it turns the draft release public. A job
that quietly uploads nothing therefore fails the release instead of shipping
half of one.

### Installing each one

| Platform | Download | Install with |
| --- | --- | --- |
| Windows | `RustMagic-<v>-Setup-x64.exe` | run it: Start menu entry, Add/Remove Programs entry, optional PATH |
| Windows, managed | `RustMagic-<v>-x64.msi` | `msiexec /i`, Group Policy, Intune |
| macOS | `RustMagic-<v>-universal.dmg` | open, drag `RustMagic.app` to Applications |
| Debian, Ubuntu | `rustmagic_<v>_amd64.deb` | `sudo apt install ./rustmagic_<v>_amd64.deb` |
| Fedora, RHEL | `rustmagic-<v>-1.x86_64.rpm` | `sudo dnf install ./rustmagic-<v>-1.x86_64.rpm` |
| Arch | `rustmagic-<v>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./rustmagic-<v>-1-x86_64.pkg.tar.zst` |
| Arch, from source | `PKGBUILD` + `rustmagic.install` + `rustmagic-<v>.tar.gz` | `makepkg -si` in that directory |

### Inside the Linux tarball

```
rustmagic-<v>-linux-x86_64/
  bin/rustmagic            the window
  bin/rustmagic-cli        the terminal front end
  bin/rustmagic-ibus       the process IBus starts
  share/ibus/component/rustmagic.xml
  share/applications/rustmagic.desktop
  share/icons/hicolor/scalable/apps/rustmagic.svg
  share/icons/hicolor/scalable/apps/rustmagic-ibus.svg
  lib/libkm_ffi.so         the C ABI, for other input-method frameworks
  include/rustmagic.h      its header
  keyboards/*.km2          the five bundled layouts, for other tools to read
  INSTALL
```

It is a prefix: `tar xf … && cp -r rustmagic-<v>-linux-x86_64/. ~/.local/`.
The `.deb` and the `.rpm` place the same tree under `/usr`; `libkm_ffi.so` goes
to `/usr/lib/rustmagic/` rather than `/usr/lib/` because it carries no soname and
has no business being on the loader's search path. `docs/LINUX.md` covers the
IBus registration in full.

### Inside the macOS bundle

`RustMagic.app` (a universal binary, `Contents/MacOS/RustMagic`, `Info.plist`,
`PkgInfo`, `Resources/RustMagic.icns` built from the brand SVG with `sips` and
`iconutil`), plus `bin/` with both universal Mach-O binaries and
`lib/libkm_ffi.dylib`. The bundle is unsigned: on first launch, right-click and
choose Open. `tools/package/dmg.sh` wraps it in a volume with an `Applications`
link so the install is a drag.

### Inside the Windows installers

The `.exe` (Inno Setup) and the `.msi` (WiX) install the same payload:
`rustmagic.exe`, `rustmagic-cli.exe`, `km_ffi.dll`, `rustmagic.h`,
`keyboards\*.km2`, `README.md`, `LICENSE`. The portable zip is that set as files,
with `INSTALL.txt` added. Neither installer is signed.

### On Arch

Two ways, both from the same release:

```sh
# the built package
sudo pacman -U rustmagic-<v>-1-x86_64.pkg.tar.zst

# or build it again from the pinned source
curl -LO https://github.com/…/releases/download/v<v>/PKGBUILD
curl -LO https://github.com/…/releases/download/v<v>/rustmagic.install
curl -LO https://github.com/…/releases/download/v<v>/rustmagic-<v>.tar.gz
makepkg -si
```

The published `PKGBUILD` is named `PKGBUILD`, because that is the name `makepkg`
looks for and renaming it is the first thing a user hits. Its `source=()` names
the tarball in the same directory, and its `sha256sums=` is the checksum of that
tarball, so `makepkg -si` works offline from the download folder and then again
on the next machine. `check()` leaves out `tools/check_repo.py`: it inspects the
working tree, and a release tarball is not a git checkout. Both `-debug` and the
debug symbols are switched off, so there is one package and not two.

## What a release does *not* contain

Being explicit here is the point of a release note:

- **No Windows text service.** There is no TSF service yet, so on Windows the
  window, the CLI and the C ABI work; typing into other applications with a
  layout does not.
- **No macOS input method.** There is no `InputMethodKit` bundle yet; the same
  three things work and system-wide typing is the next piece of work.
- **No signing.** No Apple notarisation, no Authenticode, no GPG-signature on the
  packages. The `.sha256` files let you check what you downloaded, nothing more.
- **No `crates.io`.** Every crate sets `publish = false`; the engine is consumed
  as a path dependency or through the C ABI.
- **No cross-built Linux ARM or musl.** One linux job, one target.

## Checking a release by hand

```sh
sha256sum -c rustmagic-0.2.0-linux-x86_64.tar.gz.sha256
tar xf rustmagic-0.2.0-linux-x86_64.tar.gz
./rustmagic-0.2.0-linux-x86_64/bin/rustmagic-cli type "mingalaba"
```

On any platform, the shared library is worth exercising from C once, because it
is the one interface a foreign compiler has to accept:

```sh
cargo xtask abi     # compiles tools/smoke/abi.c against the header and runs it
```
