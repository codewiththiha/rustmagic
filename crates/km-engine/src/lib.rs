//! The KeyMagic engine: rules, context and key processing.
//!
//! Layering: this crate depends on [`km_core`] and [`km_binary`] only. It has
//! no I/O beyond loading a keyboard file, no toolkit and no global state, so a
//! desktop app, a CLI and a C adapter all run the same engine.
//!
//! ```
//! let keyboard = km_engine::Keyboard::from_bytes(include_bytes!(
//!     "../../../fixtures/minimal.km2"
//! ))
//! .expect("fixture loads");
//! let mut engine = km_engine::Engine::new(std::sync::Arc::new(keyboard));
//! let outcome = engine.key(km_core::KeyInput::character('a'));
//! assert!(outcome.consumed);
//! assert_eq!(outcome.edit.insert, "\u{1000}");
//! ```

#![deny(missing_docs)]

mod engine;
mod keyboard;
mod rule;

pub use engine::{Edit, Engine, Outcome, MAX_RECURSION};
pub use keyboard::Keyboard;
pub use rule::{Emit, KeyRequirement, Pattern, Rule};
