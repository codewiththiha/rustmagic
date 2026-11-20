//! The two objects an IBus engine puts on the bus.
//!
//! The daemon never talks to a keyboard engine directly. It asks the engine's
//! service for an engine object through a factory, then drives that object with
//! key events and listens for what the engine wants written:
//!
//! ```text
//! daemon ──CreateEngine("rustmagic")──▶ /org/freedesktop/IBus/Factory
//!        ◀──────/org/freedesktop/IBus/Engine/1───────
//!        ──ProcessKeyEvent(keyval, keycode, state)──▶ engine
//!        ◀──────────────CommitText(text)─────────────
//! ```
//!
//! The signatures come from `ibusengine.c` and `ibusfactory.c` in IBus 1.5; the
//! text value is built in [`crate::text`].

// zbus generates code that names the `_`-prefixed parameters, which is exactly
// what this lint reports. The argument names are IBus's, not ours, so the lint is
// turned off for the module rather than the names being made misleading.
#![allow(clippy::used_underscore_binding)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use km_session::Session;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, Value};
use zbus::{interface, Connection};

use crate::app::EngineApp;

/// Where the factory lives.
pub const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
/// Where the first engine lives.
pub const ENGINE_PATH: &str = "/org/freedesktop/IBus/Engine/1";

/// The factory object: it answers for the engine this process hosts.
///
/// IBus asks a service for an engine when the user selects it. There is one
/// engine per process, sharing one composition, so the factory answers with the
/// path it already serves instead of creating a second.
pub struct Factory {
    count: AtomicU32,
}

impl Factory {
    /// A factory for the engine this process serves.
    pub fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
        }
    }

    /// How many times the panel has asked for an engine.
    pub fn creations(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    /// Creates the engine with `name` and returns its object path.
    ///
    /// IBus asks once per activation. The service hosts a single engine, so the
    /// name is accepted whatever it is — a second engine would share the same
    /// keyboard state and confuse the composition.
    fn create_engine(&self, _name: &str) -> ObjectPath<'static> {
        self.count.fetch_add(1, Ordering::Relaxed);
        ObjectPath::try_from(ENGINE_PATH)
            .unwrap_or_else(|_| ObjectPath::from_static_str_unchecked(ENGINE_PATH))
    }
}

/// The engine object the daemon drives.
pub struct EngineService {
    app: Arc<Mutex<EngineApp>>,
}

impl EngineService {
    /// Wraps an application.
    pub fn new(app: Arc<Mutex<EngineApp>>) -> Self {
        Self { app }
    }

    fn with<R>(&self, f: impl FnOnce(&mut EngineApp) -> R) -> R {
        let mut app = self.app.lock().expect("the engine is never poisoned");
        f(&mut app)
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl EngineService {
    /// One key press. Returns whether the engine handled it.
    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(signal_context)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let output = self.with(|app| app.process_key_event(keyval, keycode, state));
        self.publish(&emitter, output).await
    }

    /// The client moved its caret; the engine does not use it yet.
    fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) {
        let _ = (x, y, width, height);
    }

    /// Handwriting is not supported, and saying so is the whole answer.
    fn process_hand_writing_event(&self, _coordinates: Vec<f64>) {}

    /// Handwriting is not supported.
    fn cancel_hand_writing(&self, _strokes: u32) {}

    /// What the surrounding application can do.
    fn set_capabilities(&self, _capabilities: u32) {}

    /// A menu property was clicked; keyboard switches arrive this way.
    fn property_activate(&self, _name: &str, _state: u32) {}

    /// A property should be shown.
    fn property_show(&self, _name: &str) {}

    /// A property should be hidden.
    fn property_hide(&self, _name: &str) {}

    /// A candidate was clicked; no lookup table is shown, so nothing happens.
    fn candidate_clicked(&self, _index: u32, _button: u32, _state: u32) {}

    /// Focus arrived: start from a clean composition.
    async fn focus_in(&self, #[zbus(signal_context)] emitter: SignalEmitter<'_>) {
        let output = self.with(crate::app::EngineApp::focus_in);
        self.publish(&emitter, output).await;
    }

    /// Focus arrived, with the client's identity. Ignored beyond the reset.
    async fn focus_in_id(
        &self,
        _object_path: &str,
        _client: &str,
        #[zbus(signal_context)] emitter: SignalEmitter<'_>,
    ) {
        let output = self.with(crate::app::EngineApp::focus_in);
        self.publish(&emitter, output).await;
    }

    /// Focus left: the composition is committed so nothing is lost.
    async fn focus_out(&self, #[zbus(signal_context)] emitter: SignalEmitter<'_>) {
        let output = self.with(crate::app::EngineApp::focus_out);
        self.publish(&emitter, output).await;
    }

    /// Focus left, with the client's identity.
    async fn focus_out_id(
        &self,
        _object_path: &str,
        #[zbus(signal_context)] emitter: SignalEmitter<'_>,
    ) {
        let output = self.with(crate::app::EngineApp::focus_out);
        self.publish(&emitter, output).await;
    }

    /// The client asks for a clean slate.
    async fn reset(&self, #[zbus(signal_context)] emitter: SignalEmitter<'_>) {
        let output = self.with(crate::app::EngineApp::focus_out);
        self.publish(&emitter, output).await;
    }

    /// The engine became active.
    async fn enable(&self, #[zbus(signal_context)] emitter: SignalEmitter<'_>) {
        let output = self.with(crate::app::EngineApp::reload_keyboard);
        self.publish(&emitter, output).await;
    }

    /// The engine became inactive.
    async fn disable(&self, #[zbus(signal_context)] emitter: SignalEmitter<'_>) {
        let output = self.with(crate::app::EngineApp::focus_out);
        self.publish(&emitter, output).await;
    }

    /// Lookup table navigation: there is no table.
    fn page_up(&self) {}

    /// Lookup table navigation: there is no table.
    fn page_down(&self) {}

    /// Lookup table navigation: there is no table.
    fn cursor_up(&self) {}

    /// Lookup table navigation: there is no table.
    fn cursor_down(&self) {}

    /// The text around the caret. A layout engine does not need it.
    fn set_surrounding_text(&self, _text: Value<'_>, _cursor: u32, _anchor: u32) {}

    /// A panel extension message.
    fn panel_extension_received(&self, _event: Value<'_>) {}

    /// Panel extension keys.
    fn panel_extension_register_keys(&self, _data: Value<'_>) {}

    /// Text to write at the caret.
    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    /// The composition as it stands, shown before the caret.
    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_position: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    /// Auxiliary text under the composition.
    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    /// A key the engine decided not to handle but wants the panel to replay.
    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;
}

impl EngineService {
    /// Sends whatever the engine produced this call.
    async fn publish(&self, emitter: &SignalEmitter<'_>, output: crate::app::Output) -> bool {
        if let Some(commit) = output.commit {
            let _ = Self::commit_text(emitter, crate::text::text(&commit)).await;
        }
        match &output.preedit {
            Some(preedit) => {
                let _ = Self::update_preedit_text(
                    emitter,
                    crate::text::text(preedit),
                    preedit.chars().count() as u32,
                    true,
                    0,
                )
                .await;
            }
            None => {
                let _ =
                    Self::update_preedit_text(emitter, crate::text::text(""), 0, false, 0).await;
            }
        }
        output.consumed
    }
}

/// Starts the service on the session bus and waits forever.
///
/// The daemon spawns this with `--ibus`; the name it must own and the paths it
/// serves are the contract in `assets/ibus/rustmagic.xml`.
pub async fn serve(session: Session, bus_name: &str, engine_name: &str) -> zbus::Result<()> {
    let connection = Connection::session().await?;
    connection.request_name(bus_name).await?;

    let app = Arc::new(Mutex::new(EngineApp::new(session, engine_name)?));
    connection
        .object_server()
        .at(FACTORY_PATH, Factory::new())
        .await?;
    connection
        .object_server()
        .at(ENGINE_PATH, EngineService::new(app))
        .await?;

    // Nothing else to do: the bus drives the object from here. A pending future
    // keeps the connection alive for the life of the process.
    std::future::pending::<()>().await;
    Ok(())
}
