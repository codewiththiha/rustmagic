//! The try-it buffer: a text field the engine drives, with a log of what it did.

use km_core::KeyInput;
use km_engine::{Edit, Engine};

/// One key event, as the try-it view shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    /// How the key press is written, for example `a`, `Shift+A` or `Backspace`.
    pub label: String,
    /// Whether the engine took the key.
    pub consumed: bool,
    /// Characters it removed.
    pub erase: usize,
    /// What it inserted.
    pub insert: String,
    /// Source index of the rule that matched.
    pub rule: Option<usize>,
    /// The context after the event.
    pub context: String,
}

/// A buffer, an engine and a log, kept together.
#[derive(Debug, Clone, Default)]
pub struct Sandbox {
    engine: Option<Engine>,
    text: String,
    log: Vec<KeyEntry>,
}

/// How many key events the log keeps.
pub const LOG_LIMIT: usize = 200;

impl Sandbox {
    /// Points the sandbox at a keyboard, keeping the text and clearing the log.
    pub fn attach(&mut self, engine: Option<Engine>) {
        self.engine = engine;
        self.log.clear();
        if let Some(engine) = &mut self.engine {
            engine.set_context(&self.text);
            engine.reset();
            engine.set_context(&self.text);
        }
    }

    /// What the sandbox currently shows.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The engine behind the sandbox, when a keyboard is loaded.
    pub fn engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Key events, oldest first.
    pub fn log(&self) -> &[KeyEntry] {
        &self.log
    }

    /// Replaces the text and clears the log.
    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        self.text.push_str(text);
        self.log.clear();
        if let Some(engine) = &mut self.engine {
            engine.reset();
            engine.set_context(text);
        }
    }

    /// Clears the text, the log and the engine.
    pub fn reset(&mut self) {
        self.set_text("");
    }

    /// Feeds one key event through the engine and records what it did.
    pub fn key(&mut self, input: KeyInput) -> Option<&KeyEntry> {
        let engine = self.engine.as_mut()?;
        let outcome = engine.key(input);
        if outcome.consumed {
            apply(&mut self.text, &outcome.edit);
        } else if let Some(text) = input.text {
            self.text.push(text);
            engine.set_context(&self.text);
        }
        let entry = KeyEntry {
            label: label_for(input),
            consumed: outcome.consumed,
            erase: outcome.edit.erase,
            insert: outcome.edit.insert,
            rule: outcome.rule,
            context: self.text.clone(),
        };
        if self.log.len() >= LOG_LIMIT {
            self.log.remove(0);
        }
        self.log.push(entry);
        self.log.last()
    }

    /// Types text one character at a time.
    pub fn type_text(&mut self, text: &str) {
        for char in text.chars() {
            self.key(KeyInput::character(char));
        }
    }
}

/// How a key press is written in the log.
fn label_for(input: KeyInput) -> String {
    let mut parts = Vec::new();
    if input.modifiers.control {
        parts.push("Ctrl");
    }
    if input.modifiers.alt {
        parts.push("Alt");
    }
    if input.modifiers.alt_gr {
        parts.push("AltGr");
    }
    if input.modifiers.shift {
        parts.push("Shift");
    }
    let key = match (input.key, input.text) {
        (0x08, _) => "Backspace".to_owned(),
        (0x0D, _) => "Enter".to_owned(),
        (0x09, _) => "Tab".to_owned(),
        (0x20, _) | (_, Some(' ')) => "Space".to_owned(),
        (_, Some(text)) => text.to_string(),
        (key, None) => format!("VK 0x{key:02X}"),
    };
    parts.push(&key);
    parts.join("+")
}

/// Applies an edit to the end of a buffer.
fn apply(text: &mut String, edit: &Edit) {
    if edit.erase > 0 {
        let keep = text.chars().count().saturating_sub(edit.erase);
        *text = text.chars().take(keep).collect();
    }
    text.push_str(&edit.insert);
}
