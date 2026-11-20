//! X11 keysyms to what the engine matches on.
//!
//! IBus hands an engine a keysym, a hardware keycode and a modifier mask. The
//! layout language talks about virtual keys, so an engine has to translate: the
//! keysym says *what was typed*, the virtual key says *which key* was pressed.
//! Both matter — a rule may ask for `<VK_SHIFT & VK_KEY_W>` and a row of
//! `$latin[*]` only needs the character.

/// A key event, as the engine wants it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapped {
    /// Virtual-key code the engine matches against.
    pub key: u16,
    /// The character this press produced, if any.
    pub text: Option<char>,
}

/// IBus modifier bits, from `ibuskeynames.h`.
pub mod modifiers {
    /// Shift is held.
    pub const SHIFT: u32 = 1 << 0;
    /// Caps lock is on.
    pub const LOCK: u32 = 1 << 1;
    /// Control is held.
    pub const CONTROL: u32 = 1 << 2;
    /// Alt, or Meta on some keyboards.
    pub const MOD1: u32 = 1 << 3;
    /// Level 3 shift — AltGr on most layouts.
    pub const MOD5: u32 = 1 << 7;
    /// A key-release event rather than a press.
    pub const RELEASE: u32 = 1 << 30;
}

/// Keysyms the engine should let straight through: they only change state.
const PURE_MODIFIERS: &[u32] = &[
    0xffe1, // Shift_L
    0xffe2, // Shift_R
    0xffe3, // Control_L
    0xffe4, // Control_R
    0xffe7, // Meta_L
    0xffe8, // Meta_R
    0xffe9, // Alt_L
    0xffea, // Alt_R
    0xffeb, // Super_L
    0xffec, // Super_R
    0xffed, // Hyper_L
    0xffee, // Hyper_R
];

/// Whether the event only changes modifier state.
///
/// Lock keys are deliberately absent: a layout may map `VK_CAPITAL` to a
/// one-shot switch, which is how KeyMagic keyboards get an uppercase row.
pub fn is_pure_modifier(keysym: u32) -> bool {
    PURE_MODIFIERS.contains(&keysym)
}

/// The keys a composition ends on, so the application types them itself.
pub fn is_terminator(keysym: u32) -> bool {
    matches!(
        keysym,
        0x0020 // space
            | 0xff0d // Return
            | 0xff8d // KP_Enter
            | 0xff09 // Tab
            | 0xff1b // Escape
            | 0xff51..=0xff54 // arrows
            | 0xff50
            | 0xff57 // Home, End
            | 0xffe1..=0xffe4 // Shift and Control themselves
            | 0xffe7..=0xffea // Meta and Alt themselves
    )
}

/// Translates a keysym to the virtual key and character the engine sees.
///
/// Keysyms below `0x100` are the ASCII characters; keysyms from `0x01000000`
/// up carry a Unicode code point. Everything else is one of the special keys
/// listed below, which is every key a Myanmar layout ever asks about.
pub fn map(keysym: u32) -> Mapped {
    match keysym {
        // Letters and digits share their ASCII values with the virtual-key
        // codes, so no table is needed for the common case.
        0x0061..=0x007a => letter(keysym, keysym - 0x20),
        0x0041..=0x005a => letter(keysym, keysym),
        0x0030..=0x0039 => Mapped {
            key: u16::try_from(keysym).unwrap_or(0),
            text: char::from_u32(keysym),
        },
        // US punctuation: keysym, then the virtual key of the key it sits on.
        0x0021 => shifted(0x31),
        0x0040 => shifted(0x32),
        0x0023 => shifted(0x33),
        0x0024 => shifted(0x34),
        0x0025 => shifted(0x35),
        0x005e => shifted(0x36),
        0x0026 => shifted(0x37),
        0x002a => shifted(0x38),
        0x0028 => shifted(0x39),
        0x0029 => shifted(0x30),
        0x005f => shifted(0xbd),
        0x002b => shifted(0xbb),
        0x007b => shifted(0xdb),
        0x007d => shifted(0xdd),
        0x007c => shifted(0xdc),
        0x003a => shifted(0xba),
        0x0022 => shifted(0xde),
        0x003c => shifted(0xbc),
        0x003e => shifted(0xbe),
        0x003f => shifted(0xbf),
        0x007e => shifted(0xc0),
        0x002d => plain(0xbd),
        0x003d => plain(0xbb),
        0x005b => plain(0xdb),
        0x005d => plain(0xdd),
        0x005c => plain(0xdc),
        0x003b => plain(0xba),
        0x0027 => plain(0xde),
        0x0060 => plain(0xc0),
        0x002c => plain(0xbc),
        0x002e => plain(0xbe),
        0x002f => plain(0xbf),
        0x0020 => plain(0x20),
        // Editing and navigation keys.
        0xff08 => key_only(0x08), // BackSpace
        0xff09 => key_only(0x09), // Tab
        0xff0d => key_only(0x0d), // Return
        0xff1b => key_only(0x1b), // Escape
        0xffff => key_only(0x2e), // Delete
        0xff63 => key_only(0x2d), // Insert
        0xff50 => key_only(0x24), // Home
        0xff57 => key_only(0x23), // End
        0xff55 => key_only(0x21), // Page_Up
        0xff56 => key_only(0x22), // Page_Down
        0xff51 => key_only(0x25), // Left
        0xff52 => key_only(0x26), // Up
        0xff53 => key_only(0x27), // Right
        0xff54 => key_only(0x28), // Down
        // Modifier keys are passed as virtual keys so a rule can ask for them.
        0xffe1 | 0xffe2 => key_only(0x10),
        0xffe3 | 0xffe4 => key_only(0x11),
        0xffe7..=0xffea => key_only(0x12),
        0xffeb | 0xffec => key_only(0x5b),
        0xffe5 => key_only(0x14), // Caps_Lock
        0xff7f => key_only(0x90), // Num_Lock
        0xff14 => key_only(0x91), // Scroll_Lock
        // Function keys.
        0xffbe..=0xffc9 => key_only(0x70 + (keysym - 0xffbe) as u16),
        // Keypad digits and operators.
        0xffb0..=0xffb9 => Mapped {
            key: 0x60 + (keysym - 0xffb0) as u16,
            text: char::from_u32(0x30 + (keysym - 0xffb0)),
        },
        0xffaa => key_only(0x6a),          // KP_Multiply
        0xffab => key_only(0x6b),          // KP_Add
        0xffac | 0xffad => key_only(0x6d), // KP_Separator, KP_Subtract
        0xffae => key_only(0x6e),          // KP_Decimal
        0xffaf => key_only(0x6f),          // KP_Divide
        // Unicode keysyms: the code point is the character.
        0x0100_0000..=0x0110_ffff => Mapped {
            key: 0,
            text: char::from_u32(keysym - 0x0100_0000),
        },
        // Anything else printable in Latin-1 is still worth passing through.
        0x00a0..=0x00ff => Mapped {
            key: 0,
            text: char::from_u32(keysym),
        },
        _ => Mapped { key: 0, text: None },
    }
}

/// A letter: the keysym is the character, the virtual key is its uppercase code.
fn letter(keysym: u32, code: u32) -> Mapped {
    Mapped {
        key: u16::try_from(code).unwrap_or(0),
        text: char::from_u32(keysym),
    }
}

/// A key whose character is the same as the virtual key code.
fn plain(code: u16) -> Mapped {
    Mapped {
        key: code,
        text: char::from_u32(u32::from(code)),
    }
}

/// A shifted punctuation key: the virtual key is the unshifted one.
fn shifted(code: u16) -> Mapped {
    Mapped {
        key: code,
        text: None,
    }
}

/// A key with no character of its own.
fn key_only(code: u16) -> Mapped {
    Mapped {
        key: code,
        text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_carry_both_parts() {
        let mapped = map(0x0061);
        assert_eq!(mapped.key, 0x41, "`a` is VK 0x41");
        assert_eq!(mapped.text, Some('a'));
        assert_eq!(map(0x007a).key, 0x5a);
    }

    #[test]
    fn shifted_punctuation_keeps_the_physical_key() {
        let mapped = map(0x0021);
        assert_eq!(mapped.key, 0x31, "! sits on the 1 key");
        assert_eq!(mapped.text, None, "the host already knew the character");
        assert_eq!(map(0xffe1).key, 0x10, "Shift_L is VK_SHIFT");
    }

    #[test]
    fn editing_keys_map_to_windows_codes() {
        assert_eq!(map(0xff08).key, 0x08, "BackSpace");
        assert_eq!(map(0xff0d).key, 0x0d, "Return");
        assert_eq!(map(0xff51).key, 0x25, "Left");
        assert_eq!(map(0xffbe).key, 0x70, "F1");
    }

    #[test]
    fn unicode_keysyms_become_characters() {
        let mapped = map(0x0100_1041);
        assert_eq!(mapped.key, 0);
        assert_eq!(mapped.text, Some('၁'), "U+1041 is the Myanmar digit one");
    }

    #[test]
    fn modifiers_and_terminators_are_recognised() {
        assert!(is_pure_modifier(0xffe1));
        assert!(!is_pure_modifier(0xffe5), "caps lock may be a switch");
        assert!(!is_pure_modifier(0x0061));
        assert!(is_terminator(0x0020), "space ends a syllable");
        assert!(is_terminator(0xff0d));
        assert!(!is_terminator(0x0061));
    }
}
