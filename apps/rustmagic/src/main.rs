//! RustMagic, the desktop application.
//!
//! The whole of this crate is wiring: build a session on a real host, hand it to
//! the egui front end, and forward the two things the window cannot do itself
//! (a clipboard and an event loop).

#![deny(missing_docs)]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod host;

use std::sync::{Arc, Mutex};

use eframe::egui;
use km_session::{Command, Session};
use ui_egui::Ui;

/// The eframe shell.
struct Shell {
    session: Session,
    ui: Ui,
    clipboard: Arc<Mutex<Option<String>>>,
}

impl eframe::App for Shell {
    fn update(&mut self, context: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(mut slot) = self.clipboard.lock() {
            if let Some(text) = slot.take() {
                context.copy_text(text);
            }
        }
        if self.ui.show(context, &mut self.session) {
            host::setup_input_method();
        }
    }
}

fn main() -> eframe::Result<()> {
    let host = host::DesktopHost::new();
    let clipboard = host.clipboard();
    // The bundled keyboards are installed by `Session::open`, so there is
    // nothing to seed here.
    let session = Session::open(Box::new(host));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustMagic")
            .with_inner_size([1180.0, 820.0])
            // The window is laid out to survive being dragged small: the rail
            // becomes icons only, the header drops its details, and the keyboard
            // board shrinks its caps. 380 points is where egui's own widgets
            // stop being usable, not where this layout does.
            .with_min_inner_size([380.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RustMagic",
        options,
        Box::new(move |_context| {
            let mut session = session;
            session.execute(Command::Save);
            Ok(Box::new(Shell {
                session,
                ui: Ui::new(),
                clipboard,
            }))
        }),
    )
}
