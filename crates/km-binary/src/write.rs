//! Writing a keyboard file, always at the newest version.

use std::path::Path;

use km_core::{Error, Result};

use crate::model::RawKeyboard;
use crate::MAGIC;

/// Largest table the format can declare: lengths are signed 16-bit.
pub const MAX_UNITS: usize = i16::MAX as usize;

/// Writes a keyboard to disk, creating no directories.
pub fn write_file(path: impl AsRef<Path>, keyboard: &RawKeyboard) -> Result<()> {
    let path = path.as_ref();
    let bytes = write(keyboard)?;
    std::fs::write(path, bytes).map_err(|error| Error::io(path, &error))
}

/// Serialises a keyboard at version 1.5.
///
/// Refuses rather than truncates when a table exceeds what a 16-bit length can
/// describe, so a large layout fails loudly instead of loading wrongly.
pub fn write(keyboard: &RawKeyboard) -> Result<Vec<u8>> {
    check_counts(keyboard)?;
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(&MAGIC);
    out.push(crate::CURRENT_VERSION.major);
    out.push(crate::CURRENT_VERSION.minor);
    out.extend_from_slice(&length(keyboard.strings.len())?.to_le_bytes());
    out.extend_from_slice(&length(keyboard.infos.len())?.to_le_bytes());
    out.extend_from_slice(&length(keyboard.rules.len())?.to_le_bytes());
    out.extend_from_slice(&keyboard.options.to_bytes());
    out.push(0);
    for string in &keyboard.strings {
        out.extend_from_slice(&length(string.len())?.to_le_bytes());
        for unit in string {
            out.extend_from_slice(&unit.to_le_bytes());
        }
    }
    for (id, data) in &keyboard.infos {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&length(data.len())?.to_le_bytes());
        out.extend_from_slice(data);
    }
    for rule in &keyboard.rules {
        for side in [&rule.lhs, &rule.rhs] {
            out.extend_from_slice(&length(side.len())?.to_le_bytes());
            for word in side {
                out.extend_from_slice(&word.to_le_bytes());
            }
        }
    }
    Ok(out)
}

fn length(count: usize) -> Result<i16> {
    i16::try_from(count).map_err(|_| Error::InvalidKeyboard {
        detail: format!("a table of {count} entries does not fit the format's 16-bit length"),
    })
}

fn check_counts(keyboard: &RawKeyboard) -> Result<()> {
    let infos = keyboard.infos.len();
    if infos > MAX_UNITS {
        return Err(Error::InvalidKeyboard {
            detail: format!("{infos} info entries does not fit the format's 16-bit count"),
        });
    }
    Ok(())
}
