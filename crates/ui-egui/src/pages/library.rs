//! The library: what is installed, and what to install next.

use egui::{Color32, Rounding, Stroke, Vec2};

use km_session::{Command, Route, Session};
use km_store::Entry;

use crate::theme::Palette;
use crate::widgets::{self, Icon};

/// Draws the page.
pub fn show(
    ui: &mut egui::Ui,
    session: &mut Session,
    palette: &Palette,
    search: &mut String,
    import_path: &mut String,
    confirm: &mut Option<String>,
) {
    widgets::page_title(
        ui,
        palette,
        "Library",
        "Every keyboard RustMagic can type with. One of them is always active.",
    );

    install_bar(ui, session, palette, import_path);

    ui.add_space(16.0);
    search_field(ui, palette, search);
    ui.add_space(12.0);

    let query = search.trim().to_lowercase();
    let entries: Vec<Entry> = session
        .entries()
        .iter()
        .filter(|entry| {
            query.is_empty()
                || entry.display_name().to_lowercase().contains(&query)
                || entry.id.contains(&query)
                || entry.meta.description.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();

    if entries.is_empty() {
        empty_state(ui, palette, session.entries().is_empty());
        return;
    }

    let active_id = session.active().map(|active| active.id.clone());
    let width = ui.available_width();
    let columns = ((width / 330.0).floor() as usize).clamp(1, 3);
    let mut selected = None;
    let mut removed = None;

    for chunk in entries.chunks(columns) {
        ui.horizontal_top(|ui| {
            for entry in chunk {
                let is_active = active_id.as_deref() == Some(entry.id.as_str());
                let is_confirming = confirm.as_deref() == Some(entry.id.as_str());
                let card_width = (width - 12.0 * (columns as f32 - 1.0)) / columns as f32;
                ui.allocate_ui_with_layout(
                    Vec2::new(card_width - 12.0, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        card(
                            ui,
                            palette,
                            entry,
                            is_active,
                            is_confirming,
                            &mut selected,
                            &mut removed,
                            confirm,
                        );
                    },
                );
                ui.add_space(4.0);
            }
        });
        ui.add_space(4.0);
    }

    if let Some(id) = selected {
        session.execute(Command::SelectKeyboard { id });
        session.execute(Command::ShowRoute {
            route: Route::TryIt,
        });
    }
    if let Some(id) = removed {
        session.execute(Command::RemoveKeyboard { id });
        *confirm = None;
    }
}

fn install_bar(
    ui: &mut egui::Ui,
    session: &mut Session,
    palette: &Palette,
    import_path: &mut String,
) {
    widgets::card(palette).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Install a keyboard")
                        .size(14.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(
                        "A .kms source is compiled on the way in, a .km2 is verified.",
                    )
                    .size(11.5)
                    .color(palette.muted),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let install = widgets::primary_button(ui, palette, "Install");
                let field = ui.add(
                    egui::TextEdit::singleline(import_path)
                        .hint_text("/path/to/layout.kms")
                        .desired_width(300.0),
                );
                let submitted =
                    field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if (install.clicked() || submitted) && !import_path.trim().is_empty() {
                    session.execute(Command::ImportPath {
                        path: import_path.trim().to_owned(),
                    });
                    import_path.clear();
                }
            });
        });
    });
}

fn search_field(ui: &mut egui::Ui, palette: &Palette, search: &mut String) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
        widgets::icon(ui, rect, Some(Icon::Search), palette.muted);
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text("Search by name or description")
                .desired_width(320.0)
                .frame(false),
        );
        if !search.is_empty() && widgets::quiet_button(ui, palette, "Clear").clicked() {
            search.clear();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn card(
    ui: &mut egui::Ui,
    palette: &Palette,
    entry: &Entry,
    is_active: bool,
    is_confirming: bool,
    selected: &mut Option<String>,
    removed: &mut Option<String>,
    confirm: &mut Option<String>,
) {
    let stroke = if is_active {
        Stroke::new(1.0_f32, palette.accent)
    } else {
        Stroke::new(1.0_f32, palette.line)
    };
    egui::Frame::none()
        .fill(if is_active {
            palette.accent_soft
        } else {
            palette.card
        })
        .stroke(stroke)
        .rounding(Rounding::same(14.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(9.0),
                    if is_active {
                        palette.accent
                    } else {
                        palette.line
                    },
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    entry
                        .display_name()
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_uppercase()
                        .to_string(),
                    egui::FontId::proportional(15.0),
                    if is_active {
                        Color32::WHITE
                    } else {
                        palette.text
                    },
                );
                ui.add_space(2.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(entry.display_name())
                            .size(14.5)
                            .strong()
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new(&entry.id)
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
                if is_active {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.label(
                            egui::RichText::new("ACTIVE")
                                .size(9.5)
                                .strong()
                                .color(palette.accent),
                        );
                    });
                }
            });
            ui.add_space(8.0);
            let description = if entry.meta.description.is_empty() {
                "No description in the file."
            } else {
                entry.meta.description.as_str()
            };
            ui.label(
                egui::RichText::new(description)
                    .size(12.0)
                    .color(palette.muted),
            );
            ui.add_space(10.0);
            widgets::meter(ui, palette, (entry.rules as f32 / 400.0).clamp(0.03, 1.0));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                widgets::pill(
                    ui,
                    palette,
                    &format!("{} rules", entry.rules),
                    palette.accent,
                );
                widgets::pill(ui, palette, &format!("{} B", entry.size), palette.muted);
                if entry.source.is_some() {
                    widgets::pill(ui, palette, "source", palette.good);
                }
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if !is_active && widgets::primary_button(ui, palette, "Use").clicked() {
                    *selected = Some(entry.id.clone());
                }
                if is_active && widgets::quiet_button(ui, palette, "Open sandbox").clicked() {
                    *selected = Some(entry.id.clone());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_confirming {
                        if widgets::quiet_button(ui, palette, "Really remove").clicked() {
                            *removed = Some(entry.id.clone());
                        }
                        if widgets::quiet_button(ui, palette, "Keep").clicked() {
                            *confirm = None;
                        }
                    } else if widgets::quiet_button(ui, palette, "Remove").clicked() {
                        *confirm = Some(entry.id.clone());
                    }
                });
            });
        });
    ui.add_space(12.0);
}

fn empty_state(ui: &mut egui::Ui, palette: &Palette, library_empty: bool) {
    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, Rounding::same(14.0), palette.accent_soft);
            widgets::icon(ui, rect.shrink(13.0), Some(Icon::Library), palette.accent);
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(if library_empty {
                    "Nothing installed yet"
                } else {
                    "No keyboard matches that search"
                })
                .size(15.0)
                .color(palette.text),
            );
            ui.label(
                egui::RichText::new(if library_empty {
                    "Point the install field at a .kms or .km2 file to start."
                } else {
                    "Try a shorter word, or clear the field."
                })
                .size(12.0)
                .color(palette.muted),
            );
            ui.add_space(18.0);
        });
    });
}
