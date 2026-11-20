//! The layout: what every key of the physical keyboard writes.

use egui::Vec2;

use km_session::Session;

use crate::theme::Palette;
use crate::widgets;

/// Draws the page.
pub fn show(ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
    widgets::page_title(
        ui,
        palette,
        "Layout",
        "Each cap is what the active keyboard writes for that key, alone and with Shift.",
    );

    let Some(active) = session.active() else {
        widgets::card(palette).show(ui, |ui| {
            ui.label(
                egui::RichText::new("Select a keyboard to see its layout.")
                    .size(13.0)
                    .color(palette.muted),
            );
        });
        return;
    };

    let name = active.keyboard.meta.name.clone();
    let rows = session.layout().to_vec();
    let options = active.keyboard.options;
    let warnings = active.keyboard.warnings.clone();

    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&name)
                    .size(13.5)
                    .strong()
                    .color(palette.text),
            );
            widgets::pill(ui, palette, "Shift in accent", palette.accent);
        });
        ui.add_space(12.0);
        let cap_width = 74.0 + ui.spacing().item_spacing.x;
        let offset = ((ui.available_width() - cap_width * 12.0) / 2.0).max(0.0);
        for (index, row) in rows.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(offset + index as f32 * cap_width * 0.25);
                for cap in &row.caps {
                    widgets::keycap(ui, palette, cap);
                }
            });
            ui.add_space(6.0);
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new("Typed on a US keyboard. A key with no rule writes itself.")
                .size(11.0)
                .color(palette.muted),
        );
    });

    ui.add_space(12.0);
    widgets::card(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("How this keyboard behaves")
                .size(13.5)
                .strong()
                .color(palette.text),
        );
        ui.add_space(8.0);
        let flags = [
            ("Caps lock tracked", options.track_caps),
            ("Smart backspace", options.auto_bksp),
            ("Unused keys swallowed", options.eat),
            ("Positional layout", options.pos_based),
            ("Right Alt as AltGr", options.right_alt),
        ];
        ui.horizontal_wrapped(|ui| {
            for (label, on) in flags {
                let tint = if on { palette.good } else { palette.muted };
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.5, tint);
                ui.label(egui::RichText::new(label).size(11.5).color(tint));
                ui.add_space(10.0);
            }
        });
        if !warnings.is_empty() {
            ui.add_space(10.0);
            for warning in &warnings {
                ui.label(
                    egui::RichText::new(format!("· {warning}"))
                        .size(11.0)
                        .color(palette.warn),
                );
            }
        }
    });
}
