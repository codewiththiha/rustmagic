//! Turning a [`Program`] into opcodes, the way upstream does.
//!
//! Encoding details that matter for byte-identical output:
//!
//! * A variable's value is stored as raw UTF-16 units with no `OP_STRING`
//!   header, so a variable may splice other variables into itself.
//! * Literal text inside a rule carries an `OP_STRING` header and a length in
//!   UTF-16 units.
//! * A switch is a string-table entry holding one placeholder character, so
//!   switch ids and variable ids share one index space.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use km_binary::{RawKeyboard, RawRule};
use km_core::{info_id, Error, LayoutOptions, Result};

use crate::ast::{Lhs, Modifier, Program, Rhs, Rule as AstRule, Statement, Term, VariableDecl};

/// A note about something the compiler accepted but a person should know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Source file the note belongs to.
    pub path: String,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
    /// What was found.
    pub message: String,
}

/// How much a source file declared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// String-table entries, switches included.
    pub variables: usize,
    /// Rules.
    pub rules: usize,
    /// Info entries.
    pub infos: usize,
}

/// A compiled keyboard plus what the compiler noticed on the way.
#[derive(Debug, Clone)]
pub struct Compiled {
    /// The keyboard, ready to be written or loaded.
    pub keyboard: RawKeyboard,
    /// Notes, in source order.
    pub warnings: Vec<Diagnostic>,
    /// Counts for the CLI and the GUI.
    pub stats: Stats,
}

impl Compiled {
    /// Serialises the keyboard as `.km2` bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        km_binary::write(&self.keyboard)
    }
}

/// Compiles source that came from `path`; `dir` resolves includes and icons.
pub fn compile(source: &str, path: &str, dir: Option<&Path>) -> Result<Compiled> {
    let program = crate::parser::parse(&crate::lexer::tokenize(source, path)?, path)?;
    let mut builder = Builder::new();
    builder.apply(&program, path, dir, 0)?;
    Ok(builder.finish())
}

/// Compiles a `.kms` file, resolving includes relative to it.
pub fn compile_file(path: impl AsRef<Path>) -> Result<Compiled> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|error| Error::io(path, &error))?;
    let dir = path.parent().filter(|dir| !dir.as_os_str().is_empty());
    compile(&source, &path.display().to_string(), dir)
}

/// Deepest include chain accepted, matching upstream's scanner.
const MAX_INCLUDE_DEPTH: usize = 16;

/// The placeholder upstream stores for a switch's string-table entry.
const SWITCH_PLACEHOLDER: char = 'K';

struct Builder {
    options: LayoutOptions,
    strings: Vec<Vec<u16>>,
    index: BTreeMap<String, usize>,
    infos: Vec<(i32, Vec<u8>)>,
    rules: Vec<RawRule>,
    warnings: Vec<Diagnostic>,
}

impl Builder {
    fn new() -> Self {
        Self {
            options: LayoutOptions::default(),
            strings: Vec::new(),
            index: BTreeMap::new(),
            infos: Vec::new(),
            rules: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn apply(
        &mut self,
        program: &Program,
        path: &str,
        dir: Option<&Path>,
        depth: usize,
    ) -> Result<()> {
        for statement in &program.statements {
            match statement {
                Statement::Variable(declaration) => self.variable(declaration, path)?,
                Statement::Rule(rule) => self.rule(rule, path)?,
                Statement::Option {
                    name,
                    value,
                    line,
                    column,
                } => self.option(name, value, path, *line, *column, dir),
                Statement::Include {
                    path: included,
                    line,
                    column,
                } => self.include(included, path, *line, *column, dir, depth)?,
            }
        }
        Ok(())
    }

    fn finish(self) -> Compiled {
        let stats = Stats {
            variables: self.strings.len(),
            rules: self.rules.len(),
            infos: self.infos.len(),
        };
        let warnings = self
            .warnings
            .iter()
            .map(|warning| km_binary::Warning {
                rule: None,
                message: format!("{}:{}: {}", warning.line, warning.column, warning.message),
            })
            .collect();
        Compiled {
            keyboard: RawKeyboard {
                version: km_binary::CURRENT_VERSION,
                options: self.options,
                strings: self.strings,
                infos: self.infos,
                rules: self.rules,
                warnings,
            },
            warnings: self.warnings,
            stats,
        }
    }

    fn warn(&mut self, path: &str, line: u32, column: u32, message: impl Into<String>) {
        self.warnings.push(Diagnostic {
            path: path.to_owned(),
            line,
            column,
            message: message.into(),
        });
    }

    /// Adds a string-table entry and returns its 1-based id.
    fn push_string(&mut self, name: &str, units: Vec<u16>) -> u16 {
        let position = self.strings.len();
        self.strings.push(units);
        self.index.insert(name.to_owned(), position);
        position_to_id(position)
    }

    fn variable(&mut self, declaration: &VariableDecl, path: &str) -> Result<()> {
        if self.index.contains_key(&declaration.name) {
            return Err(Error::script(
                path,
                declaration.line,
                declaration.column,
                format!("variable `${}` is already defined", declaration.name),
            ));
        }
        let mut units = Vec::new();
        for term in &declaration.value {
            match term {
                Term::Unicode(point) => push_utf16(&mut units, &point.to_string()),
                Term::Text(text) => push_utf16(&mut units, text),
                Term::Variable(other) => {
                    let index =
                        self.variable_id(other, path, declaration.line, declaration.column)?;
                    units.push(km_binary::OP_VARIABLE);
                    units.push(index);
                }
            }
        }
        self.push_string(&declaration.name, units);
        Ok(())
    }

    /// Looks up a variable's 1-based id, or reports where it was used early.
    fn variable_id(&self, name: &str, path: &str, line: u32, column: u32) -> Result<u16> {
        self.index.get(name).map_or_else(
            || {
                Err(Error::script(
                    path,
                    line,
                    column,
                    format!("variable `${name}` is used before it is defined"),
                ))
            },
            |position| Ok(position_to_id(*position)),
        )
    }

    /// Returns a switch's id, creating its placeholder entry on first sight.
    fn switch_id(&mut self, name: &str) -> u16 {
        match self.index.get(name) {
            Some(position) => position_to_id(*position),
            None => self.push_string(name, vec![SWITCH_PLACEHOLDER as u16]),
        }
    }

    fn rule(&mut self, rule: &AstRule, path: &str) -> Result<()> {
        self.validate(rule, path)?;
        let mut lhs = Vec::new();
        for item in &rule.lhs {
            self.encode_lhs(item, &mut lhs, path, rule)?;
        }
        let mut rhs = Vec::new();
        for item in &rule.rhs {
            self.encode_rhs(item, &mut rhs, path, rule)?;
        }
        let index = self.rules.len();
        self.rules.push(RawRule { index, lhs, rhs });
        Ok(())
    }

    fn encode_lhs(
        &mut self,
        item: &Lhs,
        out: &mut Vec<u16>,
        path: &str,
        rule: &AstRule,
    ) -> Result<()> {
        match item {
            Lhs::Text(text) => push_literal(out, text),
            Lhs::Any => out.push(km_binary::OP_ANY),
            Lhs::Key(name) => {
                out.push(km_binary::OP_PREDEFINED);
                out.push(self.predefined(name, path, rule)?);
            }
            Lhs::Variable { name, modifier } => {
                out.push(km_binary::OP_VARIABLE);
                out.push(self.variable_id(name, path, rule.line, rule.column)?);
                match modifier {
                    Modifier::AnyOf => {
                        out.push(km_binary::OP_MODIFIER);
                        out.push(km_binary::OP_ANYOF);
                    }
                    Modifier::NotAnyOf => {
                        out.push(km_binary::OP_MODIFIER);
                        out.push(km_binary::OP_NANYOF);
                    }
                    Modifier::None | Modifier::Index(_) => {}
                }
            }
            Lhs::Switch(name) => {
                out.push(km_binary::OP_SWITCH);
                out.push(self.switch_id(name));
            }
            Lhs::Keys(keys) => {
                out.push(km_binary::OP_AND);
                for key in keys {
                    out.push(km_binary::OP_PREDEFINED);
                    out.push(self.predefined(key, path, rule)?);
                }
            }
        }
        Ok(())
    }

    fn encode_rhs(
        &mut self,
        item: &Rhs,
        out: &mut Vec<u16>,
        path: &str,
        rule: &AstRule,
    ) -> Result<()> {
        match item {
            Rhs::Text(text) => push_literal(out, text),
            Rhs::Reference(index) => {
                out.push(km_binary::OP_REFERENCE);
                out.push(*index);
            }
            Rhs::Key(name) => {
                out.push(km_binary::OP_PREDEFINED);
                out.push(self.predefined(name, path, rule)?);
            }
            Rhs::Switch(name) => {
                out.push(km_binary::OP_SWITCH);
                out.push(self.switch_id(name));
            }
            Rhs::Variable { name, index } => {
                out.push(km_binary::OP_VARIABLE);
                out.push(self.variable_id(name, path, rule.line, rule.column)?);
                if let Some(index) = index {
                    out.push(km_binary::OP_MODIFIER);
                    out.push(*index);
                }
            }
        }
        Ok(())
    }

    fn predefined(&self, name: &str, path: &str, rule: &AstRule) -> Result<u16> {
        km_core::predefined_of(name).ok_or_else(|| {
            Error::script(
                path,
                rule.line,
                rule.column,
                format!("`{name}` is not a predefined key"),
            )
        })
    }

    /// Rejects what the engine cannot represent and warns about the rest.
    ///
    /// Only the position of the key states is fatal here; the reference checks
    /// are notes, because keyboards in the wild rely on them compiling.
    #[allow(clippy::unnecessary_wraps)] // the reference checks become fatal later
    fn validate(&mut self, rule: &AstRule, path: &str) -> Result<()> {
        let keys = rule
            .lhs
            .iter()
            .position(|item| matches!(item, Lhs::Keys(_)));
        if keys.is_some_and(|position| position + 1 < rule.lhs.len()) {
            return Err(Error::script(
                path,
                rule.line,
                rule.column,
                "virtual-key states end a rule's left side; nothing may follow them",
            ));
        }
        let mut segments = 0usize;
        let mut modified = Vec::new();
        for item in &rule.lhs {
            match item {
                Lhs::Text(_) | Lhs::Variable { .. } | Lhs::Any | Lhs::Key(_) => {
                    segments += 1;
                    if matches!(
                        item,
                        Lhs::Variable {
                            modifier: Modifier::AnyOf | Modifier::NotAnyOf,
                            ..
                        }
                    ) {
                        modified.push(segments);
                    }
                }
                Lhs::Switch(_) | Lhs::Keys(_) => {}
            }
        }
        for item in &rule.rhs {
            match item {
                Rhs::Reference(0) => {
                    return Err(Error::script(
                        path,
                        rule.line,
                        rule.column,
                        "a capture reference starts at `$1`",
                    ));
                }
                Rhs::Reference(index) if usize::from(*index) > segments => {
                    self.warn(
                        path,
                        rule.line,
                        rule.column,
                        format!("`${index}` is past the {segments} left-side segments"),
                    );
                }
                Rhs::Variable {
                    index: Some(index), ..
                } if !modified.contains(&usize::from(*index)) => {
                    self.warn(
                        path,
                        rule.line,
                        rule.column,
                        format!("`[${index}]` indexes a segment without `[*]` or `[^]`"),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn option(
        &mut self,
        name: &str,
        value: &str,
        path: &str,
        line: u32,
        column: u32,
        dir: Option<&Path>,
    ) {
        match name {
            "NAME" => self.infos.push((info_id::NAME, value.as_bytes().to_vec())),
            "DESCRIPTION" => self
                .infos
                .push((info_id::DESCRIPTION, value.as_bytes().to_vec())),
            "FONTFAMILY" => self
                .infos
                .push((info_id::FONT_FAMILY, value.as_bytes().to_vec())),
            "HOTKEY" => self
                .infos
                .push((info_id::HOTKEY, value.as_bytes().to_vec())),
            "ICON" => self.icon(value, path, line, column, dir),
            "TRACK_CAPSLOCK" => self.options.track_caps = parse_bool(value),
            "EAT_ALL_UNUSED_KEYS" | "EAT_KEYS" => self.options.eat = parse_bool(value),
            "US_LAYOUT_BASED" | "POSITIONAL_LAYOUT" => self.options.pos_based = parse_bool(value),
            "SMART_BACKSPACE" => self.options.auto_bksp = parse_bool(value),
            "TREAT_CTRL_ALT_AS_RALT" => self.options.right_alt = parse_bool(value),
            other => self.warn(
                path,
                line,
                column,
                format!("`@{other}` is not an option this build knows; it was ignored"),
            ),
        }
    }

    fn icon(&mut self, value: &str, path: &str, line: u32, column: u32, dir: Option<&Path>) {
        let Some(dir) = dir else {
            self.warn(
                path,
                line,
                column,
                "`@ICON` needs a file on disk; compiling from text skips it",
            );
            return;
        };
        let icon = dir.join(value);
        match std::fs::read(&icon) {
            Ok(bytes) => self.infos.push((info_id::ICON, bytes)),
            Err(error) => self.warn(
                path,
                line,
                column,
                format!("`@ICON` could not read {}: {error}", icon.display()),
            ),
        }
    }

    fn include(
        &mut self,
        included: &str,
        path: &str,
        line: u32,
        column: u32,
        dir: Option<&Path>,
        depth: usize,
    ) -> Result<()> {
        if depth >= MAX_INCLUDE_DEPTH {
            return Err(Error::script(
                path,
                line,
                column,
                "includes nest too deeply",
            ));
        }
        let Some(dir) = dir else {
            self.warn(
                path,
                line,
                column,
                format!("`include(\"{included}\")` needs a directory to resolve against"),
            );
            return Ok(());
        };
        let resolved = PathBuf::from(dir).join(included);
        let source =
            std::fs::read_to_string(&resolved).map_err(|error| Error::io(&resolved, &error))?;
        let included_path = resolved.display().to_string();
        let program = crate::parser::parse(
            &crate::lexer::tokenize(&source, &included_path)?,
            &included_path,
        )?;
        self.apply(&program, &included_path, resolved.parent(), depth + 1)
    }
}

/// Converts a zero-based table position to the format's 1-based id.
fn position_to_id(position: usize) -> u16 {
    u16::try_from(position).map_or(u16::MAX, |index| index.saturating_add(1))
}

/// Appends text as UTF-16 units, the unit every length in the format counts.
fn push_utf16(out: &mut Vec<u16>, text: &str) {
    out.extend(text.encode_utf16());
}

/// Writes `OP_STRING`, the UTF-16 length and the text.
fn push_literal(out: &mut Vec<u16>, text: &str) {
    let start = out.len();
    out.push(km_binary::OP_STRING);
    out.push(0);
    push_utf16(out, text);
    let length = u16::try_from(out.len() - start - 2).unwrap_or(u16::MAX);
    out[start + 1] = length;
}

/// Reads a boolean option value; anything but a true word is false.
fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_ascii_uppercase().as_str(),
        "TRUE" | "1" | "YES" | "ON"
    )
}
