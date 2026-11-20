//! The window, drawn with egui.
//!
//! This crate owns no state and makes no decisions: it reads a [`Session`], asks
//! it to run [`Command`]s, and paints what comes back. That split is why the
//! engine can be tested without a window and why a second front end (a CLI, a
//! screenshot bot) needs no changes here.

#![deny(missing_docs)]

mod app;
mod pages;
mod theme;
mod widgets;

pub use app::Ui;
pub use theme::Palette;
pub use widgets::Icon;
