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
| `linux-cross` | ubuntu-latest, `cross` | `rustmagic_<v>_arm64.deb`, `rustmagic-<v>-linux-aarch64.tar.gz`, `…-x86_64-musl.tar.gz`, `…-aarch64-musl.tar.gz` | `cross build` for each target, and `cross test` under qemu for the GNU one |
| `rpm` | ubuntu-latest in a `fedora:latest` container | `rustmagic-<v>-1.x86_64.rpm` | `rpmbuild` exits zero and `rpm -qlp` lists the binaries and the five keyboards |
| `macos` | macos-latest | `RustMagic-<v>-universal.dmg` (+ `.sha256`), `rustmagic-<v>-macos-universal.zip` | `cargo test`, `lipo -archs` on each binary, then `hdiutil verify` on the disk image |
| `windows` | windows-latest | `RustMagic-<v>-Setup-x64.exe`, `RustMagic-<v>-x64.msi`, `rustmagic-<v>-windows-x86_64.zip` (+ `.sha256`) | `cargo test`, then Inno Setup and WiX both compile the installers |
| `crates` | ubuntu-latest | `km-core`, `km-binary`, `km-script`, `km-engine`, `km-ffi` on crates.io | `cargo test` for the five, then `tools/publish_crates.py` publishes them in dependency order and waits for each to appear in the index |
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
| Debian, Ubuntu (x86-64) | `rustmagic_<v>_amd64.deb` | `sudo apt install ./rustmagic_<v>_amd64.deb` |
| Debian, Ubuntu (ARM64) | `rustmagic_<v>_arm64.deb` | `sudo apt install ./rustmagic_<v>_arm64.deb` |
| Fedora, RHEL | `rustmagic-<v>-1.x86_64.rpm` | `sudo dnf install ./rustmagic-<v>-1.x86_64.rpm` |
| Arch | `rustmagic-<v>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./rustmagic-<v>-1-x86_64.pkg.tar.zst` |
| Arch, from source | `PKGBUILD` + `rustmagic.install` + `rustmagic-<v>.tar.gz` | `makepkg -si` in that directory |
| Alpine, static | `rustmagic-<v>-linux-x86_64-musl.tar.gz` | unpack and run; statically linked, so any libc |
| ARM64, static | `rustmagic-<v>-linux-aarch64-musl.tar.gz` | the same |

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

## Signing

Three mechanisms, each with its own secret, and each degrading honestly when the
secret is absent: the release is then unsigned, and the notes say so instead of
pretending.

| What | Secret | Effect |
| --- | --- | --- |
| every file | `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE` | a detached `<file>.asc`, a `SHA256SUMS`, a signed `SHA256SUMS.asc`, and the public key as `rustmagic-signing-key.asc` |
| Windows installers | `WINDOWS_CERTIFICATE_PFX` (base64), `WINDOWS_CERTIFICATE_PASSWORD` | `signtool sign` with SHA-256 and a DigiCert timestamp, on the binaries and then on the `.exe` and `.msi` |
| macOS app | `APPLE_CERTIFICATE_P12` (base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_APP_PASSWORD`, `APPLE_TEAM_ID` | `codesign` with the hardened runtime, `notarytool submit --wait`, and `stapler staple` |

`tools/package/sign.sh` is the GPG half on its own, so the same signature can be
made outside CI:

```sh
gpg --export-secret-keys --armor "$KEY" > key.asc
GPG_PRIVATE_KEY="$(cat key.asc)" tools/package/sign.sh sign dist/*
tools/package/sign.sh verify dist/*.tar.gz   # with the key imported
```

The signatures are made in the `publish` job, because it is the only job that can
see the whole draft. A signature per platform job would mean five keys to
collect before anyone could check one file.

## crates.io

The engine is five crates — `km-core`, `km-binary`, `km-script`, `km-engine`,
`km-ffi` — and the `crates` job publishes them on every tag, in dependency
order, with `tools/publish_crates.py`. It needs `CARGO_REGISTRY_TOKEN`; without
it the job fails rather than skipping, because a release that quietly publishes
nothing is how a version number goes missing.

The rest of the workspace stays unpublished on purpose: `km-store` embeds the
keyboard files that ship with the desktop build, `km-session` owns the app's
on-disk layout, `km-ibus` speaks D-Bus to a Linux session, and the window and
the three binaries are an application, not a library.

A crate is never republished: the script skips a version the index already has,
so re-running a half-finished release finishes the rest instead of failing.

## The platform input methods, and what a release proves about them

Both of them ship now. They are the pieces that make a keyboard a keyboard rather
than a preview window with a layout in it:

- **Windows**, `platform/windows/tsf`: a TSF text service,
  `RustMagicTSF.dll`. `DllRegisterServer` writes the CLSID, registers the
  language profile through `ITfInputProcessorProfiles` and registers the service
  under `GUID_TFCAT_TIP_KEYBOARD`, the immersive and systray capability
  categories and the display attribute provider category — which is what puts
  RustMagic in Settings and the language bar and lets packaged apps use it. Keys
  arrive through `ITfKeyEventSink`, are answered by the engine, and are shown as
  a composition with a display attribute until a key ends the syllable. The
  installer registers it with `regsvr32`, so there is no separate step.
- **macOS**, `platform/macos/ime`: an `InputMethodKit` bundle,
  `RustMagicIME.app`. `Info.plist` declares the connection name, the controller
  class, the script and the language; the controller receives `handle(_:client:)`
  and drives marked text through the client. The .dmg carries the bundle and a
  one-line script that installs it into `~/Library/Input Methods`.

The CI jobs named "the keyboard speaks TSF" and "the keyboard speaks
InputMethodKit" build them and then ask the operating system, not the source
tree, whether they are real: the Windows job checks the `InprocServer32` path,
the keyboard category and the language profile through the TSF APIs and
activates the service inside a live thread manager; the macOS job checks the
bundle, its signature and its `Info.plist`, installs it, and queries Text Input
Source Services for the category, the language, whether it enables and whether it
selects.

Neither job types a character, because there is no focused text field on a
runner to type into. That part stays a human check, and
`platform/windows/tsf/INSTALL.txt` and `platform/macos/ime/INSTALL.txt` list
exactly which checks those are, so a release note never claims more than the
build proved.

Everything else on the list below is now in the release: installers for every
platform, ARM64 and musl builds, GPG signatures, Authenticode and notarisation
when certificates are configured, and the engine on crates.io.

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
