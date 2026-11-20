# Architecture

RustMagic is built around one idea: the engine is a library, and every front end
is a thin adapter around it. The original KeyMagic had the opposite shape — a C++
engine that also owned the keyboard file reading, the platform glue and, in some
ports, the window — and that is what made it hard to port, hard to test and hard
to change.

## Layers

```
              apps/rustmagic        apps/rustmagic-cli
                   │                        │
                   └──────────┬─────────────┘
                              ▼
                       crates/ui-egui  ·  crates/km-ffi
                              │              │
                              └──────┬───────┘
                                     ▼
                              crates/km-session
                              │      │      │
                     ┌────────┘      │      └────────┐
                     ▼               ▼               ▼
              crates/km-store  crates/km-engine  crates/km-core
                     │               │               │
                     └───────┬───────┴───────┬───────┘
                             ▼               ▼
                      crates/km-binary  crates/km-script
                             │               │
                             └───────┬───────┘
                                     ▼
                              crates/km-core
```

A crate may depend only on crates below it. `tools/check_repo.py` reads the
manifests and refuses a dependency that points up.

### `km-core` — values

Virtual-key constants, the modifier bitfield, layout options, keyboard metadata,
hotkey parsing, and the one `Error` type. No I/O, no dependencies beyond `std`.
Everything else agrees on these types, so this is where a change is cheapest to
make and most expensive to get wrong.

### `km-binary` — the container

The `.km2` file: a header, three tables (strings, infos, rules) and a rule stream
of little-endian `u16` opcode words. Reading accepts versions 1.3 to 1.5 and
normalises them; writing always emits 1.5. The decoder is written to be safe on
hostile input — every length is checked against the remaining bytes before it is
used, and a truncated file becomes an `Error`, never a panic.

### `km-script` — the language

The `.kms` source format: an `@OPTION` header, `$variable` declarations, and
rules of the form `left => right`. A hand-written lexer and a recursive-descent
parser replace the original bison grammar; the compiler walks the tree and emits
the same opcode stream the C++ compiler did, so a file compiled here loads in the
original app and the reverse.

### `km-engine` — the rules

A `Keyboard` decodes once into `Vec<Rule>`, each rule a left-hand `Vec<Pattern>`
and a right-hand `Vec<Emit>`. An `Engine` holds the composition context, the
switch state and a short history, and turns a `KeyInput` into an `Outcome`:

```rust
pub struct Outcome {
    pub consumed: bool,      // the engine handled the key
    pub edit: Edit,          // erase N chars before the caret, insert the rest
    pub rule: Option<usize>, // which rule matched, for the log
    pub recursion_limit: bool,
}
```

`Edit` is the whole host contract. A front end applies it to whatever buffer it
owns — a text field, an IME composition, a terminal — and never needs to know how
the text got that way.

### `km-store` — persistence

Two small concerns that both outlive a run: a directory of installed keyboards
(`<state>/library/keyboards/*.km2` plus the `.kms` they came from) and
`settings.toml`. Both write through a temporary file and a rename, so a crash
mid-write leaves the old value rather than half of the new one. Library ids are
sanitised to lowercase alphanumerics and dashes, which makes a path-traversing
name impossible rather than merely unlikely.

### `km-session` — the application

`Session` is the only owner of mutable state: the library, the settings, the
active keyboard, the sandbox buffer, the route, the key log and the pending
events. It exposes:

- `execute(Command)` — the single door for every change;
- `execute_id(id, params)` — the same door, by registered name;
- `drain() -> Vec<Event>` — what happened since the last frame;
- `seed*`, getters, and nothing that lets a caller mutate state behind its back.

Because a front end cannot reach inside, the CLI and the window cannot disagree
about the engine's behaviour: they are both calling the same session.

### Front ends

`ui-egui` draws a `Session` and sends `Command`s. It owns the fleeting window
state only (search text, open palette, toasts) and no engine knowledge.

`km-ffi` is the same idea for languages that cannot call Rust: opaque handles,
UTF-8 in and out, caller-owned strings, neutral answers for null pointers.

`apps/rustmagic` is an eframe shell plus a `Host` implementation: real state
directories per platform, a clipboard handed to egui on the next frame, an
external opener per platform, and the bundled demo keyboard installed on first
run.

`apps/rustmagic-cli` builds the same session on a `HeadlessHost` and prints. It is
how a bug report can say exactly what it typed, and how an agent checks a layout
without a screen.

## A key press, end to end

```
host keystroke
   → (front end) KeyInput { key, text, modifiers }
   → (km-session) Command::SandboxKey / Engine::key
   → (km-engine) try rules in priority order against the context tail
        match → apply emits → diff the context → Edit { erase, insert }
        recurse while the output can still match, capped
   → Outcome
   → (front end) apply the edit to its buffer, log it, repaint
```

Nothing in that path allocates a keyboard, a rule or a whole context per key
beyond the edit itself, which is the difference the port is buying.

## Deliberate differences

| Original | Here | Why |
| --- | --- | --- |
| bison/flex grammar | hand-written parser | readable diagnostics with lines, no build-time tool |
| engine owns platform code | `Host` trait | the engine runs on a phone, a test and a desktop unchanged |
| rules reversed in place | stable sort by specificity | equal rules keep file order, which is what authors expect |
| 20 match rounds then fail | same cap, plus a no-progress stop | `ANY => $1` no longer swallows the key |
| settings GUI in C++ | `km-session` + `ui-egui` | one owner, testable without a window |

The engine-level consequences are written down in
[ENGINE_SEMANTICS.md](ENGINE_SEMANTICS.md); the port's history is in
[PORTING.md](PORTING.md).
