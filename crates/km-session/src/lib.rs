//! The application without a window.
//!
//! One owner: [`Session`] holds every piece of state that outlives a frame or
//! reaches disk. One door: [`Session::execute`] is the only way it changes. The
//! front end only draws it and sends commands, which is what makes the same
//! session drivable from a CLI, a test and a screenshot tool.

#![deny(missing_docs)]

mod command;
mod event;
mod host;
mod probe;
mod sandbox;
mod session;

pub use command::{Command, CommandSpec, SPECS};
pub use event::{Event, ToastKind};
pub use host::{HeadlessHost, Host};
pub use probe::{KeyCap, KeyRow, ROWS};
pub use sandbox::{KeyEntry, LOG_LIMIT, Sandbox};
pub use session::{Active, Route, Seed, Session};
