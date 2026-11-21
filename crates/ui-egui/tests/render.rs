//! The window drawn without a screen.
//!
//! egui happily runs headless, so every page is rendered here at a few window
//! sizes in both moods. A page that panics on an empty library or on a very
//! narrow window fails the build before anyone sees a white square.

use std::path::{Path, PathBuf};

use egui::{Context, RawInput, Rect, Vec2};
use km_session::{Command, HeadlessHost, Route, Seed, Session};
use km_store::Theme;
use ui_egui::Ui;

fn demo_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/keyboards/demo-greek.kms")
}

fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("rustmagic-ui-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    Session::open_with(Box::new(HeadlessHost::new(root)), Seed::Nothing)
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

/// Every window size a person can actually drag a window to, both moods, every
/// page. The window is meant to work at all of them, not at the one size it was
/// designed against.
#[test]
fn every_page_draws_at_every_window_size() {
    let sizes = [
        Vec2::new(1600.0, 1000.0),
        Vec2::new(1280.0, 800.0),
        Vec2::new(1180.0, 820.0),
        Vec2::new(1024.0, 700.0),
        Vec2::new(900.0, 640.0),
        Vec2::new(820.0, 600.0),
        Vec2::new(700.0, 560.0),
        Vec2::new(560.0, 480.0),
        Vec2::new(420.0, 420.0),
        Vec2::new(320.0, 360.0),
        Vec2::new(260.0, 300.0),
    ];
    let mut session = session("sizes");
    session.seed(&[demo_source()]);
    session.execute(Command::SandboxType {
        text: "qwer".into(),
    });
    let mut ui = Ui::new();
    for theme in [Theme::Dark, Theme::Light] {
        session.execute(Command::SetTheme { theme });
        for size in sizes {
            for route in Route::ALL {
                session.execute(Command::ShowRoute { route });
                draw(&mut ui, &mut session, size, 1);
            }
        }
    }
}

/// The keyboard board is twelve keys wide, so it has to shrink its caps rather
/// than run past the edge of the window.
#[test]
fn the_keyboard_board_fits_a_narrow_window() {
    let mut session = session("board");
    session.seed(&[demo_source()]);
    session.execute(Command::ShowRoute {
        route: Route::Layout,
    });
    let mut ui = Ui::new();
    for width in [1400.0_f32, 900.0, 700.0, 520.0] {
        let context = Context::default();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(width, 700.0),
            )),
            ..RawInput::default()
        };
        let _ = context.run(input, |context| {
            ui.show(context, &mut session);
        });
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
