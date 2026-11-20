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

## Working on it

```sh
cargo xtask check       # fmt, clippy, tests, python gates
python3 tools/install_hooks.py
cargo run -p rustmagic-cli -- layout
```

`AGENTS.md` holds the rules for changing this repository; `docs/` holds the
architecture, the engine contract, the porting record, the workflow and the
[Linux guide](docs/LINUX.md).

## Licence

GPL-2.0-or-later, the same licence as the original. See `LICENSE`.
