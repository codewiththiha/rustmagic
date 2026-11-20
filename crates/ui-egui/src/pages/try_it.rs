//! The sandbox: type, see exactly what the engine did, key by key.

use egui::{Rounding, Vec2};

use km_core::Modifiers;
use km_session::{Command, KeyEntry, Session};

use crate::theme::Palette;
use crate::widgets;

/// The chips above the composition surface, each one a party trick of the
/// active keyboard rather than a random string.
const SAMPLES: &[(&str, &str)] = &[
    ("Type a word", "mingalaba"),
    ("Mixed case", "RustMagic"),
    ("Numbers", "12345"),
    ("Punctuation", "a-b_c.d"),
];

/// Draws the page.
pub fn show(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
    widgets::page_title(
        ui,
        palette,
        "Try it",
        "Every key press goes through the engine. The panel below is its diary.",
    );

    if session.active().is_none() {
        widgets::card(palette).show(ui, |ui| {
            ui.label(
                egui::RichText::new("Install a keyboard first, on the Library page.")
                    .size(13.0)
                    .color(palette.muted),
            );
        });
        return;
    }

    composition(ui, session, palette);
    ui.add_space(12.0);
    controls(ui, session, palette);
    ui.add_space(16.0);
    log(ui, session, palette);
}

fn composition(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
    widgets::paper(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let text = session.sandbox().text().to_owned();
        let typing = !ui.ctx().wants_keyboard_input()
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Text(_) | egui::Event::Key { pressed: true, .. }
                    )
                })
            });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("composition")
                    .size(10.5)
                    .color(palette.muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widgets::pill(
                    ui,
                    palette,
                    &format!("{} chars", text.chars().count()),
                    palette.accent,
                );
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if text.is_empty() {
                ui.label(
                    egui::RichText::new("Start typing on your keyboard…")
                        .size(26.0)
                        .color(palette.muted),
                );
            } else {
                ui.label(egui::RichText::new(&text).size(28.0).color(palette.text));
            }
            let caret = ui.allocate_exact_size(Vec2::new(3.0, 30.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(caret.0, Rounding::same(2.0), palette.accent);
        });
        ui.add_space(6.0);

        // The window is not a text field, so the page collects the keystrokes
        // the physical keyboard sent this frame and hands them to the engine.
        if typing {
            let mut typed = String::new();
            let mut backspace = false;
            ui.input(|input| {
                for event in &input.events {
                    match event {
                        egui::Event::Text(text) => typed.push_str(text),
                        egui::Event::Key {
                            key: egui::Key::Backspace,
                            pressed: true,
                            ..
                        } => backspace = true,
                        _ => {}
                    }
                }
            });
            if !typed.is_empty() {
                session.execute(Command::SandboxType { text: typed });
            }
            if backspace {
                session.execute(Command::SandboxBackspace);
            }
        }
    });
}

fn controls(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
    ui.horizontal_wrapped(|ui| {
        for (label, text) in SAMPLES {
            if widgets::quiet_button(ui, palette, label).clicked() {
                let mut sample = text.to_string();
                if *label == "Mixed case" {
                    sample = sample.to_uppercase();
                }
                session.execute(Command::SandboxType { text: sample });
            }
        }
        ui.add_space(6.0);
        if widgets::quiet_button(ui, palette, "⌫ Backspace").clicked() {
            session.execute(Command::SandboxBackspace);
        }
        if widgets::quiet_button(ui, palette, "Caps lock").clicked() {
            session.execute(Command::SandboxKey {
                key: 0x14,
                modifiers: Modifiers::NONE,
            });
        }
        if widgets::quiet_button(ui, palette, "Clear").clicked() {
            session.execute(Command::SandboxReset);
        }
        if widgets::primary_button(ui, palette, "Copy").clicked() {
            session.execute(Command::SandboxCopy);
        }
    });
}

fn log(ui: &mut egui::Ui, session: &Session, palette: &Palette) {
    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Engine log")
                    .size(13.5)
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                egui::RichText::new("newest first, last two hundred keys")
                    .size(11.0)
                    .color(palette.muted),
            );
        });
        ui.add_space(8.0);
        let entries: Vec<&KeyEntry> = session.sandbox().log().iter().rev().take(40).collect();
        if entries.is_empty() {
            ui.label(
                egui::RichText::new("Nothing typed yet. Press a key.")
                    .size(12.0)
                    .color(palette.muted),
            );
            return;
        }
        egui::Grid::new("log")
            .num_columns(4)
            .spacing(Vec2::new(18.0, 6.0))
            .striped(true)
            .show(ui, |ui| {
                for heading in ["key", "engine", "edit", "context"] {
                    ui.label(
                        egui::RichText::new(heading.to_uppercase())
                            .size(9.5)
                            .color(palette.muted),
                    );
                }
                ui.end_row();
                for entry in entries {
                    ui.label(
                        egui::RichText::new(&entry.label)
                            .size(12.0)
                            .monospace()
                            .color(palette.text),
                    );
                    let (text, tint) = if entry.consumed {
                        ("handled", palette.good)
                    } else {
                        ("passed to the host", palette.warn)
                    };
                    ui.label(egui::RichText::new(text).size(11.0).color(tint));
                    let edit = if entry.erase == 0 && entry.insert.is_empty() {
                        "—".to_owned()
                    } else {
                        format!("-{} {}", entry.erase, entry.insert)
                    };
                    ui.label(
                        egui::RichText::new(edit)
                            .size(12.0)
                            .monospace()
                            .color(palette.accent),
                    );
                    ui.label(
                        egui::RichText::new(&entry.context)
                            .size(12.0)
                            .monospace()
                            .color(palette.muted),
                    );
                    ui.end_row();
                }
            });
        ui.add_space(4.0);
        let last = session.sandbox().log().last();
        if let Some(entry) = last {
            if let Some(rule) = entry.rule {
                ui.label(
                    egui::RichText::new(format!("last match: rule #{rule}"))
                        .size(10.5)
                        .color(palette.muted),
                );
            }
        }
    });
}
