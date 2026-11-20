//! The window drawn without a screen.
//!
//! egui happily runs headless, so every page is rendered here at a few window
//! sizes in both moods. A page that panics on an empty library or on a very
//! narrow window fails the build before anyone sees a white square.

use std::path::{Path, PathBuf};

use egui::{Context, RawInput, Rect, Vec2};
use km_session::{Command, HeadlessHost, Route, Session};
use km_store::Theme;
use ui_egui::Ui;

fn demo_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/keyboards/demo-greek.kms")
}

fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("rustmagic-ui-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    Session::open(Box::new(HeadlessHost::new(root)))
}

fn draw(ui: &mut Ui, session: &mut Session, size: Vec2, frames: usize) {
    let context = Context::default();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..RawInput::default()
    };
    for _ in 0..frames {
        let _ = context.run(input.clone(), |context| {
            ui.show(context, session);
        });
        let _ = context.has_requested_repaint();
    }
}

#[test]
fn every_page_draws_in_both_moods() {
    let mut session = session("moods");
    session.seed(&[demo_source()]);
    session.execute(Command::SandboxType {
        text: "qwer".into(),
    });
    let mut ui = Ui::new();
    for theme in [Theme::Dark, Theme::Light] {
        session.execute(Command::SetTheme { theme });
        for route in Route::ALL {
            session.execute(Command::ShowRoute { route });
            draw(&mut ui, &mut session, Vec2::new(1180.0, 820.0), 2);
        }
    }
}

#[test]
fn an_empty_library_still_draws() {
    let mut session = session("empty");
    let mut ui = Ui::new();
    for route in Route::ALL {
        session.execute(Command::ShowRoute { route });
        draw(&mut ui, &mut session, Vec2::new(900.0, 700.0), 2);
    }
}

#[test]
fn a_narrow_window_does_not_panic() {
    let mut session = session("narrow");
    session.seed(&[demo_source()]);
    let mut ui = Ui::new();
    for size in [Vec2::new(420.0, 640.0), Vec2::new(260.0, 420.0)] {
        for route in Route::ALL {
            session.execute(Command::ShowRoute { route });
            draw(&mut ui, &mut session, size, 2);
        }
    }
}

#[test]
fn the_sandbox_pages_follow_the_session() {
    let mut session = session("follow");
    session.seed(&[demo_source()]);
    session.execute(Command::ShowRoute {
        route: Route::TryIt,
    });
    let mut ui = Ui::new();
    draw(&mut ui, &mut session, Vec2::new(1100.0, 760.0), 1);
    session.execute(Command::SandboxType { text: "a".into() });
    draw(&mut ui, &mut session, Vec2::new(1100.0, 760.0), 1);
    assert_eq!(session.sandbox().text(), "α", "the window wrote nothing");
}
