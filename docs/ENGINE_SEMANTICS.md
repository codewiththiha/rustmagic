# Engine semantics

This is the contract `km-engine` keeps. Where it differs from the original C++
engine, the difference is deliberate and stated here, not discovered later.

## Input

```rust
pub struct KeyInput {
    pub key: u16,            // virtual-key code, 0 when the key has none
    pub text: Option<char>,  // the character the keyboard produced, if any
    pub modifiers: Modifiers // shift, control, alt, alt_gr, caps
}
```

A host fills this from whatever its platform gives it: a Windows `WM_KEYDOWN`, a
macOS key event, a terminal byte, a test literal such as `KeyInput::character('a')`.

## Rules and their patterns

A rule's left side is a list of patterns, tested against the **tail** of the
composition context:

| Pattern | Matches |
| --- | --- |
| `Text(String)` | that exact text, ending at the caret |
| `AnyOf(String)` | one character of the set — `$var[*]`, or `[abc]` |
| `NotAnyOf(String)` | one character not in the set — `[^abc]` |
| `Any` | one character that is printable ASCII (`0x21..=0x7D`) or any non-ASCII character (`0xFF..=0xFFFF`) |
| `Key(u16)` | a virtual key held with the rule |
| `Modifier(Modifiers)` | a modifier the rule requires |

`ANY` deliberately does not match space, control characters or the Latin-1 range:
the original made the same choice so that punctuation typed with Ctrl or Alt is
never swallowed.

Virtual-key and modifier patterns do not consume a character, which is why
`<VK_CAPITAL>` can be a rule with an empty match length, and why switches cost
nothing.

## Priority

Rules are ordered once, at load time, by descending:

1. number of virtual keys matched,
2. number of switch conditions satisfied syntactically,
3. length of the text matched.

Ties keep **file order** (`sort_by` is stable). This is the one place the port
knows it differs from the original, which reversed the rule list and sorted with
an unstable comparison, so two rules of equal specificity could match in either
order. Authors write specific rules above general ones and expect that to hold.

## Matching one key

```
for rule in rules:
    if rule needs keys the event does not have      -> skip
    if rule needs modifiers the event does not have -> skip
    if a switch condition is not set                -> skip
    if the tail of the context does not end with the text patterns -> skip
    -> match
```

`Control` and `Alt` must be *asked for* by the rule: a layout that does not
mention Ctrl never sees a Ctrl key. Shift may be held without being asked, which
is what lets a lowercase row match Shift+letter when the layout has nothing
better. `AltGr` is honoured directly, and additionally when the rule asks for
`AltGr` and the event carries `control + alt` while `options.right_alt` is set
(the Windows habit of reporting AltGr that way).

If the event has a character and no rule matched:

* with `eat` (`EAT_ALL_UNUSED_KEYS`) the key is swallowed;
* otherwise the character is appended to the context and the edit reports
  `insert`, `consumed: true`, so the host shows what was typed;
* a key with no character and no rule is reported `consumed: false`, which tells
  the host to do its normal thing.

## Switches

A rule's right side may contain `(name)`, which sets a switch. Setting is
immediate and applies to later keys. A switch is cleared when the next rule with
a virtual key matches, so a one-key state is one key. A switch can re-arm itself
inside its own rule, which is how `('shape') + ANY => $1 + ('shape')` keeps a
mode alive while other rules can still see it.

Switch state is a `BTreeMap<u16, String>` keyed by the string-table id, which is
what the container stores; the name never has to be spelled again after load.

## Output and recursion

The right side produces text from `Text`, `Reference(n)` (a captured left-side
piece), `Indexed { table, source, segment }` (the n-th character of a variable
table, chosen by position in a captured segment) and `Switch`.

After a rule applies, the engine feeds the new output back through the rules when
it can still match something — a rule that emits a single ASCII printable
character cannot, and the loop ends. The cap is 20 rounds; exceeding it sets
`recursion_limit` and reports `consumed: false` so the host types the key itself
rather than losing it.

**Divergence:** the port also stops when a rule leaves the context exactly as it
found it. The original would spend all 20 rounds in that case, which makes
`ANY => $1` and any pass-through rule swallow the key. A no-progress round cannot
produce a different result, so stopping is behaviour-preserving where anything
was going to happen and a fix where nothing was.

## Backspace

`KeyInput::backspace()` is key `0x08`. If a rule matches it, the rule wins. If
not and `smart_backspace` (the file's `SMART_BACKSPACE`) is on, the engine
restores the context from before the previous key — undoing a whole composition
step instead of one character — using a history of the last 64 contexts. With the
option off, backspace deletes one character.

## Edit, not text

The engine reports a difference, not a document:

```
before: "the quick"        after: "the qük"
edit:   Edit { erase: 3, insert: "qük" }
```

The common prefix is kept, the rest is replaced. A host applies that at the caret
and stays in charge of selection, undo and scroll. That is the whole integration
surface for a text field, an IME or a terminal.

## Options

| Option | Default | Effect |
| --- | --- | --- |
| `TRACK_CAPSLOCK` | `TRUE` | caps lock participates in matching |
| `SMART_BACKSPACE` | `FALSE` | backspace restores the previous context |
| `EAT_ALL_UNUSED_KEYS` | `FALSE` | unmatched keys are swallowed |
| `US_LAYOUT_BASED` | `FALSE` | the layout is defined by US key positions |
| `POSITIONAL_LAYOUT` | `FALSE` | rules are positional rather than by character |
| `TREAT_CTRL_ALT_AS_RALT` | `TRUE` | Ctrl+Alt is read as AltGr |

## Guarantees the tests hold us to

- A truncated or corrupted `.km2` returns an error and never panics; the fuzz
  cases in `crates/km-engine/tests/keys.rs` walk byte by byte through a real
  232-rule file.
- The writer reproduces upstream bytes for the fixtures upstream produced.
- A real Zawgyi keyboard loads and types: key `1` produces `ခ`.
- Typing a document through the engine is idempotent per key: the same input
  gives the same edits, and the log matches the context.
