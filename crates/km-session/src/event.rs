//! What the session tells its client.

use crate::Route;

/// How loud a message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    /// Something worked.
    Info,
    /// Something worked but with a caveat, or a file could not be read.
    Warning,
    /// Something failed and the user must know.
    Error,
}

impl ToastKind {
    /// Whether the message should stay until dismissed.
    pub fn is_urgent(self) -> bool {
        matches!(self, Self::Error)
    }
}

/// Something that happened and the window may want to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A short message for the status bar.
    Toast {
        /// How loud it is.
        kind: ToastKind,
        /// The message.
        text: String,
    },
    /// The selected keyboard changed; `None` when the library is empty.
    ActiveChanged {
        /// The new library id.
        id: Option<String>,
    },
    /// The library directory was rescanned.
    LibraryChanged,
    /// The view changed.
    RouteChanged(Route),
}
