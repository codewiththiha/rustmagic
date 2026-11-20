//! RustMagic as an input method on Linux.
//!
//! IBus engines are D-Bus services: the daemon asks for an engine object, sends
//! it key events, and listens for the text it wants written. This crate is that
//! service, and nothing else — the rules live in `km-engine`, the state lives in
//! `km-session`, and this crate only translates between the two and the bus.
//!
//! It is deliberately free of C: no glib, no GObject subclassing, no bindings to
//! libibus. The protocol is the interface, so a pure Rust process can be an
//! engine, which in turn means the engine can be tested by speaking to it.

#![deny(missing_docs)]

mod app;
mod keymap;
mod service;
mod text;

pub use app::{EngineApp, Output};
pub use keymap::{is_pure_modifier, is_terminator, map, Mapped};
pub use service::{serve, EngineService, Factory, ENGINE_PATH, FACTORY_PATH};
pub use text::text;
