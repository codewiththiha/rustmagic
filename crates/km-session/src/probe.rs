//! What each key of a US keyboard types under a layout.
//!
//! A layout is a set of rules over context, so "what does this key produce" has
//! no single answer. The layout view answers the question a person asks first:
//! what does this key do on its own, with Shift, and with AltGr.

use std::sync::Arc;

use km_core::{KeyInput, Modifiers};
use km_engine::{Engine, Keyboard};

/// One key of the layout view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCap {
    /// The key, as printed on the cap.
    pub label: String,
    /// What it produces unmodified.
    pub plain: String,
    /// What it produces with Shift.
    pub shifted: String,
}

/// One row of the layout view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRow {
    /// Caps from left to right.
    pub caps: Vec<KeyCap>,
}

/// Rows in US keyboard order: numbers, QWERTY, home, bottom.
pub const ROWS: &[&str] = &[
    "1234567890-=",
    "qwertyuiop[]\\",
    "asdfghjkl;'",
    "zxcvbnm,./",
];

/// Builds every row's caps for a keyboard.
pub fn layout_rows(keyboard: &Arc<Keyboard>) -> Vec<KeyRow> {
    let mut rows = Vec::with_capacity(ROWS.len());
    for row in ROWS {
        let caps = row
            .chars()
            .map(|char| {
                let shifted = shifted_form(char);
                KeyCap {
                    label: char.to_string(),
                    plain: probe(keyboard, char),
                    shifted: probe(keyboard, shifted),
                }
            })
            .collect();
        rows.push(KeyRow { caps });
    }
    rows
}

/// The character the same key types when Shift is held.
fn shifted_form(char: char) -> char {
    match char {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        letter => letter.to_ascii_uppercase(),
    }
}

/// Types one character into a fresh engine and returns what appears.
///
/// If nothing appears, the key is not mapped and shows itself, which is what a
/// person sees when they press it.
fn probe(keyboard: &Arc<Keyboard>, char: char) -> String {
    let mut engine = Engine::new(Arc::clone(keyboard));
    let modifiers = if char.is_ascii_uppercase() {
        Modifiers::NONE.shift()
    } else {
        Modifiers::NONE
    };
    engine.key(KeyInput::character(char).with_modifiers(modifiers));
    let produced: String = engine
        .context()
        .chars()
        .filter(|char| *char != '\0')
        .collect();
    if produced.trim().is_empty() {
        char.to_string()
    } else {
        produced.trim().to_owned()
    }
}
