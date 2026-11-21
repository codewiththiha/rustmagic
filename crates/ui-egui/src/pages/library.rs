//! The library: what is installed, and what to install next.

use egui::{Rounding, Stroke, Vec2};

use km_session::{Command, Route, Session};
use km_store::Entry;

use crate::icons::Icon;
use crate::theme::Palette;
use crate::widgets;

/// How tall a card is, margins included.
///
/// Cards in a row are the same height whatever is inside them: a grid whose
/// buttons sit at four different heights reads as broken, not as flexible.
const CARD_HEIGHT: f32 = 214.0;

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
                    Vec2::new(card_width - 12.0, CARD_HEIGHT),
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
        ui.set_width(ui.available_width());
        // Side by side while there is room for a path and a button; stacked
        // under that, because a field squeezed to "/pat…" is not a field.
        let wide = ui.available_width() >= 560.0;
        // The wide layout is right-to-left, so it draws the button and then the
        // field; the stacked one reads left to right and does the opposite.
        let mut path_row = |ui: &mut egui::Ui, width: f32, button_first: bool| {
            let mut install = None;
            if button_first {
                install = Some(widgets::primary_button(ui, palette, "Install"));
            }
            let field = egui::Frame::none()
                .fill(ui.visuals().extreme_bg_color)
                .stroke(Stroke::new(1.0_f32, palette.line))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(import_path)
                            .hint_text("/path/to/layout.kms")
                            .desired_width(width)
                            .frame(false),
                    )
                })
                .inner;
            if !button_first {
                install = Some(widgets::primary_button(ui, palette, "Install"));
            }
            let install = install.expect("one of the two branches drew the button");
            let submitted =
                field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if (install.clicked() || submitted) && !import_path.trim().is_empty() {
                session.execute(Command::ImportPath {
                    path: import_path.trim().to_owned(),
                });
                import_path.clear();
            }
        };
        if wide {
            ui.horizontal(|ui| {
                titles(ui, palette);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    path_row(ui, 300.0_f32.min(ui.available_width() * 0.7), true);
                });
            });
        } else {
            titles(ui, palette);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                path_row(ui, (ui.available_width() - 96.0).max(80.0), false);
            });
        }
    });
}

/// The two lines that name the import bar, wherever it is drawn.
fn titles(ui: &mut egui::Ui, palette: &Palette) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("Install a keyboard")
                .size(14.0)
                .strong()
                .color(palette.text),
        );
        ui.label(
            egui::RichText::new("A .kms source is compiled on the way in, a .km2 is verified.")
                .size(11.5)
                .color(palette.muted),
        );
    });
}

fn search_field(ui: &mut egui::Ui, palette: &Palette, search: &mut String) {
    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0_f32, palette.line))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(12.0, 7.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                ));
                child.add(Icon::Search.image(15.0, palette.muted));
                ui.add(
                    egui::TextEdit::singleline(search)
                        .hint_text("Search by name or description")
                        .desired_width(f32::INFINITY)
                        .frame(false),
                );
                if !search.is_empty() && widgets::quiet_button(ui, palette, "Clear").clicked() {
                    search.clear();
                }
            });
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
    // The active keyboard keeps the same surface as the others and is marked by
    // its border and its chip: filling the whole card with accent made the one
    // card you already own the loudest thing on the page.
    egui::Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(
            if is_active { 1.5_f32 } else { 1.0_f32 },
            if is_active {
                palette.accent
            } else {
                palette.line
            },
        ))
        .rounding(Rounding::same(16.0))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(CARD_HEIGHT - 36.0);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
                // The five bundled keyboards have a mark of their own, drawn from
                // what each of them is. Anything a user installed gets the plain
                // keyboard glyph on the same plate, so the row still lines up.
                let mark = Icon::for_keyboard(&entry.id).unwrap_or(Icon::Keyboard);
                ui.painter()
                    .rect_filled(rect, Rounding::same(10.0), palette.accent_soft);
                let mut child =
                    ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(6.0)).layout(
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    ));
                child.add(mark.image(22.0, palette.accent));
                ui.add_space(2.0);
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(entry.display_name())
                                .font(crate::theme::bold(14.5))
                                .color(palette.text),
                        )
                        .truncate(),
                    );
                    ui.label(
                        egui::RichText::new(&entry.id)
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
                if is_active {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        widgets::pill(ui, palette, "active", palette.accent);
                    });
                }
            });
            ui.add_space(12.0);
            // Two lines are reserved whether the file has a description or not,
            // so the stats and buttons below line up across the row.
            let description = if entry.meta.description.is_empty() {
                "No description in the file."
            } else {
                entry.meta.description.as_str()
            };
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 36.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(description)
                                .size(12.0)
                                .italics()
                                .color(palette.muted),
                        )
                        .wrap(),
                    );
                },
            );
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                widgets::pill(
                    ui,
                    palette,
                    &format!("{} rules", entry.rules),
                    palette.accent,
                );
                widgets::pill(
                    ui,
                    palette,
                    &format!("{} KB", (entry.size as f64 / 1024.0).ceil() as u64),
                    palette.muted,
                );
                if entry.source.is_some() {
                    widgets::pill(ui, palette, "source kept", palette.good);
                }
            });
            // The actions sit on the floor of the card, at the same height in
            // every card of the row.
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
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
        });
}

fn empty_state(ui: &mut egui::Ui, palette: &Palette, library_empty: bool) {
    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, Rounding::same(14.0), palette.accent_soft);
            let mut child =
                ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(13.0)).layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                ));
            child.add(Icon::Library.image(24.0, palette.accent));
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
