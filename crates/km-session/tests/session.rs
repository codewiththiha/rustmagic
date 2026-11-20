//! The session, driven the way the window drives it.

use std::path::{Path, PathBuf};

use km_core::Modifiers;
use km_session::{Command, HeadlessHost, Route, Session, ToastKind};
use km_store::Theme;

fn demo_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/keyboards/demo-greek.kms")
}

/// A session with the demo keyboard installed, in a fresh directory.
fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("rustmagic-session-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let mut session = Session::open(Box::new(HeadlessHost::new(root)));
    session.seed(&[demo_source()]);
    session
}

#[test]
fn seeding_installs_the_demo_and_selects_it() {
    let session = session("seed");
    assert_eq!(session.entries().len(), 1);
    let active = session.active().expect("a keyboard is active");
    assert_eq!(active.keyboard.meta.name, "Demo Greek");
    assert!(active.source.is_some(), "the source travelled with it");
    assert_eq!(session.route(), Route::Library);
}

#[test]
fn typing_runs_the_engine_and_fills_the_log() {
    let mut session = session("typing");
    session.execute(Command::ShowRoute {
        route: Route::TryIt,
    });
    session.execute(Command::SandboxType { text: "ab".into() });
    assert_eq!(session.sandbox().text(), "αβ");
    assert_eq!(session.sandbox().log().len(), 2);
    let last = session.sandbox().log().last().expect("a log entry");
    assert!(last.consumed);
    assert_eq!(last.insert, "β");
    assert_eq!(last.context, "αβ");
}

#[test]
fn a_keyboard_that_is_not_active_leaves_no_log() {
    let root = std::env::temp_dir().join("rustmagic-session-empty");
    let _ = std::fs::remove_dir_all(&root);
    let mut session = Session::open(Box::new(HeadlessHost::new(root)));
    session.execute(Command::SandboxType { text: "ab".into() });
    assert_eq!(session.sandbox().log(), []);
    assert_eq!(session.sandbox().text(), "");
}

#[test]
fn caps_lock_arms_one_uppercase_key() {
    let mut session = session("caps");
    session.execute(Command::SandboxKey {
        key: 0x14,
        modifiers: Modifiers::NONE,
    });
    session.execute(Command::SandboxType { text: "a".into() });
    assert_eq!(session.sandbox().text(), "Α");
    session.execute(Command::SandboxType { text: "a".into() });
    assert_eq!(session.sandbox().text(), "Αα", "the switch was spent");
}

#[test]
fn a_shifted_key_rule_beats_the_row_it_sits_on() {
    let mut session = session("shift-rule");
    session.execute(Command::SandboxType { text: "W".into() });
    assert_eq!(session.sandbox().text(), "ς", "Shift+W is the final sigma");
    session.execute(Command::SandboxType { text: "w".into() });
    assert_eq!(session.sandbox().text(), "ςψ");
}

#[test]
fn backspace_undoes_one_step() {
    let mut session = session("backspace");
    session.execute(Command::SandboxType { text: "ab".into() });
    session.execute(Command::SandboxBackspace);
    assert_eq!(session.sandbox().text(), "α", "smart backspace, one step");
}

#[test]
fn the_layout_view_names_what_each_key_produces() {
    let session = session("layout");
    let rows = session.layout();
    assert_eq!(rows.len(), km_session::ROWS.len());
    let qwerty = &rows[1];
    assert_eq!(qwerty.caps[0].label, "q");
    assert_eq!(qwerty.caps[0].plain, "θ", "the eighth Latin key is theta");
    assert_eq!(qwerty.caps[0].shifted, "Θ");
    assert_eq!(
        qwerty.caps[1].plain, "ψ",
        "w is the row's second-to-last position"
    );
    assert_eq!(
        qwerty.caps[1].shifted, "ς",
        "Shift+W is mapped by its own rule"
    );
}

#[test]
fn commands_by_id_reach_the_same_state() {
    let mut session = session("by-id");
    session
        .execute_id("view.route", &[("route", "layout")])
        .expect("route command works");
    assert_eq!(session.route(), Route::Layout);
    session
        .execute_id("sandbox.type", &[("text", "a")])
        .expect("typing works");
    assert_eq!(session.sandbox().text(), "α");
    session
        .execute_id("appearance.theme", &[("theme", "dark")])
        .expect("theme works");
    assert_eq!(session.settings().theme, Theme::Dark);
    assert!(
        session
            .execute_id("view.route", &[("route", "nope")])
            .is_err()
    );
    assert!(session.execute_id::<&str>("nope.nope", &[]).is_err());
}

#[test]
fn removing_the_active_keyboard_moves_to_the_next() {
    let mut session = session("remove");
    session.execute(Command::RemoveKeyboard {
        id: "demo-greek".into(),
    });
    assert_eq!(session.entries(), []);
    assert!(session.active().is_none());
    assert_eq!(session.layout(), []);
}

#[test]
fn importing_a_missing_file_reports_instead_of_failing() {
    let mut session = session("import");
    session.execute(Command::ImportPath {
        path: "/nonexistent/layout.kms".into(),
    });
    let events = session.drain();
    assert!(
        events.iter().any(|event| matches!(
            event,
            km_session::Event::Toast {
                kind: ToastKind::Error,
                ..
            }
        )),
        "an error toast was queued: {events:?}"
    );
}

#[test]
fn settings_and_the_library_survive_a_restart() {
    let root = std::env::temp_dir().join("rustmagic-session-restart");
    let _ = std::fs::remove_dir_all(&root);
    {
        let mut session = Session::open(Box::new(HeadlessHost::new(&root)));
        session.seed(&[demo_source()]);
        session.execute(Command::SandboxType { text: "ab".into() });
        session.execute(Command::SetTheme {
            theme: Theme::Light,
        });
        session.execute(Command::Save);
    }
    let session = Session::open(Box::new(HeadlessHost::new(&root)));
    assert_eq!(session.settings().theme, Theme::Light);
    assert_eq!(
        session.active().map(|active| active.id.as_str()),
        Some("demo-greek")
    );
    assert_eq!(session.sandbox().text(), "αβ", "the buffer resumed");
}
