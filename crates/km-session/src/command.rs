//! Every action the application can perform, registered by id.
//!
//! A command is the only way state changes: the front end, the CLI and the
//! screenshot tool all call [`crate::Session::execute`] with one of these.

use km_core::Modifiers;
use km_store::{KeyboardMenuMode, Theme};

use crate::Route;

/// One application action.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Selects a keyboard from the library.
    SelectKeyboard {
        /// Library id.
        id: String,
    },
    /// Imports a `.kms` or `.km2` file.
    ImportPath {
        /// Path as the file picker returned it.
        path: String,
    },
    /// Deletes a keyboard from the library.
    RemoveKeyboard {
        /// Library id.
        id: String,
    },
    /// Re-reads the library directory.
    ReloadLibrary,
    /// Switches the palette.
    SetTheme {
        /// Chosen theme.
        theme: Theme,
    },
    /// Turns transitions on or off.
    SetAnimation {
        /// Whether motion plays.
        on: bool,
    },
    /// Chooses which keyboard layouts appear in the system input menu.
    SetKeyboardMenuMode {
        /// Active only, all installed layouts, or a selected subset.
        mode: KeyboardMenuMode,
    },
    /// Sets the selected subset of keyboard ids for the system input menu.
    SetKeyboardMenuIds {
        /// Library ids to show.
        ids: Vec<String>,
    },
    /// Shows one view.
    ShowRoute {
        /// The view to show.
        route: Route,
    },
    /// Types text into the try-it buffer.
    SandboxType {
        /// Text to type, one key event per character.
        text: String,
    },
    /// Presses one key in the try-it buffer.
    SandboxKey {
        /// Virtual-key code.
        key: u16,
        /// Modifier state.
        modifiers: Modifiers,
    },
    /// Presses backspace in the try-it buffer.
    SandboxBackspace,
    /// Clears the try-it buffer.
    SandboxReset,
    /// Copies the try-it text to the clipboard.
    SandboxCopy,
    /// Opens the library directory.
    RevealLibrary,
    /// Opens a URL in the user's browser.
    OpenExternal {
        /// The URL.
        url: String,
    },
    /// Writes settings and the library to disk.
    Save,
}

/// What a command means to a person, and what it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// Registered id, `area.verb`.
    pub id: &'static str,
    /// Label for a menu, a palette or a help line.
    pub label: &'static str,
    /// Parameters in order, empty when the command takes none.
    pub params: &'static [&'static str],
}

/// The registry every command has a row in.
pub static SPECS: &[CommandSpec] = &[
    CommandSpec {
        id: "keyboard.select",
        label: "Select a keyboard",
        params: &["id"],
    },
    CommandSpec {
        id: "keyboard.import",
        label: "Import a keyboard file",
        params: &["path"],
    },
    CommandSpec {
        id: "keyboard.remove",
        label: "Remove a keyboard",
        params: &["id"],
    },
    CommandSpec {
        id: "library.reload",
        label: "Reload the library",
        params: &[],
    },
    CommandSpec {
        id: "library.reveal",
        label: "Show the library folder",
        params: &[],
    },
    CommandSpec {
        id: "appearance.theme",
        label: "Switch the palette",
        params: &["theme"],
    },
    CommandSpec {
        id: "appearance.animate",
        label: "Turn transitions on or off",
        params: &["on"],
    },
    CommandSpec {
        id: "keyboard-menu.mode",
        label: "Choose input menu keyboard visibility",
        params: &["mode"],
    },
    CommandSpec {
        id: "keyboard-menu.ids",
        label: "Choose keyboards in the input menu",
        params: &["ids"],
    },
    CommandSpec {
        id: "view.route",
        label: "Open a view",
        params: &["route"],
    },
    CommandSpec {
        id: "sandbox.type",
        label: "Type into the try-it buffer",
        params: &["text"],
    },
    CommandSpec {
        id: "sandbox.key",
        label: "Press a key in the try-it buffer",
        params: &["key", "shift", "control", "alt", "alt_gr"],
    },
    CommandSpec {
        id: "sandbox.backspace",
        label: "Backspace in the try-it buffer",
        params: &[],
    },
    CommandSpec {
        id: "sandbox.reset",
        label: "Clear the try-it buffer",
        params: &[],
    },
    CommandSpec {
        id: "sandbox.copy",
        label: "Copy the try-it text",
        params: &[],
    },
    CommandSpec {
        id: "app.open-external",
        label: "Open a link",
        params: &["url"],
    },
    CommandSpec {
        id: "app.save",
        label: "Save settings now",
        params: &[],
    },
];

impl Command {
    /// The registered id of this command.
    ///
    /// The match is exhaustive so a new command cannot exist without an id.
    pub fn id(&self) -> &'static str {
        match self {
            Self::SelectKeyboard { .. } => "keyboard.select",
            Self::ImportPath { .. } => "keyboard.import",
            Self::RemoveKeyboard { .. } => "keyboard.remove",
            Self::ReloadLibrary => "library.reload",
            Self::RevealLibrary => "library.reveal",
            Self::SetTheme { .. } => "appearance.theme",
            Self::SetAnimation { .. } => "appearance.animate",
            Self::SetKeyboardMenuMode { .. } => "keyboard-menu.mode",
            Self::SetKeyboardMenuIds { .. } => "keyboard-menu.ids",
            Self::ShowRoute { .. } => "view.route",
            Self::SandboxType { .. } => "sandbox.type",
            Self::SandboxKey { .. } => "sandbox.key",
            Self::SandboxBackspace => "sandbox.backspace",
            Self::SandboxReset => "sandbox.reset",
            Self::SandboxCopy => "sandbox.copy",
            Self::OpenExternal { .. } => "app.open-external",
            Self::Save => "app.save",
        }
    }

    /// The registry row for an id.
    pub fn spec(id: &str) -> Option<&'static CommandSpec> {
        SPECS.iter().find(|spec| spec.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every command, so the registry test cannot miss one.
    fn every_command() -> Vec<Command> {
        vec![
            Command::SelectKeyboard { id: "a".into() },
            Command::ImportPath { path: "a".into() },
            Command::RemoveKeyboard { id: "a".into() },
            Command::ReloadLibrary,
            Command::RevealLibrary,
            Command::SetTheme { theme: Theme::Dark },
            Command::SetAnimation { on: true },
            Command::SetKeyboardMenuMode {
                mode: KeyboardMenuMode::Selected,
            },
            Command::SetKeyboardMenuIds {
                ids: vec!["myanmar3".into()],
            },
            Command::ShowRoute {
                route: Route::TryIt,
            },
            Command::SandboxType { text: "a".into() },
            Command::SandboxKey {
                key: 0x41,
                modifiers: Modifiers::NONE,
            },
            Command::SandboxBackspace,
            Command::SandboxReset,
            Command::SandboxCopy,
            Command::OpenExternal { url: "a".into() },
            Command::Save,
        ]
    }

    #[test]
    fn every_command_has_a_registry_row() {
        for command in every_command() {
            let spec = Command::spec(command.id());
            assert!(spec.is_some(), "{} has no row", command.id());
        }
    }

    #[test]
    fn no_registry_row_is_unreachable() {
        let ids: BTreeSet<&str> = every_command().iter().map(Command::id).collect();
        for spec in SPECS {
            assert!(ids.contains(spec.id), "{} is a dead row", spec.id);
        }
    }

    #[test]
    fn ids_are_unique_and_well_formed() {
        let mut seen = BTreeSet::new();
        for spec in SPECS {
            assert!(seen.insert(spec.id), "{} is registered twice", spec.id);
            assert!(
                spec.id.split('.').count() == 2,
                "{} is not area.verb",
                spec.id
            );
            assert_ne!(spec.label, "");
        }
    }
}
