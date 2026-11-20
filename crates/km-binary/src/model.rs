//! The container model: what a file holds, before any decoding.

use km_core::LayoutOptions;

/// File version, as the two bytes after the magic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version; anything but 1 is unreadable.
    pub major: u8,
    /// Minor version: 3, 4 and 5 differ only in header fields.
    pub minor: u8,
}

impl Version {
    /// Reads the version a header declares.
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Length of a header of this version, in bytes, including padding.
    pub const fn header_len(self) -> usize {
        match self.minor {
            3 => 14,
            4 => 16,
            _ => 18,
        }
    }

    /// True when the file carries an info table.
    pub const fn has_infos(self) -> bool {
        self.minor >= 4
    }

    /// True when the file stores the right-alt option.
    pub const fn has_right_alt(self) -> bool {
        self.minor >= 5
    }
}

/// One rule's opcode words, still undecoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRule {
    /// Position in the file, from zero; reported in diagnostics.
    pub index: usize,
    /// Left side: what must match.
    pub lhs: Vec<u16>,
    /// Right side: what is produced.
    pub rhs: Vec<u16>,
}

/// A repair the reader made, or something the writer should know about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Rule the warning is about, when it is about one rule.
    pub rule: Option<usize>,
    /// What was found and what the reader did.
    pub message: String,
}

/// Everything a keyboard file holds, with opcodes left as words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawKeyboard {
    /// Version the file declares.
    pub version: Version,
    /// Behaviour flags from the header.
    pub options: LayoutOptions,
    /// Variable table, still holding inline `OP_VARIABLE` references.
    pub strings: Vec<Vec<u16>>,
    /// Info table: 32-bit id plus raw bytes, text or image.
    pub infos: Vec<(i32, Vec<u8>)>,
    /// Rules in file order.
    pub rules: Vec<RawRule>,
    /// Repairs and oddities found while reading.
    pub warnings: Vec<Warning>,
}

impl RawKeyboard {
    /// An empty keyboard at the current version.
    pub fn empty() -> Self {
        Self {
            version: crate::CURRENT_VERSION,
            options: LayoutOptions::default(),
            strings: Vec::new(),
            infos: Vec::new(),
            rules: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Human readable metadata read from the info table.
    pub fn meta(&self) -> km_core::KeyboardMeta {
        km_core::KeyboardMeta::from_infos(&self.infos)
    }
}
