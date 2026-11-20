//! Reading and writing the KeyMagic binary keyboard format, `.km2`.
//!
//! A file is a fixed header followed by three tables: variable strings, info
//! entries and rules. Rules hold KeyMagic opcodes, not characters, so this
//! crate is the byte-level layer only: [`read`] validates the container and
//! hands back the opcode words, [`write`] produces a file.
//!
//! See `docs/porting.md` for the byte layout, including the header padding
//! upstream leaves uninitialised and the recovery path for old writers.

#![deny(missing_docs)]

mod model;
mod opcode;
mod read;
mod write;

pub use model::{RawKeyboard, RawRule, Version, Warning};
pub use opcode::{
    OP_AND, OP_ANY, OP_ANYOF, OP_MODIFIER, OP_NANYOF, OP_PREDEFINED, OP_REFERENCE, OP_STRING,
    OP_SWITCH, OP_VARIABLE,
};
pub use read::{read, read_file};
pub use write::{write, write_file};

/// The newest file version this crate writes.
pub const CURRENT_VERSION: Version = Version { major: 1, minor: 5 };

/// The magic code every keyboard file starts with.
pub const MAGIC: [u8; 4] = *b"KMKL";
