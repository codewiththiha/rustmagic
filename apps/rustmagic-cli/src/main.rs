//! The command line, and the far end of the agent story.
//!
//! Everything the window can do, the CLI can do by name: same commands, same
//! state, one host swapped for another. It exists so a person (or a script) can
//! check a layout without opening a window, and so bug reports can say exactly
//! what they typed.

#![deny(missing_docs)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use km_core::Modifiers;
use km_session::{Command, HeadlessHost, Session, SPECS};

const USAGE: &str = "\
rustmagic-cli — the RustMagic front end for terminals

  list                     installed keyboards
  info <id|file>           metadata, rules and variables
  source <id>              the .kms the library kept
  layout [id]              what each key writes, as a table
  type <text>              type text through a keyboard and print the result
  key <vk>...              press virtual-key codes, decimal or 0x-prefixed
  compile <in.kms> [-o f]  compile a source file
  verify <file.km2>        decode a compiled keyboard and check it
  run <command> [k=v]...   run a registered command
  commands                 list registered commands
  where                    print the state directory

Options
  --board <id>             keyboard to use, defaulting to the active one
  --state <dir>            state directory, defaulting to the platform one
  --json                   machine-readable output where it matters
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (command, options) = Options::parse(args)?;
    let Some(command) = command else {
        print!("{USAGE}");
        return Ok(());
    };
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "commands" => {
            for spec in SPECS {
                let params = if spec.params.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", spec.params.join(" "))
                };
                println!("{}{params}  {}", spec.id, spec.label);
            }
            Ok(())
        }
        "where" => {
            println!("{}", options.state.display());
            Ok(())
        }
        "compile" => compile(&options),
        "verify" => verify(&options),
        "info" => info(&options),
        "list" => {
            list(&mut options.session()?);
            Ok(())
        }
        "source" => source(&options),
        "layout" => layout(&options),
        "type" => type_text(&options),
        "key" => press_keys(&options),
        "run" => run_command(&options),
        other => Err(format!("`{other}` is not a command; try `help`")),
    }
}

/// Parsed arguments.
struct Options {
    state: PathBuf,
    board: Option<String>,
    json: bool,
    rest: Vec<String>,
    output: Option<PathBuf>,
}

impl Options {
    /// Splits the arguments into the command, its parameters and the flags,
    /// wherever the flags appear on the line.
    fn parse(args: &[String]) -> Result<(Option<String>, Self), String> {
        let mut options = Self {
            state: default_state_dir(),
            board: None,
            json: false,
            rest: Vec::new(),
            output: None,
        };
        let mut command: Option<String> = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--state" => {
                    index += 1;
                    options.state = PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--state needs a directory".to_owned())?,
                    );
                }
                "--board" | "--keyboard" => {
                    index += 1;
                    options.board = Some(
                        args.get(index)
                            .ok_or_else(|| "--board needs an id".to_owned())?
                            .clone(),
                    );
                }
                "--json" => options.json = true,
                "-o" | "--output" => {
                    index += 1;
                    options.output = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "-o needs a path".to_owned())?,
                    ));
                }
                other => {
                    if command.is_none() {
                        command = Some(other.to_owned());
                    } else {
                        options.rest.push(other.to_owned());
                    }
                }
            }
            index += 1;
        }
        Ok((command, options))
    }

    fn first(&self) -> Result<&str, String> {
        self.rest
            .first()
            .map(String::as_str)
            .ok_or_else(|| "this command needs an argument; try `help`".to_owned())
    }

    /// A session on a headless host, with the demo installed if nothing is.
    fn session(&self) -> Result<Session, String> {
        std::fs::create_dir_all(&self.state).map_err(|error| error.to_string())?;
        let mut session = Session::open(Box::new(HeadlessHost::new(&self.state)));
        if session.entries().is_empty() {
            session.seed_text(
                "demo-greek",
                include_str!("../../../assets/keyboards/demo-greek.kms"),
            );
        }
        if let Some(board) = &self.board {
            session.execute(Command::SelectKeyboard { id: board.clone() });
            if session.active().map(|active| active.id.as_str()) != Some(board.as_str()) {
                return Err(format!("no keyboard called `{board}`"));
            }
        }
        Ok(session)
    }
}

fn default_state_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map_or_else(|| home.join("RustMagic"), PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.join("Library/Application Support/RustMagic")
    } else {
        std::env::var_os("XDG_STATE_HOME").map_or_else(
            || home.join(".local/state/rustmagic"),
            |base| PathBuf::from(base).join("rustmagic"),
        )
    }
}

fn resolve(options: &Options, name: &str) -> PathBuf {
    let path = Path::new(name);
    if path.exists() {
        path.to_path_buf()
    } else {
        options.state.join(name)
    }
}

fn compile(options: &Options) -> Result<(), String> {
    let input = resolve(options, options.first()?);
    let compiled = km_script::compile_file(&input).map_err(|error| error.to_string())?;
    let bytes = compiled.to_bytes().map_err(|error| error.to_string())?;
    let output = options
        .output
        .clone()
        .unwrap_or_else(|| input.with_extension("km2"));
    std::fs::write(&output, &bytes).map_err(|error| error.to_string())?;
    println!(
        "{} → {} ({} bytes, {} rules, {} strings, {} infos)",
        input.display(),
        output.display(),
        bytes.len(),
        compiled.stats.rules,
        compiled.stats.variables,
        compiled.stats.infos
    );
    for warning in &compiled.warnings {
        println!(
            "warning: {}:{}: {}",
            warning.line, warning.column, warning.message
        );
    }
    Ok(())
}

fn verify(options: &Options) -> Result<(), String> {
    let path = resolve(options, options.first()?);
    let keyboard = km_engine::Keyboard::from_file(&path).map_err(|error| error.to_string())?;
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    let raw = km_binary::read(&bytes).map_err(|error| error.to_string())?;
    let rewritten = km_binary::write(&raw).map_err(|error| error.to_string())?;
    let same = bytes.len() == rewritten.len()
        && bytes
            .iter()
            .zip(&rewritten)
            .filter(|(left, right)| left != right)
            .count()
            <= 1;
    println!(
        "{}: {} rules, {} variables, {} bytes, round-trip {} ({} warnings)",
        path.display(),
        keyboard.rule_count(),
        keyboard.variables.len(),
        bytes.len(),
        if same { "identical" } else { "differs" },
        keyboard.warnings.len()
    );
    for warning in &keyboard.warnings {
        println!("warning: {warning}");
    }
    Ok(())
}

fn info(options: &Options) -> Result<(), String> {
    let name = options.first()?;
    let path = resolve(options, name);
    let keyboard = if path.exists() {
        km_engine::Keyboard::from_file(&path).map_err(|error| error.to_string())?
    } else {
        options
            .session()?
            .keyboard(name)
            .map_err(|error| error.to_string())?
    };
    println!("name         {}", keyboard.meta.name);
    println!("description  {}", keyboard.meta.description);
    println!("hotkey       {}", keyboard.meta.hotkey);
    println!("font         {}", keyboard.meta.font_family);
    println!("version      {:?}", keyboard.version);
    println!("rules        {}", keyboard.rule_count());
    println!("variables    {}", keyboard.variables.len());
    println!(
        "options      track_caps={} smart_bksp={} eat={} positional={} right_alt={}",
        keyboard.options.track_caps,
        keyboard.options.auto_bksp,
        keyboard.options.eat,
        keyboard.options.pos_based,
        keyboard.options.right_alt
    );
    for (index, value) in keyboard.variables.iter().enumerate() {
        println!("  ${index} = {value}");
    }
    for rule in &keyboard.rules {
        println!("  {}", rule.describe());
    }
    Ok(())
}

fn list(session: &mut Session) {
    let active = session.active().map(|active| active.id.clone());
    if session.entries().is_empty() {
        println!("the library is empty");
        return;
    }
    for entry in session.entries() {
        let mark = if active.as_deref() == Some(entry.id.as_str()) {
            "*"
        } else {
            " "
        };
        println!(
            "{mark} {:<20} {:>4} rules {:>6} B  {}",
            entry.id,
            entry.rules,
            entry.size,
            entry.display_name()
        );
    }
}

fn source(options: &Options) -> Result<(), String> {
    let session = options.session()?;
    let id = match options.rest.first() {
        Some(id) => id.clone(),
        None => session
            .active()
            .map(|active| active.id.clone())
            .ok_or_else(|| "no keyboard is active".to_owned())?,
    };
    match session.library_source(&id) {
        Some(text) => {
            print!("{text}");
            Ok(())
        }
        None => Err(format!("`{id}` was installed as a binary only")),
    }
}

fn layout(options: &Options) -> Result<(), String> {
    let mut session = options.session()?;
    if let Some(id) = options.rest.first() {
        session.execute(Command::SelectKeyboard { id: id.clone() });
    }
    if session.active().is_none() {
        return Err("no keyboard is active".to_owned());
    }
    for row in session.layout() {
        let mut line = String::new();
        for cap in &row.caps {
            let _ = write!(line, "{}:{}  ", cap.label, cap.plain);
        }
        println!("{line}");
        let mut shifted = String::new();
        for cap in &row.caps {
            if cap.shifted != cap.plain {
                let _ = write!(shifted, "{}:{}  ", cap.label, cap.shifted);
            }
        }
        if !shifted.is_empty() {
            println!("  shift  {shifted}");
        }
    }
    Ok(())
}

fn type_text(options: &Options) -> Result<(), String> {
    let mut session = options.session()?;
    let text = options.rest.join(" ");
    if text.is_empty() {
        return Err("nothing to type".to_owned());
    }
    session.execute(Command::SandboxReset);
    session.execute(Command::SandboxType { text: text.clone() });
    let produced = session.sandbox().text().to_owned();
    if options.json {
        println!(
            "{{\"input\": {}, \"output\": {}, \"keys\": {}}}",
            json_string(&text),
            json_string(&produced),
            session.sandbox().log().len()
        );
    } else {
        println!("in   {text}");
        println!("out  {produced}");
        for entry in session.sandbox().log() {
            println!(
                "  {} {} -> {}{}",
                entry.label,
                if entry.consumed { "took" } else { "passed" },
                format_args!("-{}{}", entry.erase, entry.insert),
                entry
                    .rule
                    .map(|rule| format!(" (rule {rule})"))
                    .unwrap_or_default()
            );
        }
    }
    Ok(())
}

fn press_keys(options: &Options) -> Result<(), String> {
    let mut session = options.session()?;
    session.execute(Command::SandboxReset);
    for raw in &options.rest {
        let key = parse_key(raw)?;
        session.execute(Command::SandboxKey {
            key,
            modifiers: Modifiers::NONE,
        });
    }
    println!("{}", session.sandbox().text());
    Ok(())
}

fn parse_key(raw: &str) -> Result<u16, String> {
    let trimmed = raw.trim();
    let parsed = if let Some(hex) = trimmed.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<u16>()
    };
    parsed.map_err(|_| format!("`{raw}` is not a virtual-key code"))
}

fn run_command(options: &Options) -> Result<(), String> {
    let mut session = options.session()?;
    let id = options.first()?;
    let mut params: Vec<(&str, &str)> = Vec::new();
    let owned: Vec<(String, String)> = options.rest[1..]
        .iter()
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    for (key, value) in &owned {
        params.push((key.as_str(), value.as_str()));
    }
    session
        .execute_id(id, &params)
        .map_err(|error| error.to_string())?;
    for event in session.drain() {
        println!("{event:?}");
    }
    let buffer = session.sandbox().text();
    if !buffer.is_empty() {
        println!("buffer  {buffer}");
    }
    Ok(())
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for char in text.chars() {
        match char {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            char if (char as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", char as u32);
            }
            char => out.push(char),
        }
    }
    out.push('"');
    out
}
