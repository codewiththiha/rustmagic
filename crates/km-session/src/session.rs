//! The application without a window: state, commands, events, persistence.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use km_core::{Error, KeyInput, LayoutOptions, Modifiers};
use km_engine::{Engine, Keyboard};
use km_store::{Entry, Library, Settings, Theme};

use crate::command::Command;
use crate::event::{Event, ToastKind};
use crate::host::Host;
use crate::probe::{self, KeyRow};
use crate::sandbox::Sandbox;

/// Which view the window shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Route {
    /// The installed keyboards.
    #[default]
    Library,
    /// The typing sandbox.
    TryIt,
    /// The key-by-key layout.
    Layout,
    /// The active keyboard's source and metadata.
    Source,
    /// Settings.
    Settings,
}

impl Route {
    /// Every route, in navigation order.
    pub const ALL: [Self; 5] = [
        Self::Library,
        Self::TryIt,
        Self::Layout,
        Self::Source,
        Self::Settings,
    ];

    /// The route's name in the sidebar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::TryIt => "Try it",
            Self::Layout => "Layout",
            Self::Source => "Source",
            Self::Settings => "Settings",
        }
    }

    /// The id used by `view.route` and by the `route=` preview parameter.
    pub fn id(self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::TryIt => "try-it",
            Self::Layout => "layout",
            Self::Source => "source",
            Self::Settings => "settings",
        }
    }

    /// Parses a route id back to a route.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|route| route.id() == id)
    }
}

/// The keyboard the session is driving.
#[derive(Clone)]
pub struct Active {
    /// Library id it came from.
    pub id: String,
    /// The decoded keyboard.
    pub keyboard: Arc<Keyboard>,
    /// Its `.kms` source, when the library kept one.
    pub source: Option<String>,
}

impl std::fmt::Debug for Active {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Active")
            .field("id", &self.id)
            .field("rules", &self.keyboard.rule_count())
            .finish_non_exhaustive()
    }
}

/// Everything that outlives a frame, owned in one place.
pub struct Session {
    host: Box<dyn Host>,
    library: Library,
    settings: Settings,
    settings_path: PathBuf,
    entries: Vec<Entry>,
    active: Option<Active>,
    sandbox: Sandbox,
    route: Route,
    layout: Vec<KeyRow>,
    events: Vec<Event>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("route", &self.route)
            .field("entries", &self.entries.len())
            .field("active", &self.active.as_ref().map(|active| &active.id))
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Opens the session: reads settings, scans the library, selects a keyboard.
    pub fn open(host: Box<dyn Host>) -> Self {
        let state = host.state_dir();
        let library = Library::new(state.join("library"));
        let settings_path = state.join(km_store::SETTINGS_FILE);
        let (settings, problem) = Settings::load(&settings_path);
        let entries = library.list();
        let mut session = Self {
            host,
            library,
            settings,
            settings_path,
            entries,
            active: None,
            sandbox: Sandbox::default(),
            route: Route::Library,
            layout: Vec::new(),
            events: Vec::new(),
        };
        session.sandbox.set_text(&session.settings.sandbox.clone());
        let wanted = session.settings.active.clone();
        let chosen = wanted
            .filter(|id| session.entries.iter().any(|entry| &entry.id == id))
            .or_else(|| session.entries.first().map(|entry| entry.id.clone()));
        if let Some(id) = chosen {
            session.activate(&id);
        }
        if let Some(problem) = problem {
            session.toast(ToastKind::Warning, problem.to_string());
        }
        session
    }

    /// The platform the session runs on.
    pub fn host(&self) -> &dyn Host {
        self.host.as_ref()
    }

    /// Installed keyboards.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The selected keyboard.
    pub fn active(&self) -> Option<&Active> {
        self.active.as_ref()
    }

    /// The try-it buffer.
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    /// The view being shown.
    pub fn route(&self) -> Route {
        self.route
    }

    /// Settings as they stand.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The active keyboard's key caps, empty when no keyboard is active.
    pub fn layout(&self) -> &[KeyRow] {
        &self.layout
    }

    /// Decodes a keyboard straight from the library, without selecting it.
    pub fn keyboard(&self, id: &str) -> Result<Keyboard, Error> {
        self.library.load(id)
    }

    /// The `.kms` text the library kept for a keyboard.
    pub fn library_source(&self, id: &str) -> Option<String> {
        self.library.source_of(id)
    }

    /// Where the library lives, for the reveal command.
    pub fn library_root(&self) -> &Path {
        self.library.root()
    }

    /// Takes the events queued since the last call.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Installs any of `sources` whose keyboard is not in the library yet.
    ///
    /// The app calls this on first run so a fresh install has something to try.
    pub fn seed(&mut self, sources: &[PathBuf]) {
        for source in sources {
            let Some(id) = source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| km_store::safe_id(stem).ok())
            else {
                continue;
            };
            if self.entries.iter().any(|entry| entry.id == id) {
                continue;
            }
            match self.library.install_source(source) {
                Ok(_) => self.reload_library(),
                Err(error) => self.toast(ToastKind::Warning, error.to_string()),
            }
        }
        if self.active.is_none() {
            if let Some(id) = self.entries.first().map(|entry| entry.id.clone()) {
                self.activate(&id);
            }
        }
    }

    /// Installs a keyboard from source text that ships with the app.
    ///
    /// The window has no file to copy on first run, so the source is compiled
    /// here and written straight into the library.
    pub fn seed_text(&mut self, name: &str, source: &str) {
        if self
            .entries
            .iter()
            .any(|entry| km_store::safe_id(name).is_ok_and(|id| id == entry.id))
        {
            return;
        }
        let compiled = match km_script::compile(source, name, None) {
            Ok(compiled) => compiled,
            Err(error) => {
                self.toast(ToastKind::Warning, error.to_string());
                return;
            }
        };
        let bytes = match compiled.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.toast(ToastKind::Warning, error.to_string());
                return;
            }
        };
        match self.library.install_source_text(name, &bytes, source) {
            Ok(_) => self.reload_library(),
            Err(error) => self.toast(ToastKind::Warning, error.to_string()),
        }
        if self.active.is_none() {
            if let Some(id) = self.entries.first().map(|entry| entry.id.clone()) {
                self.activate(&id);
            }
        }
    }

    /// Runs one command. Errors become events; nothing here panics.
    pub fn execute(&mut self, command: Command) {
        match command {
            Command::SelectKeyboard { id } => self.activate(&id),
            Command::ImportPath { path } => self.import(Path::new(&path)),
            Command::RemoveKeyboard { id } => self.remove(&id),
            Command::ReloadLibrary => self.reload_library(),
            Command::RevealLibrary => self.host.reveal(self.library.root()),
            Command::SetTheme { theme } => {
                self.settings.theme = theme;
                self.persist();
            }
            Command::SetAnimation { on } => {
                self.settings.animate = on;
                self.persist();
            }
            Command::ShowRoute { route } => self.show(route),
            Command::SandboxType { text } => {
                self.sandbox.type_text(&text);
                self.after_sandbox_key();
            }
            Command::SandboxKey { key, modifiers } => {
                self.sandbox.key(KeyInput {
                    key,
                    text: char::from_u32(u32::from(key))
                        .filter(|char| !char.is_control() && u32::from(*char) >= 0x20),
                    modifiers,
                });
                self.after_sandbox_key();
            }
            Command::SandboxBackspace => {
                self.sandbox.key(KeyInput::backspace());
                self.after_sandbox_key();
            }
            Command::SandboxReset => {
                self.sandbox.reset();
                self.settings.sandbox.clear();
                self.persist();
            }
            Command::SandboxCopy => {
                let text = self.sandbox.text().to_owned();
                self.host.set_clipboard(&text);
                self.toast(ToastKind::Info, "Copied the try-it text");
            }
            Command::OpenExternal { url } => self.host.open_external(&url),
            Command::Save => self.persist(),
        }
    }

    /// Runs a registered command by id, with its parameters as text.
    ///
    /// This is the door the CLI and the preview use, so every state the window
    /// can reach by clicking is reachable by name too.
    pub fn execute_id<S: AsRef<str>>(
        &mut self,
        id: &str,
        params: &[(&str, S)],
    ) -> Result<(), Error> {
        let get = |name: &str| {
            params
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_ref().to_owned())
        };
        let flag = |name: &str| {
            get(name).is_some_and(|value| matches!(value.as_str(), "1" | "true" | "on" | "yes"))
        };
        let command = match id {
            "keyboard.select" => Command::SelectKeyboard {
                id: get("id").ok_or_else(|| missing("id"))?,
            },
            "keyboard.import" => Command::ImportPath {
                path: get("path").ok_or_else(|| missing("path"))?,
            },
            "keyboard.remove" => Command::RemoveKeyboard {
                id: get("id").ok_or_else(|| missing("id"))?,
            },
            "library.reload" => Command::ReloadLibrary,
            "library.reveal" => Command::RevealLibrary,
            "appearance.theme" => Command::SetTheme {
                theme: match get("theme").as_deref() {
                    Some("light") => Theme::Light,
                    Some("dark") => Theme::Dark,
                    Some("system") | None => Theme::System,
                    Some(other) => {
                        return Err(Error::InvalidKeyboard {
                            detail: format!("`{other}` is not a theme"),
                        });
                    }
                },
            },
            "appearance.animate" => Command::SetAnimation { on: flag("on") },
            "view.route" => Command::ShowRoute {
                route: get("route")
                    .as_deref()
                    .and_then(Route::from_id)
                    .ok_or_else(|| missing("route"))?,
            },
            "sandbox.type" => Command::SandboxType {
                text: get("text").unwrap_or_default(),
            },
            "sandbox.key" => Command::SandboxKey {
                key: get("key")
                    .and_then(|key| key.parse::<u16>().ok())
                    .ok_or_else(|| missing("key"))?,
                modifiers: Modifiers {
                    shift: flag("shift"),
                    control: flag("control"),
                    alt: flag("alt"),
                    alt_gr: flag("alt_gr"),
                    caps: flag("caps"),
                },
            },
            "sandbox.backspace" => Command::SandboxBackspace,
            "sandbox.reset" => Command::SandboxReset,
            "sandbox.copy" => Command::SandboxCopy,
            "app.open-external" => Command::OpenExternal {
                url: get("url").ok_or_else(|| missing("url"))?,
            },
            "app.save" => Command::Save,
            other => {
                return Err(Error::InvalidKeyboard {
                    detail: format!("`{other}` is not a registered command"),
                });
            }
        };
        self.execute(command);
        Ok(())
    }

    fn show(&mut self, route: Route) {
        self.route = route;
        self.events.push(Event::RouteChanged(route));
    }

    /// Selects a keyboard from the library and rebuilds what depends on it.
    fn activate(&mut self, id: &str) {
        match self.library.load(id) {
            Ok(keyboard) => {
                let keyboard = Arc::new(keyboard);
                let source = self.library.source_of(id);
                self.layout = probe::layout_rows(&keyboard);
                self.sandbox
                    .attach(Some(Engine::new(Arc::clone(&keyboard))));
                self.active = Some(Active {
                    id: id.to_owned(),
                    keyboard,
                    source,
                });
                self.settings.active = Some(id.to_owned());
                self.persist();
                self.events.push(Event::ActiveChanged {
                    id: Some(id.to_owned()),
                });
            }
            Err(error) => {
                self.toast(ToastKind::Error, error.to_string());
            }
        }
    }

    fn import(&mut self, path: &Path) {
        let result = if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("kms"))
        {
            self.library.install_source(path)
        } else {
            match std::fs::read(path) {
                Ok(bytes) => {
                    let name = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("keyboard");
                    self.library.install_bytes(name, &bytes)
                }
                Err(error) => Err(Error::io(path, &error)),
            }
        };
        match result {
            Ok(entry) => {
                self.reload_library();
                self.activate(&entry.id);
                self.toast(
                    ToastKind::Info,
                    format!("Installed {}", entry.display_name()),
                );
            }
            Err(error) => self.toast(ToastKind::Error, error.to_string()),
        }
    }

    fn remove(&mut self, id: &str) {
        match self.library.remove(id) {
            Ok(()) => {
                let was_active = self.active.as_ref().is_some_and(|active| active.id == id);
                self.reload_library();
                if was_active {
                    self.active = None;
                    self.layout.clear();
                    self.sandbox.attach(None);
                    if let Some(next) = self.entries.first().map(|entry| entry.id.clone()) {
                        self.activate(&next);
                    } else {
                        self.events.push(Event::ActiveChanged { id: None });
                    }
                }
                self.toast(ToastKind::Info, "Removed the keyboard");
            }
            Err(error) => self.toast(ToastKind::Error, error.to_string()),
        }
    }

    fn reload_library(&mut self) {
        self.entries = self.library.list();
        self.events.push(Event::LibraryChanged);
    }

    /// Keeps the sandbox text in settings so a restart resumes it.
    fn after_sandbox_key(&mut self) {
        let text = self.sandbox.text();
        if self.settings.sandbox != text {
            self.settings.sandbox.clear();
            self.settings.sandbox.push_str(text);
            self.persist();
        }
    }

    fn persist(&mut self) {
        if let Err(error) = self.settings.save(&self.settings_path) {
            self.toast(ToastKind::Error, error.to_string());
        }
    }

    fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.events.push(Event::Toast {
            kind,
            text: text.into(),
        });
    }

    /// Options of the active keyboard, if any.
    pub fn active_options(&self) -> Option<LayoutOptions> {
        self.active.as_ref().map(|active| active.keyboard.options)
    }
}

fn missing(param: &str) -> Error {
    Error::InvalidKeyboard {
        detail: format!("missing parameter `{param}`"),
    }
}
