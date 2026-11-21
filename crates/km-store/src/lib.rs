//! Where keyboards and settings live on disk.
//!
//! A library is a directory of compiled `.km2` keyboards plus the `.kms`
//! sources they came from. Writes are atomic: a settings file is written beside
//! the real one and renamed, so a crash never leaves half a file behind.

#![deny(missing_docs)]

mod bundled;
mod library;
mod settings;

pub use bundled::{BUNDLED, Bundled};
pub use library::{Entry, Library, safe_id};
pub use settings::{KeyboardMenuMode, Settings, Theme};

/// File name of the settings file inside the state directory.
pub const SETTINGS_FILE: &str = "settings.toml";
