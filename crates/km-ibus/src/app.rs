//! What the engine does with a key, in terms of the session.
//!
//! An input method cannot edit the application's text directly. It shows a
//! *preedit* — the composition so far — and *commits* text when the composition
//! ends. That maps onto this engine's `Edit` model very cleanly:
//!
//! * the composition context *is* the preedit;
//! * a key that the engine consumes updates the preedit;
//! * a key that ends the composition commits the preedit and is handed back.

use std::sync::Arc;

use km_core::{KeyInput, Modifiers};
use km_engine::{Engine, Keyboard};
use km_session::{Command, Session};

use crate::keymap::{self, modifiers};

/// What the service should send after one call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Output {
    /// Text to commit, when the composition ended.
    pub commit: Option<String>,
    /// The composition so far, when it continues.
    pub preedit: Option<String>,
    /// Whether the application should treat the key as handled.
    pub consumed: bool,
}

/// One engine, bound to the session's active keyboard.
pub struct EngineApp {
    session: Session,
    engine: Option<Engine>,
    keyboard: Option<Arc<Keyboard>>,
    engine_name: String,
}

impl EngineApp {
    /// Builds an engine for whatever keyboard the session has active.
    pub fn new(session: Session, engine_name: &str) -> zbus::Result<Self> {
        let mut app = Self {
            session,
            engine: None,
            keyboard: None,
            engine_name: engine_name.to_owned(),
        };
        app.reload_keyboard();
        Ok(app)
    }

    /// Re-reads the active keyboard, keeping a composition only if it survives.
    pub fn reload_keyboard(&mut self) -> Output {
        let Some(active) = self.session.active() else {
            self.engine = None;
            self.keyboard = None;
            return Output::default();
        };
        if self.keyboard.as_deref() == Some(&active.keyboard) {
            return Output::default();
        }
        self.keyboard = Some(Arc::clone(&active.keyboard));
        self.engine = Some(Engine::new(Arc::clone(&active.keyboard)));
        Output::default()
    }

    /// The composition as it stands.
    pub fn composition(&self) -> String {
        self.engine
            .as_ref()
            .map(|engine| engine.context().to_owned())
            .unwrap_or_default()
    }

    /// Ends the composition, returning the text to commit.
    fn take_composition(&mut self) -> Option<String> {
        let text = self.composition();
        if let Some(engine) = self.engine.as_mut() {
            engine.reset();
        }
        if text.is_empty() { None } else { Some(text) }
    }

    /// Commits and starts over, which is what focus loss and Escape do.
    pub fn focus_out(&mut self) -> Output {
        Output {
            commit: self.take_composition(),
            preedit: None,
            consumed: false,
        }
    }

    /// Focus arrived: nothing is pending, so nothing changes.
    pub fn focus_in(&mut self) -> Output {
        let commit = self.take_composition();
        Output {
            commit,
            preedit: None,
            consumed: false,
        }
    }

    /// Interprets one key event.
    pub fn process_key_event(&mut self, keysym: u32, _keycode: u32, state: u32) -> Output {
        if state & modifiers::RELEASE != 0 {
            return Output::default();
        }
        if keymap::is_pure_modifier(keysym) {
            return Output::default();
        }

        let mapped = keymap::map(keysym);
        let held = Modifiers {
            shift: state & modifiers::SHIFT != 0,
            control: state & modifiers::CONTROL != 0,
            alt: state & modifiers::MOD1 != 0,
            alt_gr: state & modifiers::MOD5 != 0,
            caps: state & modifiers::LOCK != 0,
        };

        // A keyboard's own hotkey switches to it, exactly as on Windows.
        if let Some(id) = self.hotkey_target(mapped.key, held) {
            let commit = self.take_composition();
            self.session.execute(Command::SelectKeyboard { id });
            self.reload_keyboard();
            return Output {
                commit,
                preedit: None,
                consumed: true,
            };
        }

        let Some(engine) = self.engine.as_mut() else {
            return Output::default();
        };

        // A terminator ends the syllable: the application types it itself.
        if keymap::is_terminator(keysym) {
            let commit = self.take_composition();
            return Output {
                commit,
                preedit: None,
                consumed: false,
            };
        }

        let input = KeyInput {
            key: mapped.key,
            text: mapped.text,
            modifiers: held,
        };
        let before = engine.context().to_owned();
        let outcome = engine.key(input);
        let composition = engine.context().to_owned();

        if composition.is_empty() {
            // Nothing is being composed any more: commit what was there and let
            // the application deal with the key if the engine did not.
            let commit = if before.is_empty() {
                None
            } else {
                Some(before)
            };
            return Output {
                commit,
                preedit: None,
                consumed: outcome.consumed,
            };
        }
        Output {
            commit: None,
            preedit: Some(composition),
            consumed: true,
        }
    }

    /// Finds a keyboard whose hotkey matches this key.
    fn hotkey_target(&self, key: u16, held: Modifiers) -> Option<String> {
        if key == 0 {
            return None;
        }
        let pressed = held.any();
        self.session
            .entries()
            .iter()
            .filter(|entry| !entry.meta.hotkey.is_empty())
            .find(|entry| {
                let Some(hotkey) = km_core::parse_hotkey(&entry.meta.hotkey) else {
                    return false;
                };
                let matches_key = hotkey.key == key;
                let wants = hotkey.modifiers;
                matches_key
                    && wants.shift == held.shift
                    && wants.control == held.control
                    && wants.alt == held.alt
                    && (pressed || !(wants.shift || wants.control || wants.alt))
            })
            .map(|entry| entry.id.clone())
    }

    /// The engine's name, as IBus knows it.
    pub fn name(&self) -> &str {
        &self.engine_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use km_session::{HeadlessHost, Seed};

    const DEMO: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/keyboards/demo-greek.kms"
    );

    /// Each test gets its own library, so they can run in parallel.
    fn app_named(name: &str) -> EngineApp {
        let root = std::env::temp_dir().join(format!("rustmagic-ibus-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let mut session = Session::open_with(Box::new(HeadlessHost::new(root)), Seed::Nothing);
        session.seed_text("demo-greek", &std::fs::read_to_string(DEMO).unwrap());
        EngineApp::new(session, "rustmagic").expect("an engine")
    }

    fn press(app: &mut EngineApp, keysym: u32) -> Output {
        app.process_key_event(keysym, 0, 0)
    }

    #[test]
    fn a_letter_becomes_a_preedit() {
        let mut app = app_named("letter");
        let output = press(&mut app, 0x0061);
        assert_eq!(output.preedit.as_deref(), Some("α"));
        assert!(output.consumed);
        assert_eq!(output.commit, None);
    }

    #[test]
    fn space_commits_the_syllable() {
        let mut app = app_named("space");
        press(&mut app, 0x0061);
        press(&mut app, 0x0062);
        let output = press(&mut app, 0x0020);
        assert_eq!(
            output.commit.as_deref(),
            Some("αβ"),
            "the whole composition"
        );
        assert_eq!(output.preedit, None);
        assert!(!output.consumed, "the application types the space");
        assert_eq!(app.composition(), "", "the engine started over");
    }

    #[test]
    fn a_shifted_letter_follows_the_layout() {
        let mut app = app_named("shift");
        let output = app.process_key_event(0x0041, 0, modifiers::SHIFT);
        assert_eq!(output.preedit.as_deref(), Some("Α"));
    }

    #[test]
    fn caps_lock_spends_itself_on_one_key() {
        let mut app = app_named("caps");
        assert_eq!(
            press(&mut app, 0xffe5).preedit,
            None,
            "caps lock is a key, not text"
        );
        assert_eq!(press(&mut app, 0x0061).preedit.as_deref(), Some("Α"));
        assert_eq!(press(&mut app, 0x0061).preedit.as_deref(), Some("Αα"));
    }

    #[test]
    fn backspace_undoes_a_step_and_then_leaves() {
        let mut app = app_named("back");
        press(&mut app, 0x0061);
        press(&mut app, 0x0062);
        let output = press(&mut app, 0xff08);
        assert_eq!(output.preedit.as_deref(), Some("α"), "smart backspace");
        let output = press(&mut app, 0xff08);
        assert_eq!(output.commit.as_deref(), Some("α"));
        assert_eq!(output.preedit, None);
    }

    #[test]
    fn focus_loss_commits_rather_than_dropping() {
        let mut app = app_named("focus");
        press(&mut app, 0x0061);
        let output = app.focus_out();
        assert_eq!(output.commit.as_deref(), Some("α"));
        assert_eq!(app.composition(), "");
    }

    #[test]
    fn key_releases_and_modifiers_are_not_ours() {
        let mut app = app_named("keys");
        assert_eq!(
            app.process_key_event(0x0061, 0, modifiers::RELEASE),
            Output::default()
        );
        assert_eq!(press(&mut app, 0xffe1), Output::default());
        assert_eq!(app.composition(), "");
    }

    #[test]
    fn an_unknown_keysym_changes_nothing() {
        let mut app = app_named("unknown");
        assert_eq!(press(&mut app, 0xff20), Output::default());
    }
}
