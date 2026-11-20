//! Virtual-key codes and the predefined names a layout may use.

/// Windows virtual-key codes named as KeyMagic's predefined table names them.
///
/// Layouts write these inside angle brackets, for example `<VK_KEY_A>`. The
/// numbers are what `.km2` files store and what a platform adapter must
/// translate from its own key representation.
pub mod vk {
    /// Backspace.
    pub const BACK: u16 = 0x08;
    /// Tab.
    pub const TAB: u16 = 0x09;
    /// Enter / return.
    pub const RETURN: u16 = 0x0D;
    /// Either shift key.
    pub const SHIFT: u16 = 0x10;
    /// Either control key.
    pub const CONTROL: u16 = 0x11;
    /// Either alt key.
    pub const MENU: u16 = 0x12;
    /// Pause.
    pub const PAUSE: u16 = 0x13;
    /// Caps lock.
    pub const CAPITAL: u16 = 0x14;
    /// Kana / kanji.
    pub const KANJI: u16 = 0x19;
    /// Escape.
    pub const ESCAPE: u16 = 0x1B;
    /// Space bar.
    pub const SPACE: u16 = 0x20;
    /// Page up.
    pub const PRIOR: u16 = 0x21;
    /// Page down.
    pub const NEXT: u16 = 0x22;
    /// Delete.
    pub const DELETE: u16 = 0x2E;
    /// Left shift.
    pub const LSHIFT: u16 = 0xA0;
    /// Right shift.
    pub const RSHIFT: u16 = 0xA1;
    /// Left control.
    pub const LCONTROL: u16 = 0xA2;
    /// Right control.
    pub const RCONTROL: u16 = 0xA3;
    /// Left alt.
    pub const LMENU: u16 = 0xA4;
    /// Right alt, also known as AltGr.
    pub const RMENU: u16 = 0xA5;
    /// Semicolon key on a US keyboard.
    pub const OEM_1: u16 = 0xBA;
    /// Plus key.
    pub const OEM_PLUS: u16 = 0xBB;
    /// Comma key.
    pub const OEM_COMMA: u16 = 0xBC;
    /// Minus key.
    pub const OEM_MINUS: u16 = 0xBD;
    /// Period key.
    pub const OEM_PERIOD: u16 = 0xBE;
    /// Slash key.
    pub const OEM_2: u16 = 0xBF;
    /// Backtick key.
    pub const OEM_3: u16 = 0xC0;
    /// Left bracket key.
    pub const OEM_4: u16 = 0xDB;
    /// Backslash key.
    pub const OEM_5: u16 = 0xDC;
    /// Right bracket key.
    pub const OEM_6: u16 = 0xDD;
    /// Quote key.
    pub const OEM_7: u16 = 0xDE;
    /// The extra key some layouts carry.
    pub const OEM_8: u16 = 0xDF;
    /// The AX key.
    pub const OEM_AX: u16 = 0xE1;
    /// The 102nd key on ISO keyboards.
    pub const OEM_102: u16 = 0xE2;
    /// Help key.
    pub const ICO_HELP: u16 = 0xE3;
    /// The 00 key.
    pub const ICO_00: u16 = 0xE4;
}

/// Number of predefined ids, including `NULL` at 1.
pub const PREDEFINED_COUNT: u16 = 101;

/// Canonical predefined names in id order; entry `n` has id `n + 1`.
///
/// The key code is what a rule emits when the name appears on a rule's right
/// side; `NULL` emits nothing, which is how layouts delete text.
pub const CANONICAL: &[(&str, Option<u16>)] = &[
    ("NULL", None),
    ("VK_BACK", Some(vk::BACK)),
    ("VK_TAB", Some(vk::TAB)),
    ("VK_RETURN", Some(vk::RETURN)),
    ("VK_SHIFT", Some(vk::SHIFT)),
    ("VK_CONTROL", Some(vk::CONTROL)),
    ("VK_MENU", Some(vk::MENU)),
    ("VK_PAUSE", Some(vk::PAUSE)),
    ("VK_CAPITAL", Some(vk::CAPITAL)),
    ("VK_KANJI", Some(vk::KANJI)),
    ("VK_ESCAPE", Some(vk::ESCAPE)),
    ("VK_SPACE", Some(vk::SPACE)),
    ("VK_PRIOR", Some(vk::PRIOR)),
    ("VK_NEXT", Some(vk::NEXT)),
    ("VK_DELETE", Some(vk::DELETE)),
    ("VK_KEY_0", Some(b'0' as u16)),
    ("VK_KEY_1", Some(b'1' as u16)),
    ("VK_KEY_2", Some(b'2' as u16)),
    ("VK_KEY_3", Some(b'3' as u16)),
    ("VK_KEY_4", Some(b'4' as u16)),
    ("VK_KEY_5", Some(b'5' as u16)),
    ("VK_KEY_6", Some(b'6' as u16)),
    ("VK_KEY_7", Some(b'7' as u16)),
    ("VK_KEY_8", Some(b'8' as u16)),
    ("VK_KEY_9", Some(b'9' as u16)),
    ("VK_KEY_A", Some(b'A' as u16)),
    ("VK_KEY_B", Some(b'B' as u16)),
    ("VK_KEY_C", Some(b'C' as u16)),
    ("VK_KEY_D", Some(b'D' as u16)),
    ("VK_KEY_E", Some(b'E' as u16)),
    ("VK_KEY_F", Some(b'F' as u16)),
    ("VK_KEY_G", Some(b'G' as u16)),
    ("VK_KEY_H", Some(b'H' as u16)),
    ("VK_KEY_I", Some(b'I' as u16)),
    ("VK_KEY_J", Some(b'J' as u16)),
    ("VK_KEY_K", Some(b'K' as u16)),
    ("VK_KEY_L", Some(b'L' as u16)),
    ("VK_KEY_M", Some(b'M' as u16)),
    ("VK_KEY_N", Some(b'N' as u16)),
    ("VK_KEY_O", Some(b'O' as u16)),
    ("VK_KEY_P", Some(b'P' as u16)),
    ("VK_KEY_Q", Some(b'Q' as u16)),
    ("VK_KEY_R", Some(b'R' as u16)),
    ("VK_KEY_S", Some(b'S' as u16)),
    ("VK_KEY_T", Some(b'T' as u16)),
    ("VK_KEY_U", Some(b'U' as u16)),
    ("VK_KEY_V", Some(b'V' as u16)),
    ("VK_KEY_W", Some(b'W' as u16)),
    ("VK_KEY_X", Some(b'X' as u16)),
    ("VK_KEY_Y", Some(b'Y' as u16)),
    ("VK_KEY_Z", Some(b'Z' as u16)),
    ("VK_NUMPAD0", Some(0x60)),
    ("VK_NUMPAD1", Some(0x61)),
    ("VK_NUMPAD2", Some(0x62)),
    ("VK_NUMPAD3", Some(0x63)),
    ("VK_NUMPAD4", Some(0x64)),
    ("VK_NUMPAD5", Some(0x65)),
    ("VK_NUMPAD6", Some(0x66)),
    ("VK_NUMPAD7", Some(0x67)),
    ("VK_NUMPAD8", Some(0x68)),
    ("VK_NUMPAD9", Some(0x69)),
    ("VK_MULTIPLY", Some(0x6A)),
    ("VK_ADD", Some(0x6B)),
    ("VK_SEPARATOR", Some(0x6C)),
    ("VK_SUBTRACT", Some(0x6D)),
    ("VK_DECIMAL", Some(0x6E)),
    ("VK_DIVIDE", Some(0x6F)),
    ("VK_F1", Some(0x70)),
    ("VK_F2", Some(0x71)),
    ("VK_F3", Some(0x72)),
    ("VK_F4", Some(0x73)),
    ("VK_F5", Some(0x74)),
    ("VK_F6", Some(0x75)),
    ("VK_F7", Some(0x76)),
    ("VK_F8", Some(0x77)),
    ("VK_F9", Some(0x78)),
    ("VK_F10", Some(0x79)),
    ("VK_F11", Some(0x7A)),
    ("VK_F12", Some(0x7B)),
    ("VK_LSHIFT", Some(vk::LSHIFT)),
    ("VK_RSHIFT", Some(vk::RSHIFT)),
    ("VK_LCONTROL", Some(vk::LCONTROL)),
    ("VK_RCONTROL", Some(vk::RCONTROL)),
    ("VK_LMENU", Some(vk::LMENU)),
    ("VK_RMENU", Some(vk::RMENU)),
    ("VK_OEM_1", Some(vk::OEM_1)),
    ("VK_OEM_PLUS", Some(vk::OEM_PLUS)),
    ("VK_OEM_COMMA", Some(vk::OEM_COMMA)),
    ("VK_OEM_MINUS", Some(vk::OEM_MINUS)),
    ("VK_OEM_PERIOD", Some(vk::OEM_PERIOD)),
    ("VK_OEM_2", Some(vk::OEM_2)),
    ("VK_OEM_3", Some(vk::OEM_3)),
    ("VK_OEM_4", Some(vk::OEM_4)),
    ("VK_OEM_5", Some(vk::OEM_5)),
    ("VK_OEM_6", Some(vk::OEM_6)),
    ("VK_OEM_7", Some(vk::OEM_7)),
    ("VK_OEM_8", Some(vk::OEM_8)),
    ("VK_OEM_AX", Some(vk::OEM_AX)),
    ("VK_OEM_102", Some(vk::OEM_102)),
    ("VK_ICO_HELP", Some(vk::ICO_HELP)),
    ("VK_ICO_00", Some(vk::ICO_00)),
];

/// Names accepted in addition to [`CANONICAL`], and the name each stands for.
///
/// Layouts in the wild use both spellings; the alias table keeps a single id
/// for each key so matching never depends on how a rule was written.
pub const ALIASES: &[(&str, &str)] = &[
    ("VK_ENTER", "VK_RETURN"),
    ("VK_CTRL", "VK_CONTROL"),
    ("VK_ALT", "VK_MENU"),
    ("VK_CAPSLOCK", "VK_CAPITAL"),
    ("VK_ESC", "VK_ESCAPE"),
    ("VK_LCTRL", "VK_LCONTROL"),
    ("VK_RCTRL", "VK_RCONTROL"),
    ("VK_LALT", "VK_LMENU"),
    ("VK_RALT", "VK_RMENU"),
    ("VK_ALT_GR", "VK_RMENU"),
    ("VK_COLON", "VK_OEM_1"),
    ("VK_QUESTION", "VK_OEM_2"),
    ("VK_CFLEX", "VK_OEM_3"),
    ("VK_LBRACKET", "VK_OEM_4"),
    ("VK_BACKSLASH", "VK_OEM_5"),
    ("VK_RBRACKET", "VK_OEM_6"),
    ("VK_QUOTE", "VK_OEM_7"),
    ("VK_EXCM", "VK_OEM_8"),
    ("VK_LESSTHEN", "VK_OEM_102"),
];

/// Returns the canonical name an alias stands for, if `name` is an alias.
pub fn predefined_alias(name: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
        .map(|(_, target)| *target)
}

/// Returns the predefined id of a canonical `name`, if it is canonical.
pub fn canonical_id(name: &str) -> Option<u16> {
    let position = CANONICAL
        .iter()
        .position(|(canonical, _)| canonical.eq_ignore_ascii_case(name))?;
    u16::try_from(position).ok().map(|index| index + 1)
}

/// Resolves a layout's predefined name, alias or canonical, to its id.
pub fn predefined_of(name: &str) -> Option<u16> {
    if let Some(id) = canonical_id(name) {
        return Some(id);
    }
    predefined_alias(name).and_then(canonical_id)
}

/// Returns whether `name` names a predefined key.
pub fn is_predefined(name: &str) -> bool {
    predefined_of(name).is_some()
}

/// Returns the canonical name of a predefined id.
pub fn predefined_name(id: u16) -> Option<&'static str> {
    let index = usize::from(id.checked_sub(1)?);
    CANONICAL.get(index).map(|(name, _)| *name)
}

/// Returns the key code a predefined id emits, `None` for `NULL` and unknowns.
pub fn predefined_key(id: u16) -> Option<u16> {
    let index = usize::from(id.checked_sub(1)?);
    CANONICAL.get(index).and_then(|(_, key)| *key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_match_the_upstream_numbering() {
        assert_eq!(predefined_of("NULL"), Some(1));
        assert_eq!(predefined_of("VK_BACK"), Some(2));
        assert_eq!(predefined_of("VK_KEY_A"), Some(26));
        assert_eq!(predefined_of("VK_ALT_GR"), Some(85));
        assert_eq!(predefined_of("VK_ICO_00"), Some(PREDEFINED_COUNT));
    }

    #[test]
    fn aliases_resolve_to_canonical_ids() {
        assert_eq!(predefined_of("VK_ENTER"), predefined_of("VK_RETURN"));
        assert_eq!(predefined_of("vk_alt_gr"), predefined_of("VK_RMENU"));
        assert_eq!(predefined_of("VK_CFLEX"), predefined_of("VK_OEM_3"));
    }

    #[test]
    fn canonical_key_codes_round_trip() {
        for (offset, (name, key)) in CANONICAL.iter().enumerate() {
            let id = u16::try_from(offset).expect("id fits") + 1;
            assert_eq!(predefined_name(id), Some(*name));
            assert_eq!(predefined_key(id), *key);
        }
        assert_eq!(predefined_key(1), None, "NULL emits nothing");
        assert_eq!(predefined_key(PREDEFINED_COUNT + 1), None);
    }
}
