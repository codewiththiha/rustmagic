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

| Job | Runner | Files | Verified by |
| --- | --- | --- | --- |
| `linux` | ubuntu-22.04 | `rustmagic-<v>-linux-x86_64.tar.gz` (+ `.sha256`) | `cargo test`, then `tools/smoke/ibus.sh` over a real D-Bus session |
| `macos` | macos-14 | `rustmagic-<v>-macos-universal.zip` (+ `.sha256`) | `cargo test`, plus a `lipo` check that both slices are present |
| `windows` | windows-latest | `rustmagic-<v>-windows-x86_64.zip` (+ `.sha256`) | `cargo test` |
| `arch` | `archlinux:latest` container | `rustmagic-<v>-1-x86_64.pkg.tar.zst`, `rustmagic-<v>-PKGBUILD`, `rustmagic-<v>.SRCINFO` | `makepkg` builds the package as a non-root user, after the same tests |

The four jobs run in parallel; `publish` waits for all of them and asserts that
each expected file exists before it turns the draft release public. A job that
quietly uploads nothing therefore fails the release instead of shipping half of
one.

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
  INSTALL
```

It is a prefix: `tar xf … && cp -r rustmagic-<v>-linux-x86_64/. ~/.local/`.
`docs/LINUX.md` covers the IBus registration in full.

### Inside the macOS bundle

`RustMagic.app` (a universal binary, `Contents/MacOS/RustMagic`, `Info.plist`,
`PkgInfo`, `Resources/RustMagic.icns` built from `packaging/macos/rustmagic.png`
with `sips` and `iconutil`), plus `bin/` with both universal Mach-O binaries and
`lib/libkm_ffi.dylib`. The bundle is unsigned: on first launch, right-click and
choose Open.

### Inside the Windows zip

`rustmagic.exe`, `rustmagic-cli.exe`, `km_ffi.dll`, `rustmagic.h`,
`rustmagic.ico`, `INSTALL.txt`.

### On Arch

The download is the package itself plus the two files needed to rebuild exactly
it: the `-PKGBUILD` has the version and the checksum of that tag pinned, so
`makepkg -si` next to it reproduces the same package, and `pacman -U` installs
the `.pkg.tar.zst` directly. Both `-debug` and the debug symbols are switched
off, so there is one package and not two. `packaging/arch/PKGBUILD-git` is the
rolling alternative and is not built by CI.

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
