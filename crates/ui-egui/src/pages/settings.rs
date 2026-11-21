//! Settings: the handful of choices the app keeps, and where it keeps them.

use km_session::{Command, Session};
use km_store::Theme;

use crate::theme::Palette;
use crate::widgets;

/// Draws the page.
pub fn show(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) -> bool {
    let mut setup_requested = false;
    widgets::page_title(
        ui,
        palette,
        "Settings",
        "Written to settings.toml the moment you change something.",
    );

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("Appearance")
                .font(crate::theme::bold(13.5))
                .strong()
                .color(palette.text),
        );
        ui.add_space(10.0);
        widgets::setting_row(
            ui,
            palette,
            "Palette",
            "Follow the system, or pin one mood.",
            |ui| {
                if let Some(theme) = widgets::segmented(
                    ui,
                    palette,
                    session.settings().theme,
                    &[
                        (Theme::System, "System"),
                        (Theme::Light, "Light"),
                        (Theme::Dark, "Dark"),
                    ],
                ) {
                    session.execute(Command::SetTheme { theme });
                }
            },
        );
        widgets::separator(ui, palette);
        widgets::setting_row(
            ui,
            palette,
            "Motion",
            "Animate the sandbox and the toast fades.",
            |ui| {
                if widgets::toggle(ui, palette, session.settings().animate).clicked() {
                    let on = !session.settings().animate;
                    session.execute(Command::SetAnimation { on });
                }
            },
        );
    });

    ui.add_space(12.0);

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("Type in other apps")
                .font(crate::theme::bold(13.5))
                .strong()
                .color(palette.text),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Set up RustMagic as a system input source. The next screen shows where to add it; then choose RustMagic from the input menu and select a keyboard.",
            )
            .size(11.5)
            .color(palette.muted),
        );
        ui.add_space(10.0);
        if widgets::quiet_button(ui, palette, "Set up RustMagic…").clicked() {
            setup_requested = true;
        }
    });

    ui.add_space(12.0);

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("Library")
                .font(crate::theme::bold(13.5))
                .strong()
                .color(palette.text),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Folder")
                        .font(crate::theme::bold(14.0))
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(session.library_root().display().to_string())
                        .size(11.5)
                        .monospace()
                        .color(palette.muted),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::quiet_button(ui, palette, "Reveal").clicked() {
                    session.execute(Command::RevealLibrary);
                }
                if widgets::quiet_button(ui, palette, "Rescan").clicked() {
                    session.execute(Command::ReloadLibrary);
                }
            });
        });
        ui.add_space(10.0);
        widgets::separator(ui, palette);
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "{} keyboards installed · buffer of {} characters kept across restarts",
                session.entries().len(),
                session.sandbox().text().chars().count()
            ))
            .size(11.5)
            .color(palette.muted),
        );
    });

    ui.add_space(12.0);

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("Keyboard shortcuts")
                .font(crate::theme::bold(13.5))
                .strong()
                .color(palette.text),
        );
        ui.add_space(8.0);
        for (keys, meaning) in [
            ("Cmd/Ctrl + K", "Command palette"),
            ("Cmd/Ctrl + 1…5", "Switch page"),
            ("Cmd/Ctrl + S", "Save settings now"),
            ("Esc", "Close the palette"),
        ] {
            ui.horizontal(|ui| {
                widgets::pill(ui, palette, keys, palette.accent);
                ui.label(egui::RichText::new(meaning).size(12.0).color(palette.muted));
            });
        }
    });
    setup_requested
}
