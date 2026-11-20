# Agent workflow

The repository is built so that an agent — or a person in a hurry — can find out
what is true without guessing. Every check below is a command that prints a
verdict, and none of them need a window.

## The one command

```sh
cargo xtask check
```

Runs, in order: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace`, `python3 tools/check_repo.py`, and the
documentation link check. Anything it prints with a `✗` is a failure.

## Tasks

| Task | What it does |
| --- | --- |
| `cargo xtask fmt` | formats the workspace |
| `cargo xtask test` | `cargo test --workspace` |
| `cargo xtask clippy` | the lint gate on its own |
| `cargo xtask gates` | the python gates only (instant, no build) |
| `cargo xtask commit "…"` | checks a candidate commit subject and body |
| `cargo xtask demo` | compiles the bundled keyboard and prints its rules |
| `cargo xtask hooks` | installs the git hooks |
| `tools/smoke/ibus.sh` | drives the IBus engine over a private bus with a C client |

## The python gates

They are small on purpose: they check the things a compiler cannot see.

`tools/check_repo.py` — the repository's shape.

| Rule | Meaning |
| --- | --- |
| `unsafe` | only `crates/km-ffi/src/lib.rs` may contain it |
| `layers` | no crate depends on one above it; no engine crate depends on egui |
| `placeholders` | no `//! Placeholder.`, no `fn main() {}`, no file that is a stub |
| `docs` | the documents exist, are not thin, and their links resolve |
| `manifests` | every crate has a description, licence, repository and `publish = false` |
| `tools` | every script has a shebang and a `main()`; every hook is a shell script |
| `workspace` | every workspace member has a manifest |
| `fixtures` | the test keyboards and the shipped keyboard are present |

`tools/check_commit_msg.py` — the commit rules. Subject at most 50 characters,
imperative, lowercase after the area prefix, no trailing full stop, body wrapped
at 72 after a blank line, no `wip`.

`tools/install_hooks.py` — copies `tools/hooks` into `.git/hooks` and makes them
executable. `commit-msg` runs the subject check; `pre-commit` runs the repo gates
and the fast tests.

Both gates take flags that make them usable from an editor or another agent:
`check_repo.py --list` prints rule names, `--json` prints a machine report, and
`check_commit_msg.py -` reads the message from standard input.

## The CLI is the debugging surface

```sh
cargo run -p rustmagic-cli -- type "mingalaba"          # in, out, per-key log
cargo run -p rustmagic-cli -- layout                     # every key cap
cargo run -p rustmagic-cli -- info demo-greek            # metadata and rules
cargo run -p rustmagic-cli -- compile fixtures/basic.kms -o /tmp/basic.km2
cargo run -p rustmagic-cli -- verify fixtures/basic.km2   # decode + round-trip
cargo run -p rustmagic-cli -- run sandbox.type text=abc   # by command id
cargo run -p rustmagic-cli -- commands                    # the registry
```

`--state <dir>` puts the library somewhere disposable, which is what the tests
and any exploratory run should use. `--json` turns `type` into a machine-readable
one-liner.

## Driving the app without a window

`km_session::Session` needs only a `Host`, and `HeadlessHost` is one. A test or a
script can therefore do everything a user can:

```rust
let mut session = Session::open(Box::new(HeadlessHost::new("/tmp/state")));
session.execute(Command::SandboxType { text: "ab".into() });
assert_eq!(session.sandbox().text(), "αβ");
```

The window itself is tested the same way: `crates/ui-egui/tests/render.rs` runs a
real egui context headlessly over every page, every palette, an empty library and
a window narrow enough to break layouts.

## What to run before saying "done"

1. `cargo xtask check` — the whole gate.
2. If you changed a page: `cargo test -p ui-egui` (the headless renders).
3. If you changed the engine: `cargo test -p km-engine` — 23 integration tests
   plus the fuzz walk.
4. If you changed the format or the compiler: `cargo test -p km-binary -p km-script
   -p km-store`, then `verify` a fixture with the CLI.
5. If you touched the IBus engine: `dbus-run-session -- tools/smoke/ibus.sh`.
6. Report the command and its output, not a summary of it.

## Continuous integration

`.github/workflows/ci.yml` runs the same steps on Linux, macOS and Windows plus
four extra jobs: one that fails on a commit subject over 50 characters for every
commit in the push; one that builds the C ABI and links a C program against it,
because an ABI that only Rust can use is not an ABI; one that starts the IBus
engine on a private bus and checks the replies with libibus; and one that builds
everything, and validates the packaging, inside an `archlinux` container.
