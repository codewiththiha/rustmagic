//! The machine the engine runs on, behind a trait.
//!
//! Everything platform-specific a session needs crosses this boundary, so the
//! session itself runs in a test with a headless host.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Platform capabilities the session asks for.
pub trait Host: Send {
    /// Where settings and the keyboard library live.
    fn state_dir(&self) -> PathBuf;
    /// Where throwaway files may go.
    fn cache_dir(&self) -> PathBuf;
    /// Opens a URL in the user's browser.
    fn open_external(&self, url: &str);
    /// Puts text on the clipboard.
    fn set_clipboard(&self, text: &str);
    /// Shows a file in the desktop's file manager.
    fn reveal(&self, path: &Path);
}

/// A host that keeps everything in memory, for tests and the CLI.
#[derive(Debug, Clone, Default)]
pub struct HeadlessHost {
    root: PathBuf,
    clipboard: Arc<Mutex<String>>,
    opened: Arc<Mutex<Vec<String>>>,
}

impl HeadlessHost {
    /// A headless host whose state lives under `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            clipboard: Arc::new(Mutex::new(String::new())),
            opened: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// What the last [`Host::set_clipboard`] call put on the clipboard.
    pub fn clipboard(&self) -> String {
        self.clipboard
            .lock()
            .map(|text| text.clone())
            .unwrap_or_default()
    }

    /// Every URL [`Host::open_external`] was asked to open.
    pub fn opened(&self) -> Vec<String> {
        self.opened
            .lock()
            .map(|urls| urls.clone())
            .unwrap_or_default()
    }
}

impl Host for HeadlessHost {
    fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    fn open_external(&self, url: &str) {
        if let Ok(mut opened) = self.opened.lock() {
            opened.push(url.to_owned());
        }
    }

    fn set_clipboard(&self, text: &str) {
        if let Ok(mut clipboard) = self.clipboard.lock() {
            clipboard.clear();
            clipboard.push_str(text);
        }
    }

    fn reveal(&self, _path: &Path) {}
}
