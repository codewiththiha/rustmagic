//! What a keyboard says about itself: name, description, font, hotkey, icon.
//!
//! The `.km2` info table stores a 32-bit id and a byte string. Text ids hold
//! UTF-8 as written by the compiler; files from older writers sometimes hold
//! UTF-16, which [`decode_text`] detects.

use crate::keys;

/// The 32-bit ids the info table uses, spelled as upstream spells them.
pub mod info_id {
    /// Display name.
    pub const NAME: i32 = 0x6e61_6d65;
    /// Long description.
    pub const DESCRIPTION: i32 = 0x6465_7363;
    /// Preferred font family.
    pub const FONT_FAMILY: i32 = 0x666f_6e74;
    /// Icon image bytes.
    pub const ICON: i32 = 0x6963_6f6e;
    /// Switch hotkey, for example `CTRL+SHIFT+M`.
    pub const HOTKEY: i32 = 0x6874_6b79;
}

/// What a keyboard tells the library about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyboardMeta {
    /// Display name.
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Font the layout was designed for.
    pub font_family: String,
    /// Hotkey that selects this layout.
    pub hotkey: String,
    /// Raw icon bytes, undecoded: the GUI decides how to show them.
    pub icon: Option<Vec<u8>>,
}

impl KeyboardMeta {
    /// Reads metadata from an info table, last entry of an id winning.
    pub fn from_infos(infos: &[(i32, Vec<u8>)]) -> Self {
        let mut meta = Self::default();
        for (id, data) in infos {
            match *id {
                info_id::NAME => meta.name = decode_text(data),
                info_id::DESCRIPTION => meta.description = decode_text(data),
                info_id::FONT_FAMILY => meta.font_family = decode_text(data),
                info_id::HOTKEY => meta.hotkey = decode_text(data),
                info_id::ICON => meta.icon = Some(data.clone()),
                _ => {}
            }
        }
        meta
    }

    /// Writes metadata as an info table in a stable order, skipping empties.
    pub fn to_infos(&self) -> Vec<(i32, Vec<u8>)> {
        let mut infos = Vec::new();
        for (id, text) in [
            (info_id::NAME, &self.name),
            (info_id::DESCRIPTION, &self.description),
            (info_id::FONT_FAMILY, &self.font_family),
            (info_id::HOTKEY, &self.hotkey),
        ] {
            if !text.is_empty() {
                infos.push((id, text.as_bytes().to_vec()));
            }
        }
        if let Some(icon) = &self.icon {
            infos.push((info_id::ICON, icon.clone()));
        }
        infos
    }

    /// The name to show in a list, falling back to a placeholder.
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            "(unnamed keyboard)"
        } else {
            &self.name
        }
    }
}

/// Decodes info text: UTF-8 when it is valid, UTF-16LE when it looks like it.
///
/// Older writers stored the info table as UTF-16; a NUL in every high byte is
/// what tells the two apart in practice.
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let looks_utf16 = bytes.len() % 2 == 0
        && bytes.len() > 1
        && bytes
            .iter()
            .skip(1)
            .step_by(2)
            .take(4)
            .all(|byte| *byte == 0);
    if looks_utf16 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// A parsed hotkey: a key plus the modifiers that must be held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    /// Virtual-key code of the final key.
    pub key: u16,
    /// Modifiers required, with `caps` always false.
    pub modifiers: crate::Modifiers,
}

/// Parses a hotkey string such as `CTRL+SHIFT+M` or `Ctrl + Alt + 1`.
///
/// Names are matched case-insensitively. Returns `None` when the string has no
/// recognisable final key, so a layout's typo never becomes a live shortcut.
pub fn parse_hotkey(text: &str) -> Option<Hotkey> {
    let mut modifiers = crate::Modifiers::NONE;
    let mut key = None;
    for part in text
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers.control = true,
            "SHIFT" => modifiers.shift = true,
            "ALT" | "MENU" => modifiers.alt = true,
            "ALTGR" => modifiers.alt_gr = true,
            other => {
                let resolved = keys::predefined_of(other)
                    .or_else(|| keys::predefined_of(&format!("VK_{other}")))
                    .or_else(|| single_char_key(other));
                if key.is_some() {
                    return None;
                }
                key = resolved;
            }
        }
    }
    Some(Hotkey {
        key: key?,
        modifiers,
    })
}

/// Maps a one-character hotkey component such as `1` or `K` to its key code.
fn single_char_key(text: &str) -> Option<u16> {
    let mut chars = text.chars();
    let single = chars.next()?;
    if chars.next().is_some() || !single.is_ascii() {
        return None;
    }
    let (key, _shift) = crate::input::key_for_char(single.to_ascii_uppercase());
    (key != 0).then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infos_round_trip() {
        let meta = KeyboardMeta {
            name: "Demo".into(),
            description: "Shows things".into(),
            font_family: "Inter".into(),
            hotkey: "CTRL+SHIFT+M".into(),
            icon: Some(vec![1, 2, 3]),
        };
        assert_eq!(KeyboardMeta::from_infos(&meta.to_infos()), meta);
    }

    #[test]
    fn utf16_info_text_is_detected() {
        let bytes: Vec<u8> = "Zawgyi".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_text(&bytes), "Zawgyi");
        assert_eq!(decode_text(b"Zawgyi"), "Zawgyi");
    }

    #[test]
    fn hotkey_parses_modifiers_and_key() {
        let hotkey = parse_hotkey("CTRL + SHIFT + M").expect("parses");
        assert!(hotkey.modifiers.control && hotkey.modifiers.shift);
        assert_eq!(
            hotkey.key,
            keys::predefined_of("VK_KEY_M").map_or(0, |_| 0x4D)
        );
        assert!(parse_hotkey("CTRL+").is_none());
    }

    #[test]
    fn display_name_falls_back() {
        assert_eq!(KeyboardMeta::default().display_name(), "(unnamed keyboard)");
    }
}
