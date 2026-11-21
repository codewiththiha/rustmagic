//! Small pieces the pages are assembled from.
//!
//! Each one takes the palette rather than reaching for global style, so a page
//! can be drawn against the dark mood and the light mood in the same test.

use egui::{Align, Align2, Color32, Layout, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

use km_session::KeyCap;

use crate::icons::Icon;
use crate::theme::Palette;

/// A page heading with a line of context under it.
pub fn page_title(ui: &mut Ui, palette: &Palette, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(24.0)
            .strong()
            .color(palette.text),
    );
    ui.label(
        egui::RichText::new(subtitle)
            .size(13.0)
            .color(palette.muted),
    );
    ui.add_space(14.0);
}

/// A small rounded label, used for counts and states.
pub fn pill(ui: &mut Ui, palette: &Palette, text: &str, tint: Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), egui::FontId::proportional(11.5), tint);
    let size = Vec2::new(galley.size().x + 16.0, galley.size().y + 8.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(size.y / 2.0), palette.accent_soft);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, palette.text);
}

/// A button that reads as the primary action of its card.
pub fn primary_button(ui: &mut Ui, palette: &Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(13.0).color(Color32::WHITE))
            .fill(palette.accent)
            .rounding(Rounding::same(9.0)),
    )
}

/// A quiet button, for everything that is not the point of the page.
pub fn quiet_button(ui: &mut Ui, palette: &Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(13.0).color(palette.muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, palette.line))
            .rounding(Rounding::same(9.0)),
    )
}

/// A row of mutually exclusive choices; returns the one clicked.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut Ui,
    palette: &Palette,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        for (value, label) in options {
            let selected = *value == current;
            let button =
                egui::Button::new(egui::RichText::new(*label).size(12.5).color(if selected {
                    palette.text
                } else {
                    palette.muted
                }))
                .fill(if selected {
                    palette.accent_soft
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if selected {
                        palette.accent
                    } else {
                        palette.line
                    },
                ))
                .rounding(Rounding::same(8.0));
            if ui.add(button).clicked() && !selected {
                chosen = Some(*value);
            }
        }
    });
    chosen
}

/// A switch that reads as a physical one.
pub fn toggle(ui: &mut Ui, palette: &Palette, on: bool) -> Response {
    let size = Vec2::new(38.0, 21.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let knob_x = if on {
        rect.right() - 10.5
    } else {
        rect.left() + 10.5
    };
    ui.painter().rect_filled(
        rect,
        Rounding::same(size.y / 2.0),
        if on { palette.accent } else { palette.line },
    );
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        7.5,
        if on { Color32::WHITE } else { palette.muted },
    );
    response
}

/// One key cap of the layout view, drawn as a real cap.
///
/// The size comes from the caller: a keyboard is twelve keys wide, which does
/// not fit in a narrow window, so the board shrinks its caps instead of
/// scrolling sideways with half a key cut off.
pub fn keycap(ui: &mut Ui, palette: &Palette, cap: &KeyCap, size: Vec2) {
    let scale = (size.x / 74.0).clamp(0.55, 1.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let hovered = response.hovered();
    ui.painter().rect_filled(
        rect,
        Rounding::same(11.0 * scale),
        if hovered {
            palette.card_hover
        } else {
            palette.card
        },
    );
    ui.painter().rect_stroke(
        rect,
        Rounding::same(11.0 * scale),
        Stroke::new(
            1.0_f32,
            if hovered {
                palette.accent
            } else {
                palette.line
            },
        ),
    );
    let glyph = if cap.plain.is_empty() {
        "·"
    } else {
        cap.plain.as_str()
    };
    ui.painter().text(
        rect.left_top() + Vec2::new(11.0 * scale, 8.0 * scale),
        Align2::LEFT_TOP,
        &cap.label,
        egui::FontId::proportional(11.5 * scale),
        palette.muted,
    );
    ui.painter().text(
        rect.center_bottom() - Vec2::new(0.0, 12.0 * scale),
        Align2::CENTER_BOTTOM,
        glyph,
        egui::FontId::proportional(24.0 * scale),
        palette.text,
    );
    if cap.shifted != cap.plain {
        ui.painter().text(
            rect.right_top() + Vec2::new(-11.0 * scale, 8.0 * scale),
            Align2::RIGHT_TOP,
            &cap.shifted,
            egui::FontId::proportional(12.5 * scale),
            palette.accent,
        );
    }
}

/// A card: the base surface every page puts content on.
pub fn card(palette: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(1.0_f32, palette.line))
        .rounding(Rounding::same(14.0))
        .inner_margin(egui::Margin::same(18.0))
}

/// A raised card, for the composition surface.
pub fn paper(palette: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.paper)
        .stroke(Stroke::new(1.0_f32, palette.line))
        .rounding(Rounding::same(16.0))
        .inner_margin(egui::Margin::same(22.0))
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if palette.bg.r() < 128 { 90 } else { 24 }),
        })
}

/// A line of settings: label and explanation on the left, control on the right.
pub fn setting_row(
    ui: &mut Ui,
    palette: &Palette,
    title: &str,
    detail: &str,
    control: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(14.0).color(palette.text));
            ui.label(egui::RichText::new(detail).size(12.0).color(palette.muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
    ui.add_space(6.0);
}

/// A hairline.
pub fn separator(ui: &mut Ui, palette: &Palette) {
    let rect = ui.available_rect_before_wrap();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(rect.width(), 9.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, palette.line),
    );
}

/// A bar that fills `fraction` of `rect`, used for the rule-count meter.
pub fn meter(ui: &mut Ui, palette: &Palette, fraction: f32) {
    let width = ui.available_width().min(180.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 5.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), palette.line);
    let filled = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    ui.painter()
        .rect_filled(filled, Rounding::same(3.0), palette.accent);
}

/// A sidebar row: icon, label, selection state.
///
/// `compact` drops the label, which is what the rail does when the window is
/// too narrow to hold both.
pub fn nav_row(
    ui: &mut Ui,
    palette: &Palette,
    active: bool,
    kind: Icon,
    label: &str,
    compact: bool,
) -> Response {
    let height = 38.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            Rounding::same(10.0),
            if active {
                palette.accent_soft
            } else {
                palette.card_hover
            },
        );
    }
    if active {
        let bar = Rect::from_min_size(
            rect.left_top() + Vec2::new(0.0, 9.0),
            Vec2::new(3.0, height - 18.0),
        );
        ui.painter()
            .rect_filled(bar, Rounding::same(2.0), palette.accent);
    }
    let colour = if active {
        palette.accent
    } else {
        palette.muted
    };
    let icon_size = 20.0;
    let icon_left = if compact {
        rect.center().x - icon_size / 2.0
    } else {
        rect.left() + 14.0
    };
    let icon_rect = Rect::from_min_size(
        egui::pos2(icon_left, rect.center().y - icon_size / 2.0),
        Vec2::splat(icon_size),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(icon_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.add(kind.image(icon_size, colour));
    if !compact {
        ui.painter().text(
            rect.left_center() + Vec2::new(46.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
            if active { palette.text } else { palette.muted },
        );
    }
    response
}
