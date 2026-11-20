//! Reading a `.km2` file into its tables, with recovery for old writers.

use std::path::Path;

use km_core::{Error, LayoutOptions, Result};

use crate::model::{RawKeyboard, RawRule, Version, Warning};
use crate::MAGIC;

/// Reads a keyboard file from disk.
pub fn read_file(path: impl AsRef<Path>) -> Result<RawKeyboard> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|error| Error::io(path, &error))?;
    read(&bytes)
}

/// Reads a keyboard from memory.
///
/// Fails with [`Error::CountMismatch`] when the header's counts, the declared
/// sections and the file length cannot all be true at once, unless a single
/// consistent reading exists, in which case that reading is used and a
/// [`Warning`] records the repair.
pub fn read(bytes: &[u8]) -> Result<RawKeyboard> {
    let header = Header::read(bytes)?;
    match header.parse(
        bytes,
        header.string_count,
        header.info_count,
        Some(header.rule_count),
    ) {
        Ok(keyboard) => Ok(keyboard),
        Err(primary) => header.recover(bytes).ok_or(primary),
    }
}

/// Section counts and flags as the header declares them.
struct Header {
    version: Version,
    options: LayoutOptions,
    string_count: usize,
    info_count: usize,
    rule_count: usize,
}

impl Header {
    fn read(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        if reader.take(4, "magic code")? != MAGIC {
            return Err(Error::NotKeyMagic);
        }
        let version = Version::new(reader.u8("major version")?, reader.u8("minor version")?);
        if version.major != 1 || !(3..=5).contains(&version.minor) {
            return Err(Error::UnsupportedVersion {
                major: version.major,
                minor: version.minor,
            });
        }
        let string_count = usize::from(reader.u16("string count")?);
        let (info_count, rule_count) = if version.has_infos() {
            (
                usize::from(reader.u16("info count")?),
                usize::from(reader.u16("rule count")?),
            )
        } else {
            (0, usize::from(reader.u16("rule count")?))
        };
        let mut flags = [0u8; 5];
        let known = if version.has_right_alt() { 5 } else { 4 };
        for slot in flags.iter_mut().take(known) {
            *slot = reader.u8("layout option")?;
        }
        if version.has_right_alt() {
            reader.u8("header padding")?;
        } else {
            flags[4] = 1;
        }
        let options = LayoutOptions::from_bytes(flags)?;
        Ok(Self {
            version,
            options,
            string_count,
            info_count,
            rule_count,
        })
    }

    /// Parses the tables after the header, requiring them to fill the file.
    fn parse(
        &self,
        bytes: &[u8],
        string_count: usize,
        info_count: usize,
        rule_count: Option<usize>,
    ) -> Result<RawKeyboard> {
        let mut reader = Reader::at(bytes, self.version.header_len());
        let strings = read_strings(&mut reader, string_count)?;
        let infos = read_infos(&mut reader, info_count)?;
        let rules = read_rules(&mut reader, rule_count)?;
        reader.finish()?;
        Ok(RawKeyboard {
            version: self.version,
            options: self.options,
            strings,
            infos,
            rules,
            warnings: Vec::new(),
        })
    }

    /// Finds the one string/info split that consumes the file exactly.
    ///
    /// Old writers put different fields in the same header slots, so the fix is
    /// to trust the file's own structure: strings first, then a small number of
    /// info entries, then rules that must end exactly at the last byte.
    fn recover(&self, bytes: &[u8]) -> Option<RawKeyboard> {
        // A count that the file could actually hold is a corruption, not an
        // old writer, and guessing a different layout would hide it.
        if self.rule_count.saturating_mul(MIN_RULE_BYTES) <= bytes.len() {
            return None;
        }
        for info_count in 0..=MAX_REPAIRED_INFOS {
            let Ok(mut keyboard) = self.parse(bytes, self.string_count, info_count, None) else {
                continue;
            };
            keyboard.warnings.push(Warning {
                rule: None,
                message: format!(
                    "header declared {declared} info entries but the file holds {info_count}",
                    declared = self.info_count
                ),
            });
            return Some(keyboard);
        }
        None
    }
}

/// How many info entries the recovery path will search for.
const MAX_REPAIRED_INFOS: usize = 64;

/// Smallest possible rule: two side lengths and no words.
const MIN_RULE_BYTES: usize = 4;

fn read_strings(reader: &mut Reader<'_>, count: usize) -> Result<Vec<Vec<u16>>> {
    let mut strings = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        let length = reader.u16("string entry")?;
        let length = usize::from(length);
        strings.push(read_units(reader, length, "string")?);
    }
    Ok(strings)
}

fn read_infos(reader: &mut Reader<'_>, count: usize) -> Result<Vec<(i32, Vec<u8>)>> {
    let mut infos = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        let id = reader.i32("info id")?;
        let length = usize::from(reader.u16("info length")?);
        let data = reader.take(length, "info data")?.to_vec();
        infos.push((id, data));
    }
    Ok(infos)
}

/// Reads rules; `count` is `None` to walk to the end of the file.
fn read_rules(reader: &mut Reader<'_>, count: Option<usize>) -> Result<Vec<RawRule>> {
    let mut rules = Vec::new();
    loop {
        if let Some(count) = count {
            if rules.len() >= count {
                break;
            }
            if reader.exhausted() {
                return Err(Error::CountMismatch {
                    detail: format!(
                        "declared {count} rules but the file ends after {}",
                        rules.len()
                    ),
                });
            }
        } else if reader.exhausted() {
            break;
        }
        let index = rules.len();
        let lhs_len = usize::from(reader.u16("left side length")?);
        let lhs = read_units(reader, lhs_len, "left side")?;
        let rhs_len = usize::from(reader.u16("right side length")?);
        let rhs = read_units(reader, rhs_len, "right side")?;
        rules.push(RawRule { index, lhs, rhs });
    }
    Ok(rules)
}

fn read_units(reader: &mut Reader<'_>, length: usize, what: &'static str) -> Result<Vec<u16>> {
    if length > reader.remaining() / 2 {
        return Err(Error::BadLength {
            what,
            length,
            available: reader.remaining() / 2,
        });
    }
    let mut units = Vec::with_capacity(length);
    for _ in 0..length {
        units.push(reader.u16(what)?);
    }
    Ok(units)
}

/// A bounds-checked cursor; every read either advances or fails.
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn at(bytes: &'a [u8], offset: usize) -> Self {
        Self { bytes, offset }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    const fn exhausted(&self) -> bool {
        self.remaining() == 0
    }

    fn take(&mut self, length: usize, what: &'static str) -> Result<&'a [u8]> {
        if length > self.remaining() {
            return Err(Error::Truncated {
                what,
                offset: self.offset,
            });
        }
        let start = self.offset;
        self.offset += length;
        Ok(&self.bytes[start..self.offset])
    }

    fn u8(&mut self, what: &'static str) -> Result<u8> {
        Ok(self.take(1, what)?[0])
    }

    fn u16(&mut self, what: &'static str) -> Result<u16> {
        let bytes = self.take(2, what)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn i32(&mut self, what: &'static str) -> Result<i32> {
        let bytes = self.take(4, what)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn finish(&self) -> Result<()> {
        if self.exhausted() {
            Ok(())
        } else {
            Err(Error::CountMismatch {
                detail: format!("{} bytes after the last rule", self.remaining()),
            })
        }
    }
}
