//! Vocabulary shared by every RustMagic crate.
//!
//! This is layer L0: plain data and the error type. It has no dependencies and
//! no I/O, so every other crate can agree on what a key, a modifier, a layout
//! option and a keyboard's metadata are without pulling in the engine.
//!
//! Key codes are Windows virtual-key codes, the numbering that `.km2` files
//! store. Nothing here interprets them.

#![deny(missing_docs)]

mod error;
mod input;
mod keys;
mod layout;
mod meta;

pub use error::{Error, Result};
pub use input::{KeyInput, Modifiers};
pub use keys::{
    ALIASES, CANONICAL, PREDEFINED_COUNT, canonical_id, is_predefined, predefined_alias,
};
pub use keys::{predefined_key, predefined_name, predefined_of, vk};
pub use layout::LayoutOptions;
pub use meta::{Hotkey, KeyboardMeta, info_id, parse_hotkey};
