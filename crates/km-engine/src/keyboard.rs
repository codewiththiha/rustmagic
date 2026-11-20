//! Decoding a `.km2` container into rules the engine can run.
//!
//! Decoding happens once, when a keyboard is loaded: variable references are
//! spliced, opcodes become [`Pattern`] and [`Emit`] values, and rules are put
//! in match order. Nothing here touches the context.

use std::path::Path;

use km_binary::{RawKeyboard, RawRule, Version};
use km_core::{Error, KeyboardMeta, LayoutOptions, Result, predefined_key, vk};

use crate::rule::{Emit, KeyRequirement, Pattern, Rule};

/// How deep variable references may nest before the file is rejected.
const MAX_VARIABLE_DEPTH: usize = 32;

/// A keyboard ready to drive the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyboard {
    /// Name, description, font, hotkey and icon.
    pub meta: KeyboardMeta,
    /// Behaviour flags.
    pub options: LayoutOptions,
    /// Variable values, indexed so that id 1 is `variables[0]`.
    pub variables: Vec<String>,
    /// Rules in match order: most specific first.
    pub rules: Vec<Rule>,
    /// Version the file declared.
    pub version: Version,
    /// Everything odd the decoder had to accept.
    pub warnings: Vec<String>,
}

impl Keyboard {
    /// Reads and decodes a keyboard file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_raw(km_binary::read_file(path)?)
    }

    /// Decodes a keyboard from `.km2` bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::from_raw(km_binary::read(bytes)?)
    }

    /// Decodes a container that has already been read.
    pub fn from_raw(raw: RawKeyboard) -> Result<Self> {
        let mut warnings: Vec<String> = raw
            .warnings
            .iter()
            .map(|warning| warning.message.clone())
            .collect();
        let variables = resolve_variables(&raw.strings, &mut warnings)?;
        let mut rules = Vec::with_capacity(raw.rules.len());
        for rule in &raw.rules {
            rules.push(decode_rule(rule, &variables, &mut warnings)?);
        }
        rules.sort_by(priority);
        Ok(Self {
            meta: raw.meta(),
            options: raw.options,
            variables,
            rules,
            version: raw.version,
            warnings,
        })
    }

    /// Number of rules the engine will try, in match order.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// The value of a string-table id, or an empty string when it is absent.
    pub fn variable(&self, id: u16) -> &str {
        usize::from(id)
            .checked_sub(1)
            .and_then(|index| self.variables.get(index))
            .map_or("", String::as_str)
    }
}

/// Match order: more switch conditions, then more keys, then longer patterns.
///
/// Upstream sorts ascending on the same three keys and reverses, which leaves
/// ties in an arbitrary order; this is a stable sort on the same keys, so ties
/// keep their file order and two runs agree.
fn priority(left: &Rule, right: &Rule) -> std::cmp::Ordering {
    right
        .switch_count
        .cmp(&left.switch_count)
        .then_with(|| right.key_count().cmp(&left.key_count()))
        .then_with(|| right.match_len.cmp(&left.match_len))
}

/// Splices variable references into their values.
fn resolve_variables(strings: &[Vec<u16>], warnings: &mut Vec<String>) -> Result<Vec<String>> {
    let mut resolved = Vec::with_capacity(strings.len());
    for id in 1..=strings.len() {
        let mut stack = Vec::new();
        resolved.push(expand(strings, id, 0, &mut stack, warnings)?);
    }
    Ok(resolved)
}

fn expand(
    strings: &[Vec<u16>],
    id: usize,
    depth: usize,
    stack: &mut Vec<usize>,
    warnings: &mut Vec<String>,
) -> Result<String> {
    if depth >= MAX_VARIABLE_DEPTH || stack.contains(&id) {
        return Err(Error::VariableCycle {
            index: u16::try_from(id).unwrap_or(u16::MAX),
        });
    }
    let units = strings.get(id - 1).ok_or(Error::DanglingReference {
        index: u16::try_from(id).unwrap_or(u16::MAX),
        table: "variable",
    })?;
    stack.push(id);
    let mut out = Vec::with_capacity(units.len());
    let mut cursor = 0;
    while let Some(word) = units.get(cursor) {
        cursor += 1;
        if *word != km_binary::OP_VARIABLE {
            out.push(*word);
            continue;
        }
        let Some(index) = units.get(cursor) else {
            warnings.push(format!("variable {id} ends with a broken reference"));
            break;
        };
        cursor += 1;
        let nested = usize::from(*index);
        if nested == 0 || nested > strings.len() {
            warnings.push(format!(
                "variable {id} refers to {index}, which does not exist"
            ));
            continue;
        }
        let text = expand(strings, nested, depth + 1, stack, warnings)?;
        out.extend(text.encode_utf16());
    }
    stack.pop();
    Ok(String::from_utf16_lossy(&out))
}

/// Decodes one rule's words into patterns and emissions.
fn decode_rule(raw: &RawRule, variables: &[String], warnings: &mut Vec<String>) -> Result<Rule> {
    let mut patterns = Vec::new();
    let mut keys = KeyRequirement::default();
    let mut cursor = 0;
    while let Some(word) = raw.lhs.get(cursor).copied() {
        cursor += 1;
        match word {
            km_binary::OP_STRING => {
                let text = read_text(&raw.lhs, raw.index, &mut cursor, warnings);
                patterns.push(Pattern::Text(text));
            }
            km_binary::OP_VARIABLE => {
                let index = raw.lhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                patterns.push(Pattern::Text(variable_text(variables, index, raw.index)));
            }
            km_binary::OP_ANY => patterns.push(Pattern::Any),
            km_binary::OP_SWITCH => {
                let index = raw.lhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                patterns.push(Pattern::Switch(index));
            }
            km_binary::OP_PREDEFINED => {
                let id = raw.lhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                if let Some(key) = predefined_key(id) {
                    patterns.push(Pattern::Text(character(key)));
                } else {
                    warnings.push(format!(
                        "rule {} matches on a key that emits nothing; it consumes no text",
                        raw.index
                    ));
                    patterns.push(Pattern::Text(String::new()));
                }
            }
            km_binary::OP_MODIFIER => {
                let mode = raw.lhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                apply_modifier(&mut patterns, mode, raw.index, warnings);
            }
            km_binary::OP_REFERENCE => {
                cursor += 1;
                warnings.push(format!(
                    "rule {} refers to a capture on its left side, which no KeyMagic engine supports",
                    raw.index
                ));
                patterns.push(Pattern::Unsupported);
            }
            km_binary::OP_AND => {
                cursor += read_keys(raw, &mut cursor, &mut keys);
                if cursor < raw.lhs.len() {
                    warnings.push(format!(
                        "rule {} has left-side segments after its key states; they are ignored, as upstream ignores them",
                        raw.index
                    ));
                }
                break;
            }
            other => {
                return Err(Error::BadOpcode {
                    opcode: other,
                    rule: raw.index,
                });
            }
        }
    }
    let emits = decode_emits(raw, variables, &patterns, warnings)?;
    let match_len = patterns.iter().map(Pattern::width).sum();
    let switch_count = patterns
        .iter()
        .filter(|pattern| matches!(pattern, Pattern::Switch(_)))
        .count();
    Ok(Rule {
        index: raw.index,
        match_len,
        keys,
        patterns,
        emits,
        switch_count,
    })
}

/// Reads the `OP_PREDEFINED` list after an `OP_AND`, returning words consumed.
fn read_keys(raw: &RawRule, cursor: &mut usize, keys: &mut KeyRequirement) -> usize {
    let start = *cursor;
    while raw.lhs.get(*cursor) == Some(&km_binary::OP_PREDEFINED) {
        let id = raw.lhs.get(*cursor + 1).copied().unwrap_or(0);
        *cursor += 2;
        record_key(keys, predefined_key(id));
    }
    *cursor - start
}

/// Sorts a predefined key into the requirement it implies.
fn record_key(keys: &mut KeyRequirement, key: Option<u16>) {
    match key {
        Some(vk::SHIFT | vk::LSHIFT | vk::RSHIFT) => keys.shift = true,
        Some(vk::CONTROL | vk::LCONTROL | vk::RCONTROL) => keys.control = true,
        Some(vk::MENU | vk::LMENU) => keys.alt = true,
        Some(vk::RMENU) => keys.alt_gr = true,
        Some(other) => keys.keys.push(other),
        None => {}
    }
}

/// Applies `OP_MODIFIER` to the segment before it.
fn apply_modifier(patterns: &mut [Pattern], mode: u16, rule: usize, warnings: &mut Vec<String>) {
    let Some(last) = patterns.last_mut() else {
        warnings.push(format!("rule {rule} modifies nothing"));
        return;
    };
    let Some(text) = last.text().map(str::to_owned) else {
        warnings.push(format!("rule {rule} modifies a segment that holds no text"));
        return;
    };
    *last = match mode {
        km_binary::OP_ANYOF => Pattern::AnyOf(text),
        km_binary::OP_NANYOF => Pattern::NotAnyOf(text),
        _ => {
            warnings.push(format!(
                "rule {rule} captures from its left side, which no KeyMagic engine supports"
            ));
            Pattern::Unsupported
        }
    };
}

/// Decodes the right side.
fn decode_emits(
    raw: &RawRule,
    variables: &[String],
    patterns: &[Pattern],
    warnings: &mut Vec<String>,
) -> Result<Vec<Emit>> {
    let mut emits = Vec::new();
    let mut cursor = 0;
    while let Some(word) = raw.rhs.get(cursor).copied() {
        cursor += 1;
        match word {
            km_binary::OP_STRING => {
                let text = read_text(&raw.rhs, raw.index, &mut cursor, warnings);
                emits.push(Emit::Text(text));
            }
            km_binary::OP_VARIABLE => {
                let index = raw.rhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                let text = variable_text(variables, index, raw.index).clone();
                if raw.rhs.get(cursor) == Some(&km_binary::OP_MODIFIER) {
                    let segment = raw.rhs.get(cursor + 1).copied().unwrap_or(0);
                    cursor += 2;
                    emits.push(Emit::Indexed {
                        table: text,
                        source: capture_source(patterns, usize::from(segment.saturating_sub(1))),
                        segment: usize::from(segment.saturating_sub(1)),
                    });
                } else {
                    emits.push(Emit::Text(text));
                }
            }
            km_binary::OP_REFERENCE => {
                let index = raw.rhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                emits.push(Emit::Reference(usize::from(index.saturating_sub(1))));
            }
            km_binary::OP_PREDEFINED => {
                let id = raw.rhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                emits.push(Emit::Text(
                    predefined_key(id).map(character).unwrap_or_default(),
                ));
            }
            km_binary::OP_SWITCH => {
                let index = raw.rhs.get(cursor).copied().unwrap_or(0);
                cursor += 1;
                emits.push(Emit::Switch(index));
            }
            other => {
                return Err(Error::BadOpcode {
                    opcode: other,
                    rule: raw.index,
                });
            }
        }
    }
    Ok(emits)
}

/// Reads an `OP_STRING` payload: length in units, then the text.
fn read_text(side: &[u16], rule: usize, cursor: &mut usize, warnings: &mut Vec<String>) -> String {
    let length = usize::from(side.get(*cursor).copied().unwrap_or(0));
    *cursor += 1;
    let available = side.len().saturating_sub(*cursor);
    if length > available {
        warnings.push(format!(
            "rule {rule} declares {length} characters but carries {available}"
        ));
    }
    let end = (*cursor + length).min(side.len());
    let units = &side[*cursor..end];
    *cursor = end;
    String::from_utf16_lossy(units)
}

/// The text of the `segment`-th capturing pattern, counting from zero.
fn capture_source(patterns: &[Pattern], segment: usize) -> String {
    patterns
        .iter()
        .filter(|pattern| pattern.captures())
        .nth(segment)
        .and_then(Pattern::text)
        .unwrap_or_default()
        .to_owned()
}

fn variable_text(variables: &[String], index: u16, _rule: usize) -> String {
    usize::from(index)
        .checked_sub(1)
        .and_then(|position| variables.get(position))
        .cloned()
        .unwrap_or_default()
}

/// Turns a key code into the character a rule emits for it.
fn character(key: u16) -> String {
    char::from_u32(u32::from(key))
        .map(|char| char.to_string())
        .unwrap_or_default()
}
