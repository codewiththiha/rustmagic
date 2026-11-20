//! The source: what the keyboard says, and what it is made of.

use egui::{Color32, Rounding, Vec2};

use km_session::Session;

use crate::theme::Palette;
use crate::widgets;

/// Draws the page.
pub fn show(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
    widgets::page_title(
        ui,
        palette,
        "Source",
        "The file the active keyboard came from, and the metadata inside it.",
    );

    let Some(active) = session.active() else {
        widgets::card(palette).show(ui, |ui| {
            ui.label(
                egui::RichText::new("Select a keyboard to read its source.")
                    .size(13.0)
                    .color(palette.muted),
            );
        });
        return;
    };

    let meta = active.keyboard.meta.clone();
    let variables = active.keyboard.variables.clone();
    let source = active.source.clone();
    let rule_count = active.keyboard.rule_count();

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new(&meta.name)
                .size(15.0)
                .strong()
                .color(palette.text),
        );
        if !meta.description.is_empty() {
            ui.label(
                egui::RichText::new(&meta.description)
                    .size(12.0)
                    .color(palette.muted),
            );
        }
        ui.add_space(10.0);
        let facts = [
            ("rules", rule_count.to_string()),
            ("variables", variables.len().to_string()),
            (
                "hotkey",
                if meta.hotkey.is_empty() {
                    "none".to_owned()
                } else {
                    meta.hotkey.clone()
                },
            ),
            (
                "font",
                if meta.font_family.is_empty() {
                    "system".to_owned()
                } else {
                    meta.font_family.clone()
                },
            ),
        ];
        ui.horizontal_wrapped(|ui| {
            for (label, value) in facts {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(0.0, 0.0), egui::Sense::hover());
                let _ = rect;
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(label.to_uppercase())
                            .size(9.0)
                            .color(palette.muted),
                    );
                    ui.label(
                        egui::RichText::new(value)
                            .size(13.0)
                            .monospace()
                            .color(palette.text),
                    );
                });
                ui.add_space(22.0);
            }
        });
    });

    ui.add_space(12.0);

    if variables.is_empty() {
        ui.label(
            egui::RichText::new("This keyboard declares no variables.")
                .size(11.5)
                .color(palette.muted),
        );
    } else {
        widgets::card(palette).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("Variables")
                    .size(13.5)
                    .strong()
                    .color(palette.text),
            );
            ui.add_space(6.0);
            for (index, value) in variables.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("${index}"))
                            .size(11.0)
                            .monospace()
                            .color(palette.accent),
                    );
                    let shown: String = value.chars().take(60).collect();
                    let more = if value.chars().count() > 60 {
                        "…"
                    } else {
                        ""
                    };
                    ui.label(
                        egui::RichText::new(format!("{shown}{more}"))
                            .size(12.0)
                            .monospace()
                            .color(palette.muted),
                    );
                });
            }
        });
        ui.add_space(12.0);
    }

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Source text")
                    .size(13.5)
                    .strong()
                    .color(palette.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(source) = &source {
                    widgets::pill(
                        ui,
                        palette,
                        &format!("{} lines", source.lines().count()),
                        palette.accent,
                    );
                }
            });
        });
        ui.add_space(8.0);
        match &source {
            Some(text) => {
                egui::Frame::none()
                    .fill(palette.paper)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                            .max_height(340.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (index, line) in text.lines().enumerate() {
                                    ui.horizontal_top(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:>3}", index + 1))
                                                .size(11.0)
                                                .monospace()
                                                .color(palette.line.gamma_multiply(1.6)),
                                        );
                                        ui.label(
                                            egui::RichText::new(line)
                                                .size(12.0)
                                                .monospace()
                                                .color(comment_colour(palette, line)),
                                        );
                                    });
                                }
                            });
                    });
            }
            None => {
                ui.label(
                    egui::RichText::new(
                        "Only the compiled keyboard was kept, so there is no source to show.",
                    )
                    .size(12.0)
                    .color(palette.muted),
                );
            }
        }
    });
}

/// Comments and options read differently from rules.
fn comment_colour(palette: &Palette, line: &str) -> Color32 {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
        palette.muted
    } else if trimmed.starts_with('@') {
        palette.accent
    } else {
        palette.text
    }
}
