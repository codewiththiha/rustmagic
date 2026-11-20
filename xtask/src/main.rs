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
        "demo" => demo(),
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
