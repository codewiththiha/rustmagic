//! Settings that survive a restart.

use std::path::Path;

use km_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Which palette the window uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the desktop's light or dark preference.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    /// Whether this theme wants a dark palette, given the desktop's answer.
    pub fn is_dark(self, desktop_prefers_dark: bool) -> bool {
        match self {
            Self::System => desktop_prefers_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }

    /// Every theme, for the picker.
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// The label to show for this theme.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// Everything the app remembers between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Id of the selected keyboard, if the library still has it.
    pub active: Option<String>,
    /// Palette choice.
    pub theme: Theme,
    /// Whether transitions play at all.
    pub animate: bool,
    /// Last sandbox text, so the try-it view resumes where it was.
    pub sandbox: String,
}

impl Settings {
    /// Reads settings, falling back to defaults when the file is missing or
    /// unreadable: a broken settings file must never stop the app.
    pub fn load(path: impl AsRef<Path>) -> (Self, Option<Error>) {
        let path = path.as_ref();
        let Ok(text) = std::fs::read_to_string(path) else {
            return (Self::default(), None);
        };
        match toml::from_str::<Self>(&text) {
            Ok(settings) => (settings, None),
            Err(error) => (
                Self::default(),
                Some(Error::InvalidKeyboard {
                    detail: format!("{}: {error}", path.display()),
                }),
            ),
        }
    }

    /// Writes settings atomically, creating the directory when needed.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let text = toml::to_string(self).map_err(|error| Error::InvalidKeyboard {
            detail: format!("settings could not be serialised: {error}"),
        })?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| Error::io(parent, &error))?;
            }
        }
        let temporary = path.with_extension("toml.tmp");
        std::fs::write(&temporary, text).map_err(|error| Error::io(&temporary, &error))?;
        std::fs::rename(&temporary, path).map_err(|error| Error::io(path, &error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_a_file() {
        let dir = std::env::temp_dir().join("rustmagic-settings-test");
        let path = dir.join("settings.toml");
        let settings = Settings {
            active: Some("demo".into()),
            theme: Theme::Dark,
            animate: true,
            sandbox: "ka\u{1000}".into(),
        };
        settings.save(&path).expect("saves");
        let (loaded, error) = Settings::load(&path);
        assert!(error.is_none());
        assert_eq!(loaded, settings);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_yields_defaults_and_a_reason() {
        let dir = std::env::temp_dir().join("rustmagic-settings-broken");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("settings.toml");
        std::fs::write(&path, "theme = [not valid").expect("writes");
        let (settings, error) = Settings::load(&path);
        assert_eq!(settings, Settings::default());
        assert!(error.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let (settings, error) = Settings::load("/nonexistent/rustmagic/settings.toml");
        assert_eq!(settings, Settings::default());
        assert!(error.is_none());
    }

    #[test]
    fn theme_follows_the_desktop_only_when_asked() {
        assert!(Theme::System.is_dark(true));
        assert!(!Theme::Light.is_dark(true));
        assert!(Theme::Dark.is_dark(false));
    }
}
