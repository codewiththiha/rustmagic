# AGENTS.md

Instructions for anyone — human or agent — changing this repository. `CLAUDE.md`
is a byte-for-byte copy; `tools/check_repo.py` fails if the two drift, so edit
`AGENTS.md` and copy it over.

## What this is

RustMagic is a from-scratch Rust port of [KeyMagic](https://github.com/thantthet/keymagic),
a keyboard layout engine for Myanmar and other scripts. It keeps the file format,
the layout language and the semantics that existing keyboards depend on, and
throws away the parts of the original that made it hard to change: the separate
C++ engine and bison/flex compiler, the platform code welded into the engine, and
the per-keypress allocation.

The engine is the product. Everything else — a window, a CLI, a C ABI — is a
front end that owns no rules of its own.

## Repository map

```
crates/
  km-core/      values with no dependencies: virtual keys, modifiers, errors,
                metadata, hotkeys, layout options
  km-binary/    the .km2 container: read, write, verify, byte-for-byte
  km-script/    the .kms language: lexer, parser, compiler to the container
  km-engine/    matching and composing: rules, patterns, the key loop
  km-store/     the library of installed keyboards and settings.toml
  km-session/   the application without a window: state, commands, events
  km-ffi/       the C ABI for platform input-method adapters; include/ holds the
                header that a C or C++ caller compiles against
  km-ibus/      the IBus engine: the session, spoken over D-Bus
  ui-egui/      the window, drawn with egui; owns no state
apps/
  rustmagic/      the desktop app: eframe shell plus a real Host
  rustmagic-cli/  the same session, driven from a terminal
  rustmagic-ibus/ the process IBus starts
xtask/            repository chores, `cargo xtask <task>`
tools/            python gates: repo shape, commit messages, hooks
docs/             the documents listed under "Documentation"
fixtures/         keyboard files every test may rely on
assets/keyboards/ keyboards that ship with the app
assets/ibus/      the IBus component file
packaging/        Arch PKGBUILDs, desktop entry and icons, the macOS Info.plist,
                  the app icons
tools/smoke/      the C clients: one drives the IBus engine over D-Bus, one links
                  the C ABI through the installed header
.github/workflows/ci.yml on every push; release.yml on a v* tag
```

Dependencies point one way only: `km-core` at the bottom, `km-session` in the
middle, front ends at the top. A crate may never depend on a crate above it, and
no engine crate may depend on a window library. `tools/check_repo.py` enforces
the order from `Cargo.toml`, not from good intentions.

## The rules that matter

1. **The engine owns the rules.** Anything that decides what a key does belongs
   in `km-engine`. A front end that reimplements a match is a bug.
2. **State lives in one place.** `Session` owns the library, the settings, the
   buffer and the route. Front ends read it and send `Command`s; they never
   mutate what the session owns.
3. **Every action has a name.** A `Command` without a row in `SPECS` fails the
   build's own test. The CLI, the window and the tests use the same ids, so a
   feature cannot exist in only one of them.
4. **No `unsafe`, except in `crates/km-ffi/src/lib.rs`.** The workspace denies
   it; the C ABI allows it because a C boundary is what it is. Every other
   occurrence is rejected by `tools/check_repo.py`.
5. **No panics on bad input.** A malformed `.kms` or `.km2` returns an `Error`
   with a line and column, or a warning when the original tolerated it. Fuzzing
   tests feed truncated and corrupted files; they must not panic.
6. **Byte compatibility is a test, not an aspiration.** The writer must reproduce
   the upstream bytes for the fixtures that upstream produced.
7. **Comments explain why.** The code says what. A comment that restates the next
   line is noise; a comment that records a decision, a quirk of the original or
   a trap is the point.

## Rust

- Edition 2024, `rust-version = "1.85"`, resolver 2. Edition 2024 is a hard
  requirement: `#[unsafe(no_mangle)]`, `unsafe extern`, and the stricter
  `gen`/`unsafe_op_in_unsafe_fn` rules all come with it, and
  `tools/check_repo.py` rejects a crate that overrides the edition.
- `#![deny(missing_docs)]` in every crate: public items carry `///`, modules
  carry `//!`. A doc comment is part of the interface.
- Workspace lints: `unsafe_code = deny`, `missing_docs = warn`, clippy `all` and
  `pedantic` at `warn`. Do not silence one with `#[allow]` unless the reason is
  written next to it.
- Prefer owned data at the boundary and borrowed data inside. `Arc<Keyboard>` is
  shared by every engine that runs it; nothing clones a keyboard per keypress.
- Errors are a plain `enum Error` in `km-core` with `Display` and `std::error::Error`.
  No `anyhow` in libraries; a front end may add context.
- Formatting is `cargo fmt`, imports grouped by `rustfmt`, no manual alignment.
- Run `cargo xtask check` before you say you are done. It runs `fmt --check`,
  `clippy -D warnings`, `test --workspace`, and the python gates.
- `cargo xtask abi` compiles `tools/smoke/abi.c` through
  `crates/km-ffi/include/rustmagic.h`. A change to the ABI that no C program can
  call is not a change to a shared library, it is a private function.
- Releasing is `cargo xtask release <version>` and then `git push --tags`; see
  [docs/RELEASING.md](docs/RELEASING.md) for what each platform ships.

## egui

- `ui-egui` is a library, not an application: it takes `&egui::Context` and
  `&mut Session` and draws. It does not open windows, read files, or spawn
  processes. `apps/rustmagic` does that, and passes a `Host` to the session.
- Every colour, radius and font size comes from `crates/ui-egui/src/theme.rs`.
  A page that hardcodes `Color32::from_rgb` is a review comment.
- One file per page under `src/pages/`, each exposing a single `show`. A page
  draws and sends commands; it does not hold state across frames. The little
  state the window does need (search text, open palette, toasts) lives in
  `Ui` and nowhere else.
- Interactions return commands, never side effects: `if …clicked() { session.execute(…) }`.
- Every page must draw with an empty library, a narrow window and both palettes.
  `crates/ui-egui/tests/render.rs` proves it headlessly; add a case when you add
  a page.
- Keyboard input reaching the sandbox must respect `ctx().wants_keyboard_input()`
  so a text field keeps working.

## The IBus engine

- `km-ibus` is a front end like `ui-egui`: it owns no rules, only the translation
  between key events and `Session`. Its state is the preedit and the engine
  handle; nothing else.
- The D-Bus interface is IBus's, not ours. Before changing a signature, read
  `src/ibusengine.c` in IBus 1.5 and keep the argument order.
- An `IBusText` on the wire is a variant of `(sa{sv}sv)` — built in
  `crates/km-ibus/src/text.rs`. `docs/PORTING.md` records how that was verified.
- The preedit is the composition; a commit happens when the syllable ends. Adding
  a key that should end a composition means adding it to `is_terminator`.
- `tools/smoke/ibus.sh` must pass after any change to the engine: it drives the
  service over a private bus and decodes the replies with libibus.
- Never make `km-ibus` depend on glib or libibus. The whole point is that a plain
  Rust process can be an engine, which is what makes it testable.

## Documentation

- `docs/ARCHITECTURE.md` explains the layers and the data flow.
- `docs/ENGINE_SEMANTICS.md` is the contract: matching, priority, recursion,
  switches, options, and every deliberate difference from the original.
- `docs/PORTING.md` records what came from where, the `.km2` layout, the
  compiler's encoding, and the parity decisions.
- `docs/AGENT_WORKFLOW.md` describes the commands, gates and hooks.
- `docs/LINUX.md` explains the IBus integration, the Arch package, and what is
  verified where.
- A behaviour change updates the document that describes it in the same commit.
  A new divergence from the original gets a row in the table in
  `docs/ENGINE_SEMANTICS.md`.
- Public API docs carry examples where an example is shorter than a sentence.

## Commits

- Subject: **at most 50 characters**, imperative mood, lowercase after the area
  prefix, no trailing full stop.
- Prefix with the area you touched: `engine:`, `script:`, `session:`, `ui:`,
  `docs:`, `ci:`, `tools:`. The list lives in `tools/check_commit_msg.py`.
- Body: one blank line after the subject, wrapped at 72 columns. Explain what
  changed and why it had to; leave "how" to the diff.
- One logical change per commit. A reformat and a behaviour change are two.
- `tools/check_commit_msg.py` runs from the `commit-msg` hook and again in CI, so
  a long subject is refused before it exists.
- Never commit a credential. The state directory, `target/` and `.env` are
  ignored; a token pasted into a file is a leaked token.

## Working on it

```sh
cargo xtask check       # everything CI runs
cargo xtask fmt         # format
cargo xtask test        # workspace tests
cargo xtask gates       # python gates only
cargo xtask demo        # print the demo keyboard's rules
python3 tools/install_hooks.py   # once per clone
```

When you add a command: add the variant, add the `id()` arm, add the `SPECS`
row, handle it in `Session::execute`, and — if a person needs it — a button.
The registry test tells you if you missed one.

When you add a crate: add it to `Cargo.toml` members and workspace dependencies,
give it a description, `publish = false`, the workspace lints, and a row in the
layer table in `tools/check_repo.py`.

## Reporting

State what you ran and what it printed. "Tests pass" without the command is not
a report; `cargo test --workspace: 79 passed` is.
