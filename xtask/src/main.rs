//! Repository chores, in one place with one spelling.
//!
//! `cargo xtask <task>` is the whole interface. Each task is a `Command` run
//! from the workspace root, so what CI runs and what a developer runs are the
//! same lines of code rather than two files that drift.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
cargo xtask <task>

  check     fmt, clippy, tests, python gates — what CI runs
  fmt       format the workspace
  test      run the workspace tests
  clippy    the lint gate on its own
  gates     the python gates only, no build
  commit    check a candidate commit subject: cargo xtask commit \"...\"
  demo      compile the bundled keyboard and print its rules
  hooks     install the git hooks
  abi       compile and run the C ABI example
  release   bump to a version, gate, commit and tag
            cargo xtask release 0.2.0 [--dry-run]
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(task) = args.first() else {
        print!("{USAGE}");
        return Ok(());
    };
    let rest = &args[1..];
    match task.as_str() {
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Ok(())
        }
        "fmt" => cargo(&["fmt", "--all"]),
        "test" => cargo(&["test", "--workspace"]),
        "clippy" => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        "gates" => {
            python(&["tools/check_repo.py"])?;
            Ok(())
        }
        "hooks" => python(&["tools/install_hooks.py"]),
        "commit" => {
            let message = rest.join(" ");
            if message.is_empty() {
                return Err("give the subject to check".to_owned());
            }
            let mut child = Command::new("python3")
                .args(["tools/check_commit_msg.py", "-"])
                .current_dir(root())
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|error| format!("cannot run python3: {error}"))?;
            {
                use std::io::Write;
                let stdin = child.stdin.as_mut().ok_or_else(|| "no stdin".to_owned())?;
                stdin
                    .write_all(message.as_bytes())
                    .map_err(|error| error.to_string())?;
            }
            let status = child.wait().map_err(|error| error.to_string())?;
            if status.success() {
                println!("the subject is fine");
                Ok(())
            } else {
                Err("the subject is too long or malformed".to_owned())
            }
        }
        "abi" => shell(&["tools/smoke/abi.sh"]),
        "demo" => demo(),
        "release" => release(rest),
        "check" => {
            cargo(&["fmt", "--all", "--check"])?;
            cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])?;
            cargo(&["test", "--workspace"])?;
            python(&["tools/check_repo.py"])?;
            println!("\neverything is green");
            Ok(())
        }
        other => Err(format!("`{other}` is not a task; try `cargo xtask help`")),
    }
}

/// Cuts a release: bump every version, gate the tree, commit, tag.
///
/// Pushing the tag is left to the person doing the release, because that is the
/// step that publishes to the world and it should be deliberate.
fn release(args: &[String]) -> Result<(), String> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let Some(version) = args.iter().find(|arg| !arg.starts_with("--")) else {
        return Err("give a version, for example `cargo xtask release 0.2.0`".to_owned());
    };
    if !version
        .trim_start_matches(|char: char| char.is_ascii_digit() || char == '.')
        .is_empty()
    {
        return Err(format!("`{version}` is not a bare version such as 0.2.0"));
    }
    if !dry_run
        && (!status("git", &["diff", "--quiet"])
            || !status("git", &["diff", "--cached", "--quiet"]))
    {
        return Err("the working tree has changes; commit them first".to_owned());
    }

    // Four places carry the version, and each is anchored to something that
    // occurs once, so an unrelated number in the same file is left alone.
    let bumped = [
        root().join("Cargo.toml"),
        root().join("packaging/arch/PKGBUILD"),
        root().join("packaging/macos/Info.plist"),
        root().join("assets/ibus/rustmagic.xml"),
    ];
    bump_after(
        &bumped[0],
        "[workspace.package]",
        "version = \"",
        "\"",
        version,
    )?;
    bump_after(&bumped[1], "pkgname=rustmagic", "pkgver=", "\n", version)?;
    bump_after(
        &bumped[2],
        "<key>CFBundleShortVersionString</key>",
        "<string>",
        "</string>",
        version,
    )?;
    bump_all(&bumped[3], "<version>", "</version>", version)?;
    println!("version is {version}");

    if dry_run {
        println!("--dry-run: this is what changes, and it has been put back");
        let _ = Command::new("git")
            .arg("diff")
            .arg("--")
            .args(&bumped)
            .current_dir(root())
            .status();
        let mut restore = vec!["checkout".to_owned(), "--".to_owned()];
        restore.extend(bumped.iter().map(|path| path.display().to_string()));
        let restore: Vec<&str> = restore.iter().map(String::as_str).collect();
        return git(&restore);
    }

    run_task("check")?;
    git(&["add", "-A"])?;
    git(&["commit", "-m", &format!("release: {version}")])?;
    git(&[
        "tag",
        "-a",
        &format!("v{version}"),
        "-m",
        &format!("RustMagic {version}"),
    ])?;
    println!("\ntagged v{version}. To publish:");
    println!("  git push origin main --tags");
    println!("\nCI then builds Linux, macOS, Windows and the Arch package, and");
    println!("publishes them as one release.");
    Ok(())
}

/// Replaces the value between `open` and `close`, searching only after `anchor`.
fn bump_after(
    path: &std::path::Path,
    anchor: &str,
    open: &str,
    close: &str,
    version: &str,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let Some(anchor_at) = text.find(anchor) else {
        return Err(format!("no {anchor} in {}", path.display()));
    };
    let head = &text[..anchor_at];
    let tail = &text[anchor_at..];
    let Some(open_at) = tail.find(open) else {
        return Err(format!("no {open} after {anchor} in {}", path.display()));
    };
    let value_at = open_at + open.len();
    let Some(close_at) = tail[value_at..].find(close) else {
        return Err(format!("no {close} after {open} in {}", path.display()));
    };
    let changed = format!(
        "{head}{}{open}{version}{close}{}",
        &tail[..open_at],
        &tail[value_at + close_at + close.len()..]
    );
    std::fs::write(path, changed).map_err(|error| error.to_string())
}

/// Replaces the value between every `open` and its following `close`.
fn bump_all(path: &std::path::Path, open: &str, close: &str, version: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut out = String::with_capacity(text.len());
    let mut replaced = 0usize;
    let mut rest = text.as_str();
    while let Some(open_at) = rest.find(open) {
        let value_at = open_at + open.len();
        let Some(close_at) = rest[value_at..].find(close) else {
            break;
        };
        out.push_str(&rest[..open_at]);
        out.push_str(open);
        out.push_str(version);
        out.push_str(close);
        rest = &rest[value_at + close_at + close.len()..];
        replaced += 1;
    }
    out.push_str(rest);
    if replaced == 0 {
        return Err(format!("no {open} in {}", path.display()));
    }
    std::fs::write(path, out).map_err(|error| error.to_string())
}

/// Runs a task by name, as if it had been asked for on the command line.
fn run_task(name: &str) -> Result<(), String> {
    run(&[name.to_owned()])
}

fn status(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .current_dir(root())
        .status()
        .is_ok_and(|status| status.success())
}

fn git(args: &[&str]) -> Result<(), String> {
    if status("git", args) {
        Ok(())
    } else {
        Err(format!("git {} failed", args.join(" ")))
    }
}

/// Compiles the shipped keyboard and lists what it decided.
fn demo() -> Result<(), String> {
    let source = root().join("assets/keyboards/demo-greek.kms");
    let text = std::fs::read_to_string(&source).map_err(|error| error.to_string())?;
    let compiled =
        km_script::compile(&text, "demo-greek", None).map_err(|error| error.to_string())?;
    println!(
        "{} rules compiled from {}",
        compiled.stats.rules,
        source.display()
    );
    for warning in &compiled.warnings {
        println!(
            "warning {}:{}: {}",
            warning.line, warning.column, warning.message
        );
    }
    let bytes = compiled.to_bytes().map_err(|error| error.to_string())?;
    println!("{} bytes of .km2", bytes.len());
    let keyboard = km_engine::Keyboard::from_bytes(&bytes).map_err(|error| error.to_string())?;
    for rule in &keyboard.rules {
        println!("  {}", rule.describe());
    }
    Ok(())
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn cargo(args: &[&str]) -> Result<(), String> {
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
}

fn shell(args: &[&str]) -> Result<(), String> {
    let status = Command::new("sh")
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|error| format!("cannot run sh: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed", args.join(" ")))
    }
}

fn python(args: &[&str]) -> Result<(), String> {
    let status = Command::new("python3")
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|error| format!("cannot run python3: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed", args.join(" ")))
    }
}
