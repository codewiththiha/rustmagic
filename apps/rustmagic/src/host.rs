//! The desktop as the session sees it: folders, the clipboard, the browser.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use km_session::Host;

/// Opens the platform's real input-source setup, installing RustMagic first
/// where the platform has a bundled setup command.
pub fn setup_input_method() {
    #[cfg(target_os = "macos")]
    {
        if let Ok(executable) = std::env::current_exe() {
            if let Some(contents) = executable.parent().and_then(Path::parent) {
                let ime = contents
                    .join("Resources/Input Methods/RustMagicIME.app/Contents/MacOS/RustMagicIME");
                if ime.is_file() {
                    let _ = Command::new(ime).arg("--setup").spawn();
                    return;
                }
            }
        }
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.Keyboard-Settings.extension")
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:regionlanguage"])
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        if Command::new("ibus-setup").spawn().is_err() {
            let _ = Command::new("gnome-control-center").arg("region").spawn();
        }
    }
}

/// Where the desktop keeps things, without pulling in a crate to say so.
///
/// Linux follows the XDG base directory spec, macOS the Application Support
/// convention and Windows `APPDATA`, which is what each platform's users expect
/// from `Settings` and `Application Support`.
pub fn state_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map_or_else(|| home.join("RustMagic"), PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.join("Library/Application Support/RustMagic")
    } else {
        std::env::var_os("XDG_STATE_HOME").map_or_else(
            || home.join(".local/state/rustmagic"),
            |base| PathBuf::from(base).join("rustmagic"),
        )
    }
}

/// Where the desktop keeps throwaway files.
pub fn cache_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map_or_else(|| home.join("RustMagic/cache"), PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.join("Library/Caches/RustMagic")
    } else {
        std::env::var_os("XDG_CACHE_HOME").map_or_else(
            || home.join(".cache/rustmagic"),
            |base| PathBuf::from(base).join("rustmagic"),
        )
    }
}

/// The session's host on a real desktop.
pub struct DesktopHost {
    state: PathBuf,
    cache: PathBuf,
    /// Text waiting to be handed to the window's clipboard.
    ///
    /// The session runs before a frame is drawn, so a copy lands here and the
    /// shell picks it up at the start of the next frame, when a context exists.
    clipboard: Arc<Mutex<Option<String>>>,
}

impl DesktopHost {
    /// A host rooted at the platform's own directories.
    pub fn new() -> Self {
        Self {
            state: state_dir(),
            cache: cache_dir(),
            clipboard: Arc::new(Mutex::new(None)),
        }
    }

    /// A handle the shell can poll for clipboard text.
    pub fn clipboard(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.clipboard)
    }
}

impl Default for DesktopHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for DesktopHost {
    fn state_dir(&self) -> PathBuf {
        self.state.clone()
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache.clone()
    }

    fn set_clipboard(&self, text: &str) {
        if let Ok(mut slot) = self.clipboard.lock() {
            *slot = Some(text.to_owned());
        }
    }

    fn open_external(&self, url: &str) {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return;
        }
        let opener = if cfg!(target_os = "windows") {
            "cmd"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let mut command = Command::new(opener);
        if cfg!(target_os = "windows") {
            command.args(["/C", "start", ""]);
        }
        let _ = command.arg(url).spawn();
    }

    fn reveal(&self, path: &Path) {
        if !path.exists() {
            let _ = std::fs::create_dir_all(path);
        }
        self.open_external(&format!("file://{}", path.display()));
        if cfg!(target_os = "linux") {
            let _ = Command::new("xdg-open").arg(path).spawn();
        }
    }
}
