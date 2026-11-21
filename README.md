# RustMagic

A keyboard layout engine, in Rust, with its own window.

RustMagic is a port of [KeyMagic](https://github.com/thantthet/keymagic) rebuilt
around one rule: **the engine is a library and everything else is a front end.**
It reads the same `.km2` keyboards, compiles the same `.kms` sources, matches the
same rules with the same semantics, and does all of that without a window, a
platform SDK or a build-time parser generator.

```sh
cargo run -p rustmagic          # the window
cargo run -p rustmagic-cli -- type "mingalaba"

# Arch Linux: build and install everything, including the IBus engine
cd packaging/arch && makepkg -si
```

## What it does

- **Reads and writes `.km2`** — versions 1.3, 1.4 and 1.5, byte-verified against
  files the original compiler produced.
- **Compiles `.kms`** — variables, back-references, `[*]`, `[^]`, `ANY`, switches,
  virtual keys, modifiers, metadata: every construct the original accepts, with
  line-and-column diagnostics.
- **Matches like the original** — specificity ordering, tail matching, switch
  state, smart backspace, the twenty-round recursion cap, and the option set
  (`TRACK_CAPSLOCK`, `SMART_BACKSPACE`, `EAT_ALL_UNUSED_KEYS`, `US_LAYOUT_BASED`,
  `POSITIONAL_LAYOUT`, `TREAT_CTRL_ALT_AS_RALT`).
- **Hands the host an edit, not a document** — `Edit { erase, insert }` at the
  caret is the entire integration surface.
- **Drives every front end from one session** — the window, the CLI and the tests
  call the same commands, so they cannot drift.
- **Speaks C** — `km-ffi` exposes the ABI platform input methods already call.
- **Is an input method on Linux** — `km-ibus` is an IBus engine that speaks the
  D-Bus protocol directly, with no glib and no libibus bindings, and it ships an
  Arch package.
- **Builds for all four platforms from one tag** — Linux tarball, universal macOS
  app, Windows zip and an Arch package, every one of them tested before upload.

## The crates

| Crate | Responsibility |
| --- | --- |
| `km-core` | virtual keys, modifiers, options, metadata, errors |
| `km-binary` | the `.km2` container, read and write |
| `km-script` | the `.kms` language: lexer, parser, compiler |
| `km-engine` | rules, matching, composition, edits |
| `km-store` | installed keyboards and `settings.toml` |
| `km-session` | the application without a window: commands, events, state |
| `km-ffi` | the C ABI |
| `km-ibus` | the IBus engine: the session over D-Bus |
| `ui-egui` | the window, drawn with egui |
| `apps/rustmagic` | the desktop app |
| `apps/rustmagic-cli` | the terminal front end |
| `apps/rustmagic-ibus` | the process IBus starts |

Dependencies point one way only, and `tools/check_repo.py` enforces it.

## The demo keyboard

`assets/keyboards/demo-greek.kms` ships with the app — a short, readable layout
that shows the parts of the language worth seeing:

```
$latin = "abgdezhqiklmnxoprstufcwy"
$lower = "αβγδεζηθικλμνξοπρστυφχψω"

$latin[*] => $lower[$1]              // position picked the letter
<VK_CAPITAL> => ('caps')             // caps arms…
('caps') + $latin[*] => $upper[$1]   // …for exactly one key
```

## Releases

Every platform has an installer, not a zip you have to unpack first:

| Platform | Artifact | Install with | What works today |
| --- | --- | --- | --- |
| Windows | `RustMagic-<v>-Setup-x64.exe`, `RustMagic-<v>-x64.msi` | run it, or `msiexec /i` | window, CLI, C ABI |
| macOS | `RustMagic-<v>-universal.dmg` | open, drag to Applications | window, CLI, C ABI (unsigned) |
| Debian, Ubuntu | `rustmagic_<v>_amd64.deb`, `…_arm64.deb` | `sudo apt install ./…deb` | window, CLI, C ABI, **IBus engine** |
| Fedora, RHEL | `rustmagic-<v>-1.x86_64.rpm` | `sudo dnf install ./…rpm` | the same |
| Arch | `rustmagic-<v>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./…zst` | the same |
| Arch, from source | `PKGBUILD`, `rustmagic.install`, `rustmagic-<v>.tar.gz` | `makepkg -si` | the same, compiled here |
| Alpine, static | `…-x86_64-musl.tar.gz`, `…-aarch64-musl.tar.gz` | unpack and run | the same, statically linked |
| anywhere | `.tar.gz` / `.zip` | unpack and run | the same, no install step |

Every file carries a `.sha256` and a detached GPG signature (`.asc`), with the
public key beside them; the Windows installers are Authenticode-signed and the
macOS app notarised when the release was cut with those certificates. The engine
also lives on crates.io: `km-engine`, `km-binary`, `km-script`, `km-core` and
`km-ffi`.

`git push origin v0.2.0` builds all of them — one job per platform, the Arch one
in a container, the icons once for every installer — and publishes a single
release once every artifact is present.
System-wide typing works on all three: IBus on Linux, a TSF text service on
Windows (`platform/windows/tsf`) and an InputMethodKit bundle on macOS
(`platform/macos/ime`). Each one is selectable in its system's own keyboard
settings and types in other applications; the two INSTALL files say how they are
installed and what a test can and cannot prove about them.

## Working on it

```sh
cargo xtask check       # fmt, clippy, tests, python gates
cargo xtask abi         # compile and run the C example against the header
python3 tools/install_hooks.py
cargo run -p rustmagic-cli -- layout
```

`AGENTS.md` holds the rules for changing this repository; `docs/` holds the
architecture, the engine contract, the porting record, the workflow, the
[Linux guide](docs/LINUX.md) and [how a release is cut](docs/RELEASING.md).

## Licence

GPL-2.0-or-later, the same licence as the original. See `LICENSE`.
