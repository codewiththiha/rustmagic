//! How the engine signals a problem it cannot work around.

#![deny(missing_docs)]

use std::fmt;

/// Result alias used across the crate family.
pub type Result<T> = std::result::Result<T, Error>;

/// A failure with enough context for a person to act on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Reading or writing a file failed.
    Io {
        /// Path that failed, as given to the call.
        path: String,
        /// Operating-system message.
        message: String,
    },
    /// The bytes are not a KeyMagic keyboard.
    NotKeyMagic,
    /// The file was written by a newer KeyMagic than this build understands.
    UnsupportedVersion {
        /// Major version byte.
        major: u8,
        /// Minor version byte.
        minor: u8,
    },
    /// The file ended in the middle of a structure.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// Offset within the file.
        offset: usize,
    },
    /// Declared section counts do not describe the file's actual contents.
    CountMismatch {
        /// What the header promised plus what the file held.
        detail: String,
    },
    /// A length in the file exceeds the file itself.
    BadLength {
        /// What was being read.
        what: &'static str,
        /// The declared length.
        length: usize,
        /// Bytes available.
        available: usize,
    },
    /// An opcode is not part of the KeyMagic instruction set.
    BadOpcode {
        /// The unknown opcode.
        opcode: u16,
        /// Rule index that contains it.
        rule: usize,
    },
    /// A rule refers to a variable or switch that does not exist.
    DanglingReference {
        /// The 1-based index in the file.
        index: u16,
        /// What kind of table the index points into.
        table: &'static str,
    },
    /// Variables refer to each other without end.
    VariableCycle {
        /// The 1-based index that closed the cycle.
        index: u16,
    },
    /// A compiled keyboard breaks a rule the engine relies on.
    InvalidKeyboard {
        /// Human readable description of the broken rule.
        detail: String,
    },
    /// The layout script could not be compiled.
    Script {
        /// 1-based line in the source or its includes.
        line: u32,
        /// 1-based column.
        column: u32,
        /// Source path or `<memory>`.
        path: String,
        /// What went wrong.
        message: String,
    },
    /// A file path holds no file name or an unusable one.
    BadPath {
        /// The rejected path.
        path: String,
    },
}

impl Error {
    /// Builds an [`Error::Io`] from a path and the operating-system error.
    pub fn io(path: impl AsRef<std::path::Path>, error: &std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            message: error.to_string(),
        }
    }

    /// Builds an [`Error::Script`] with the source position.
    pub fn script(path: &str, line: u32, column: u32, message: impl Into<String>) -> Self {
        Self::Script {
            line,
            column,
            path: path.to_owned(),
            message: message.into(),
        }
    }

    /// Returns the file path involved, when the variant carries one.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Io { path, .. } | Self::Script { path, .. } | Self::BadPath { path } => {
                Some(path)
            }
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{path}: {message}"),
            Self::NotKeyMagic => {
                f.write_str("not a KeyMagic keyboard file (magic code is not KMKL)")
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(f, "keyboard file version {major}.{minor} is newer than 1.5")
            }
            Self::Truncated { what, offset } => {
                write!(f, "file ends inside {what} at byte {offset}")
            }
            Self::CountMismatch { detail } => write!(f, "keyboard file layout mismatch: {detail}"),
            Self::BadLength {
                what,
                length,
                available,
            } => write!(
                f,
                "{what} declares {length} units but only {available} remain"
            ),
            Self::BadOpcode { opcode, rule } => {
                write!(f, "rule {rule} uses unknown opcode {opcode:#06x}")
            }
            Self::DanglingReference { index, table } => {
                write!(f, "index {index} is outside the {table} table")
            }
            Self::VariableCycle { index } => {
                write!(
                    f,
                    "variable {index} refers to itself through the variable table"
                )
            }
            Self::InvalidKeyboard { detail } => f.write_str(detail),
            Self::Script {
                line,
                column,
                path,
                message,
            } => write!(f, "{path}:{line}:{column}: {message}"),
            Self::BadPath { path } => write!(f, "{path}: path has no usable file name"),
        }
    }
}

impl std::error::Error for Error {}
