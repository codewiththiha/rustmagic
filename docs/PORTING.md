# Porting notes

What this repository took from KeyMagic, what it left behind, and what has to
stay byte-compatible.

## The two originals

| Repository | What it is | What we took |
| --- | --- | --- |
| [`thantthet/keymagic`](https://github.com/thantthet/keymagic) | the C++ original: engine, bison/flex compiler, Windows TSF, IBus, macOS | the file format, the layout language, the matching semantics, the option set, the hotkey grammar |
| [`thantthet/keymagic-3`](https://github.com/thantthet/keymagic-3) | a partial Rust rewrite with `docs/ENGINE_LOGIC.md`, `KM2_FORMAT.md`, `KMS_FORMAT.md` | the C ABI surface other platforms already call, and the confirmation that the container layout below is right |

Neither was copied module for module. The C++ engine's structure is a consequence
of `KeyMagicEngine` owning everything; reproducing that in Rust would have been
C++ written in Rust syntax. The Rust port had good instincts but kept the same
shape, cloned rules per keypress and reached for `unsafe` to reinterpret the
header.

## The `.km2` container

Verified by decoding upstream files and by byte-comparing what this repository
writes with what the upstream compiler wrote.

```
"KMKL"                     magic
u8  major (= 1)            major version
u8  minor (3 | 4 | 5)      minor version: 1.3, 1.4, 1.5
i16 stringCount            little-endian
i16 infoCount              present when minor >= 4
i16 ruleCount
option bytes               4 (1.3, 1.4) or 5 (1.5, adds rightAlt)
padding                   1 byte, only in 1.5
```

Header lengths follow from that: 14, 16 and 18 bytes. The tables are strings
(`i16` length, then that many UTF-16 units), infos (`i32` four-character code,
`i16` length, then bytes), and rules (`i16` count and words, twice: left side,
then right side).

Writing always emits 1.5. One byte of upstream output (the padding byte) is not
written deterministically by the original; it is normalised to zero and the
round-trip test allows that single difference.

### Opcode words

| Word | Meaning |
| --- | --- |
| `0x00F0` | string literal, followed by length and units |
| `0x00F1` | variable reference, 1-based |
| `0x00F2` | reference to a captured left-side piece, 1-based |
| `0x00F3` | predefined virtual key |
| `0x00F4` | modifier |
| `0x00F5` | any-of set |
| `0x00F6` | and (joins left-side items) |
| `0x00F7` | not-any-of set |
| `0x00F8` | the `ANY` wildcard |
| `0x00F9` | switch, followed by a 1-based string id |

### What the compiler writes

- A variable's value is raw UTF-16 units in the string table, with no `OP_STRING`.
- A literal on the left is `OP_STRING`, length, units.
- Each switch is a string-table entry holding one `'K'` (`0x4B`).
- A key on the left becomes `OP_AND` followed by `OP_PREDEFINED`.
- Modifiers become `OP_MODIFIER` plus `OP_ANYOF`/`OP_NANYOF`.
- `$n` on the right becomes `OP_REFERENCE n`; `$v[$n]` becomes
  `OP_VARIABLE id OP_MODIFIER n`.

## The `.kms` language

```
/* @NAME = "…" @DESCRIPTION = "…" @HOTKEY = "CTRL+SHIFT+M" … */

$row = "abcde"
$rowUpper = "ABCDE"

$row[*] => $rowUpper[$1]
"'" + ANY => U2019
```

Escapes are `\uXXXX`, `\xNN`, `\b`, `\t`, `\n`, `\v`, `\f`, `\r`. Comments are
`/…/` in the original and `//` or `/* … */` here; both are accepted. Options are
`NAME`, `DESCRIPTION`, `FONTFAMILY`, `ICON`, `HOTKEY`, `TRACK_CAPSLOCK`,
`EAT_ALL_UNUSED_KEYS`, `US_LAYOUT_BASED`, `POSITIONAL_LAYOUT`, `SMART_BACKSPACE`,
`TREAT_CTRL_ALT_AS_RALT`.

The compiler keeps the original's rules about what is an error and what is a
warning, because keyboards in the wild rely on them:

- `$0` is an error — there is no such capture.
- A `[*]`/`[^]` pattern indexed by an out-of-range `$n` is a warning; the text is
  compiled and the index simply matches nothing at run time.
- Virtual keys must come **last** on the left side.
- Switch conditions consume no characters.

## The Linux input method

KeyMagic's Linux build is an IBus engine written in C against glib: a GObject
subclass of `IBusEngine`, linked to the engine through the same C ABI this
repository now exposes. RustMagic keeps the behaviour and drops the C: the engine
process in `apps/rustmagic-ibus` speaks the IBus D-Bus protocol itself.

What that protocol turned out to be, taken from IBus 1.5's `src/ibusengine.c`,
`src/ibusfactory.c` and `src/ibusserializable.c`, and confirmed by serialising an
`IBusText` with libibus and comparing:

| Piece | Wire form |
| --- | --- |
| Factory object | `/org/freedesktop/IBus/Factory`, `CreateEngine(s) -> o` |
| Engine object | `/org/freedesktop/IBus/Engine/1`, interface `org.freedesktop.IBus.Engine` |
| Key event | `ProcessKeyEvent(u keyval, u keycode, u state) -> b` |
| Commit | signal `CommitText(v)` |
| Preedit | signal `UpdatePreeditText(v text, u cursor, b visible, u mode)` |
| Text payload | `v` holding `(s "IBusText", a{sv} {}, s text, v(… "IBusAttrList" …))` |

The keysym-to-virtual-key table in `crates/km-ibus/src/keymap.rs` replaces the
original's `keycode_map.c`; the composition policy (preedit while composing,
commit at a boundary) replaces its `engine.c` state machine, and is simpler
because this engine's context *is* the preedit.

Verified by `tools/smoke/ibus.sh`: a C client on a private bus asks the factory
for an engine, sends `a` and space, and decodes the committed text with libibus.

## The C ABI

`crates/km-ffi` exposes the surface the platform adapters already call, so an
existing Windows TSF service or macOS input method can link this engine with a
thin shim rather than being rewritten:

```
rm_keyboard_load / rm_keyboard_from_bytes / rm_keyboard_free
rm_keyboard_name / _description / _hotkey / _rule_count
rm_engine_new / rm_engine_free / rm_engine_reset
rm_engine_key / rm_engine_set_context / rm_engine_context
rm_string_free / rm_version
```

Strings this library returns are its own allocations and are released with
`rm_string_free`. A null handle is answered neutrally rather than dereferenced.
The `rm_edit` out-parameter carries the same `erase`/`insert` pair the Rust API
uses.

## Parity status

| Area | Status |
| --- | --- |
| `.km2` reading, 1.3–1.5 | complete, byte-verified |
| `.km2` writing | complete, emits 1.5 |
| `.kms` parsing and compiling | complete for every construct the original's compiler accepted |
| Rule matching, priorities, switches, recursion | complete, with the divergences listed in [ENGINE_SEMANTICS.md](ENGINE_SEMANTICS.md) |
| Metadata, hotkeys, icons | metadata and hotkeys complete; icon bytes are carried through unparsed |
| Linux input method (IBus) | complete: `km-ibus` plus an Arch package, driven by a libibus smoke test |
| Windows text service (TSF) | complete: `platform/windows/tsf`, registered as a keyboard TIP, driven by `keys` and not by a window |
| macOS input method (InputMethodKit) | complete: `platform/macos/ime`, an input source that marks and commits text |
| The original's settings window | replaced by `km-session` + `ui-egui` |
| Keyboard manager dialogs (hotkey capture, layout preview) | preview and hotkey display present; hotkey recording is not |

Anything in that last block is honest work remaining, not a silent gap: the
engine, the format and the language are what a port has to preserve, and they are
preserved and tested.

## Fixtures

`fixtures/` holds the files the tests use:

- `minimal.kms` / `minimal.km2` — two rules, the smallest useful file.
- `basic.kms` / `basic.km2` — fourteen rules exercising every construct the
  parser accepts: variables, references, any-of, not-any-of, `ANY`, switches,
  virtual keys, modifiers, metadata.
- `zawgyi-l-unicode.km2` — a real 232-rule Zawgyi keyboard (version 1.4, 12
  strings, 5 infos) used to prove the engine survives a real file.

`assets/keyboards/demo-greek.kms` ships with the app: a small, readable layout
that shows variables, back-references and a one-key switch.
