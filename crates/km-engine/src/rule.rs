//! One compiled rule: what it matches, what it needs held, what it emits.

/// A left-side pattern element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// Literal text, matched exactly.
    Text(String),
    /// One character taken from the string.
    AnyOf(String),
    /// One character that is not in the string.
    NotAnyOf(String),
    /// One character the engine considers matchable.
    Any,
    /// The switch with this string-table id must be on.
    Switch(u16),
    /// A left-side capture reference, which upstream never implemented.
    ///
    /// The rule can never match, which is what upstream does with it too.
    Unsupported,
}

impl Pattern {
    /// How many characters this element consumes from the context.
    pub fn width(&self) -> usize {
        match self {
            Self::Text(text) => text.chars().count(),
            Self::AnyOf(_) | Self::NotAnyOf(_) | Self::Any => 1,
            Self::Switch(_) | Self::Unsupported => 0,
        }
    }

    /// The string this element tests against, when it has one.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) | Self::AnyOf(text) | Self::NotAnyOf(text) => Some(text),
            _ => None,
        }
    }

    /// True when matching this element captures the characters it consumed.
    pub fn captures(&self) -> bool {
        matches!(
            self,
            Self::Text(_) | Self::AnyOf(_) | Self::NotAnyOf(_) | Self::Any
        )
    }
}

/// A right-side element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emit {
    /// Literal text.
    Text(String),
    /// The text a left-side segment matched, by zero-based segment index.
    Reference(usize),
    /// A character from `table`, at the position the segment matched.
    Indexed {
        /// The string the emitted character is taken from.
        table: String,
        /// The left-side string the position is measured in.
        source: String,
        /// Zero-based left-side segment whose match gives the position.
        segment: usize,
    },
    /// Toggles the switch with this string-table id.
    Switch(u16),
}

/// Which modifiers a rule needs, and which key must be the one pressed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyRequirement {
    /// Shift must be down.
    pub shift: bool,
    /// Control must be down, and is consumed by the rule.
    pub control: bool,
    /// Left alt must be down, and is consumed by the rule.
    pub alt: bool,
    /// Right alt must be down.
    pub alt_gr: bool,
    /// Keys that must be the one being pressed.
    pub keys: Vec<u16>,
}

impl KeyRequirement {
    /// True when the rule says nothing about keys or modifiers.
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.control && !self.alt && !self.alt_gr && self.keys.is_empty()
    }
}

/// A rule as the engine sees it: patterns, requirements and emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Position in the source file, kept for logs and diagnostics.
    pub index: usize,
    /// Characters the left side consumes.
    pub match_len: usize,
    /// Modifiers and keys the rule requires.
    pub keys: KeyRequirement,
    /// Left side, in order.
    pub patterns: Vec<Pattern>,
    /// Right side, in order.
    pub emits: Vec<Emit>,
    /// Number of switch conditions, used for ordering.
    pub switch_count: usize,
}

impl Rule {
    /// How many keys the rule names, modifiers included; ordering uses it.
    pub fn key_count(&self) -> usize {
        usize::from(self.keys.shift)
            + usize::from(self.keys.control)
            + usize::from(self.keys.alt)
            + usize::from(self.keys.alt_gr)
            + self.keys.keys.len()
    }

    /// Describes the left side the way a layout author writes it.
    pub fn describe(&self) -> String {
        let patterns: Vec<String> = self
            .patterns
            .iter()
            .map(|pattern| match pattern {
                Pattern::Text(text) => format!("\"{text}\""),
                Pattern::AnyOf(text) => format!("$[{text}*]"),
                Pattern::NotAnyOf(text) => format!("$[{text}^]"),
                Pattern::Any => "ANY".to_owned(),
                Pattern::Switch(id) => format!("(switch {id})"),
                Pattern::Unsupported => "$?".to_owned(),
            })
            .collect();
        let mut text = patterns.join(" + ");
        if !self.keys.is_empty() {
            let mut keys = Vec::new();
            if self.keys.shift {
                keys.push("VK_SHIFT".to_owned());
            }
            if self.keys.control {
                keys.push("VK_CONTROL".to_owned());
            }
            if self.keys.alt {
                keys.push("VK_MENU".to_owned());
            }
            if self.keys.alt_gr {
                keys.push("VK_ALT_GR".to_owned());
            }
            for key in &self.keys.keys {
                keys.push(key_name(*key));
            }
            let states = format!("<{}>", keys.join(" & "));
            text = if text.is_empty() {
                states
            } else {
                format!("{states} + {text}")
            };
        }
        text
    }
}

/// Names a virtual key for display, falling back to its number.
fn key_name(key: u16) -> String {
    for id in 1..=km_core::PREDEFINED_COUNT {
        if km_core::predefined_key(id) == Some(key) {
            if let Some(name) = km_core::predefined_name(id) {
                return name.to_owned();
            }
        }
    }
    format!("0x{key:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_counts_characters_not_bytes() {
        assert_eq!(Pattern::Text("\u{1000}\u{1001}".into()).width(), 2);
        assert_eq!(Pattern::Any.width(), 1);
        assert_eq!(Pattern::Switch(3).width(), 0);
    }

    #[test]
    fn describing_a_rule_shows_both_sides() {
        let rule = Rule {
            index: 0,
            match_len: 1,
            keys: KeyRequirement {
                shift: true,
                keys: vec![0x41],
                ..KeyRequirement::default()
            },
            patterns: vec![Pattern::Text("a".into())],
            emits: vec![Emit::Text("b".into())],
            switch_count: 0,
        };
        assert_eq!(rule.describe(), "<VK_SHIFT & VK_KEY_A> + \"a\"");
    }
}
