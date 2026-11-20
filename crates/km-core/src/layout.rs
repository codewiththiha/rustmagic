//! The five switches a compiled keyboard carries in its header.

/// Behaviour flags baked into a keyboard file.
///
/// Upstream defaults them to `track_caps: true`, `right_alt: true` and the rest
/// false; a layout overrides them with `@` options in its comment header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutOptions {
    /// Caps lock participates in matching. Hosts apply it when shaping the
    /// character they send; the engine keeps the flag for display and parity.
    pub track_caps: bool,
    /// A backspace that matches no rule undoes the previous step instead of
    /// deleting one character.
    pub auto_bksp: bool,
    /// Keys that match nothing are swallowed instead of typed.
    pub eat: bool,
    /// The layout is defined by physical key positions on a US keyboard.
    pub pos_based: bool,
    /// Control plus alt is treated as right alt (AltGr).
    pub right_alt: bool,
}

impl LayoutOptions {
    /// Upstream defaults, used when a file carries no options.
    pub const DEFAULT: Self = Self {
        track_caps: true,
        auto_bksp: false,
        eat: false,
        pos_based: false,
        right_alt: true,
    };

    /// The flag bytes in file order: caps, backspace, eat, positional, altgr.
    pub fn to_bytes(self) -> [u8; 5] {
        [
            u8::from(self.track_caps),
            u8::from(self.auto_bksp),
            u8::from(self.eat),
            u8::from(self.pos_based),
            u8::from(self.right_alt),
        ]
    }

    /// Reads the flag bytes in file order; anything but 0 or 1 is an error.
    pub fn from_bytes(bytes: [u8; 5]) -> crate::Result<Self> {
        for byte in bytes {
            if byte > 1 {
                return Err(crate::Error::InvalidKeyboard {
                    detail: format!("layout option byte {byte} is not a boolean"),
                });
            }
        }
        Ok(Self {
            track_caps: bytes[0] == 1,
            auto_bksp: bytes[1] == 1,
            eat: bytes[2] == 1,
            pos_based: bytes[3] == 1,
            right_alt: bytes[4] == 1,
        })
    }
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_round_trip() {
        let options = LayoutOptions {
            track_caps: true,
            auto_bksp: false,
            eat: true,
            pos_based: false,
            right_alt: true,
        };
        assert_eq!(LayoutOptions::from_bytes(options.to_bytes()), Ok(options));
    }

    #[test]
    fn default_matches_upstream() {
        let bytes = LayoutOptions::default().to_bytes();
        assert_eq!(bytes, [1, 0, 0, 0, 1]);
    }

    #[test]
    fn non_boolean_option_is_rejected() {
        assert!(LayoutOptions::from_bytes([1, 2, 0, 0, 1]).is_err());
    }
}
