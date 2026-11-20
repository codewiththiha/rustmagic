//! What a platform adapter hands the engine for one key event.

/// Modifier state at the moment a key went down.
///
/// `alt_gr` is the right-alt key that produces an extra symbol layer. Hosts
/// that cannot tell the two alts apart leave it false and let the layout's
/// `right_alt` option interpret ctrl+alt instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Shift is down.
    pub shift: bool,
    /// Control is down.
    pub control: bool,
    /// Left alt is down.
    pub alt: bool,
    /// Right alt (AltGr) is down.
    pub alt_gr: bool,
    /// Caps lock is on.
    pub caps: bool,
}

impl Modifiers {
    /// No modifier is down.
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        alt_gr: false,
        caps: false,
    };

    #[must_use]
    /// Returns these modifiers with shift held.
    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    #[must_use]
    /// Returns these modifiers with control held.
    pub const fn control(mut self) -> Self {
        self.control = true;
        self
    }

    #[must_use]
    /// Returns these modifiers with left alt held.
    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    #[must_use]
    /// Returns these modifiers with right alt held.
    pub const fn alt_gr(mut self) -> Self {
        self.alt_gr = true;
        self
    }

    /// Returns true when any of shift, control or alt is down.
    pub const fn any(self) -> bool {
        self.shift || self.control || self.alt || self.alt_gr
    }
}

/// One key event: the physical key, the character it produced, and modifiers.
///
/// `text` is what the platform says the key typed, already shifted and
/// capslocked. `key` is the virtual-key code from [`crate::vk`]; `0` means the
/// host has no code for the character, which is the case for most non-ASCII
/// input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    /// Virtual-key code, or `0` when unknown.
    pub key: u16,
    /// The character the key produced, if it produced one.
    pub text: Option<char>,
    /// Modifier state for this event.
    pub modifiers: Modifiers,
}

impl KeyInput {
    /// Builds an event for a typed character, deriving key code and shift.
    pub fn character(text: char) -> Self {
        let (key, shift) = key_for_char(text);
        Self {
            key,
            text: Some(text),
            modifiers: if shift {
                Modifiers::NONE.shift()
            } else {
                Modifiers::NONE
            },
        }
    }

    /// Builds an event for a key that types nothing, such as backspace.
    pub fn virtual_key(key: u16) -> Self {
        Self {
            key,
            text: None,
            modifiers: Modifiers::NONE,
        }
    }

    /// Builds a backspace event, the key a layout's smart backspace watches.
    pub fn backspace() -> Self {
        Self::virtual_key(crate::vk::BACK)
    }

    #[must_use]
    /// Returns this event with the given modifier state.
    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}

/// Maps a character to the US-layout virtual key that types it and whether
/// shift is needed. Returns `(0, false)` for characters with no US key.
pub fn key_for_char(text: char) -> (u16, bool) {
    const SHIFTED: &[(char, u16)] = &[
        ('!', 0x31),
        ('@', 0x32),
        ('#', 0x33),
        ('$', 0x34),
        ('%', 0x35),
        ('^', 0x36),
        ('&', 0x37),
        ('*', 0x38),
        ('(', 0x39),
        (')', 0x30),
        ('_', crate::vk::OEM_MINUS),
        ('+', crate::vk::OEM_PLUS),
        ('{', crate::vk::OEM_4),
        ('}', crate::vk::OEM_6),
        ('|', crate::vk::OEM_5),
        (':', crate::vk::OEM_1),
        ('"', crate::vk::OEM_7),
        ('<', crate::vk::OEM_COMMA),
        ('>', crate::vk::OEM_PERIOD),
        ('?', crate::vk::OEM_2),
        ('~', crate::vk::OEM_3),
    ];
    const PLAIN: &[(char, u16)] = &[
        (' ', crate::vk::SPACE),
        ('-', crate::vk::OEM_MINUS),
        ('=', crate::vk::OEM_PLUS),
        ('[', crate::vk::OEM_4),
        (']', crate::vk::OEM_6),
        ('\\', crate::vk::OEM_5),
        (';', crate::vk::OEM_1),
        ('\'', crate::vk::OEM_7),
        (',', crate::vk::OEM_COMMA),
        ('.', crate::vk::OEM_PERIOD),
        ('/', crate::vk::OEM_2),
        ('`', crate::vk::OEM_3),
        ('\t', crate::vk::TAB),
        ('\r', crate::vk::RETURN),
        ('\n', crate::vk::RETURN),
        ('\u{8}', crate::vk::BACK),
    ];

    if text.is_ascii_digit() {
        return (u16::from(text as u8), false);
    }
    if text.is_ascii_alphabetic() {
        let upper = text.to_ascii_uppercase();
        return (u16::from(upper as u8), text == upper);
    }
    if let Some((_, key)) = PLAIN.iter().find(|(plain, _)| *plain == text) {
        return (*key, false);
    }
    if let Some((_, key)) = SHIFTED.iter().find(|(shifted, _)| *shifted == text) {
        return (*key, true);
    }
    (0, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_carry_shift_when_uppercase() {
        let lower = KeyInput::character('a');
        assert_eq!(lower.key, 0x41);
        assert!(!lower.modifiers.shift);
        let upper = KeyInput::character('A');
        assert_eq!(upper.key, 0x41);
        assert!(upper.modifiers.shift);
        assert_eq!(upper.text, Some('A'));
    }

    #[test]
    fn shifted_punctuation_uses_the_unshifted_key() {
        assert_eq!(key_for_char('!'), (0x31, true));
        assert_eq!(key_for_char('1'), (0x31, false));
        assert_eq!(key_for_char('?'), (crate::vk::OEM_2, true));
        assert_eq!(key_for_char(' '), (crate::vk::SPACE, false));
    }

    #[test]
    fn non_ascii_has_no_virtual_key() {
        assert_eq!(key_for_char('\u{1000}'), (0, false));
        let input = KeyInput::character('\u{1000}');
        assert_eq!(input.key, 0);
        assert_eq!(input.text, Some('\u{1000}'));
    }

    #[test]
    fn backspace_is_a_virtual_key_with_no_text() {
        let input = KeyInput::backspace();
        assert_eq!(input.key, crate::vk::BACK);
        assert_eq!(input.text, None);
    }
}
