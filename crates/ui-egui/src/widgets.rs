//! Small pieces the pages are assembled from.
//!
//! Each one takes the palette rather than reaching for global style, so a page
//! can be drawn against the dark mood and the light mood in the same test.

use egui::{Align, Align2, Color32, Layout, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

use km_session::KeyCap;

use crate::icons::Icon;
use crate::theme::{self, Palette};

/// A page heading with a line of context under it.
pub fn page_title(ui: &mut Ui, palette: &Palette, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .font(theme::bold(26.0))
            .color(palette.text),
    );
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(subtitle)
            .size(13.5)
            .color(palette.muted),
    );
    ui.add_space(18.0);
}

/// A small rounded label, used for counts and states.
///
/// The tint carries the meaning and the fill is that tint at low strength, so a
/// chip says what it is about rather than being another grey box.
pub fn pill(ui: &mut Ui, palette: &Palette, text: &str, tint: Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), theme::bold(11.0), tint);
    let size = Vec2::new(galley.size().x + 16.0, galley.size().y + 9.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(
        rect,
        Rounding::same(size.y / 2.0),
        Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 34),
    );
    let _ = palette;
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, tint);
}

/// A button that reads as the primary action of its card.
///
/// Drawn by hand rather than through `egui::Button`: a filled button needs its
/// own hover colour, and the widget's own idea of a hovered fill is the palette's
/// card, which would make the one button worth pressing the one that vanishes.
pub fn primary_button(ui: &mut Ui, palette: &Palette, text: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), theme::bold(13.0), palette.on_accent);
    let size = Vec2::new(galley.size().x + 28.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        palette.accent
    } else if response.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    ui.painter().rect_filled(rect, Rounding::same(10.0), fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A quiet button, for everything that is not the point of the page.
pub fn quiet_button(ui: &mut Ui, palette: &Palette, text: &str) -> Response {
    let colour = palette.muted;
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), theme::bold(12.5), colour);
    let size = Vec2::new(galley.size().x + 24.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect, Rounding::same(10.0), palette.card_hover);
    }
    ui.painter().rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            if hovered {
                palette.accent
            } else {
                palette.line
            },
        ),
    );
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        theme::bold(12.5),
        if hovered { palette.text } else { palette.muted },
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, palette.muted);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
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
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
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
    let rounding = Rounding::same(9.0 * scale);
    // A key is a block with a top face, not a rectangle drawn on the card: the
    // skirt under it is what makes it stand up.
    let skirt = rect;
    let face = Rect::from_min_max(
        rect.min + Vec2::new(0.0, 0.0),
        rect.max - Vec2::new(0.0, 3.5 * scale),
    );
    ui.painter().rect_filled(
        skirt,
        rounding,
        if hovered {
            palette.accent
        } else {
            palette.line
        },
    );
    ui.painter().rect_filled(
        face,
        rounding,
        if hovered {
            palette.accent_soft
        } else {
            palette.card_hover
        },
    );
    // The top edge catches the light, and the bottom of the face keeps a hair of
    // shadow so the skirt reads as thickness rather than as a border.
    ui.painter().hline(
        egui::Rangef::new(face.left() + 7.0 * scale, face.right() - 7.0 * scale),
        face.top() + 1.0,
        Stroke::new(
            1.0_f32,
            if hovered {
                palette.accent
            } else {
                Color32::from_white_alpha(if palette.bg.r() < 128 { 22 } else { 150 })
            },
        ),
    );
    ui.painter().hline(
        face.x_range(),
        face.bottom() - 0.5,
        Stroke::new(1.0_f32, palette.line),
    );
    let glyph = if cap.plain.is_empty() {
        "·"
    } else {
        cap.plain.as_str()
    };
    ui.painter().text(
        face.left_top() + Vec2::new(9.0 * scale, 6.0 * scale),
        Align2::LEFT_TOP,
        &cap.label,
        theme::bold(10.5 * scale),
        palette.muted,
    );
    ui.painter().text(
        face.center_bottom() - Vec2::new(0.0, 8.0 * scale),
        Align2::CENTER_BOTTOM,
        glyph,
        egui::FontId::proportional(26.0 * scale),
        palette.text,
    );
    if cap.shifted != cap.plain {
        ui.painter().text(
            face.right_top() + Vec2::new(-9.0 * scale, 6.0 * scale),
            Align2::RIGHT_TOP,
            &cap.shifted,
            egui::FontId::proportional(13.0 * scale),
            if hovered {
                palette.text
            } else {
                palette.accent
            },
        );
    }
}

/// A card: the base surface every page puts content on.
pub fn card(palette: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.card)
        .stroke(Stroke::new(1.0_f32, palette.line))
        .rounding(Rounding::same(16.0))
        .inner_margin(egui::Margin::same(20.0))
}

/// A raised card, for the composition surface.
pub fn paper(palette: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.paper)
        .stroke(Stroke::new(1.0_f32, palette.line))
        .rounding(Rounding::same(18.0))
        .inner_margin(egui::Margin::same(24.0))
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 8.0),
            blur: 22.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if palette.bg.r() < 128 { 96 } else { 26 }),
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
            ui.label(
                egui::RichText::new(title)
                    .font(theme::bold(13.5))
                    .color(palette.text),
            );
            ui.label(egui::RichText::new(detail).size(12.0).color(palette.muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
    ui.add_space(8.0);
}

/// A hairline.
pub fn separator(ui: &mut Ui, palette: &Palette) {
    let rect = ui.available_rect_before_wrap();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(rect.width(), 13.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, palette.line),
    );
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
    let height = 40.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            Rounding::same(11.0),
            if active {
                palette.accent_soft
            } else {
                palette.card_hover
            },
        );
    }
    let colour = if active {
        palette.accent
    } else if response.hovered() {
        palette.text
    } else {
        palette.muted
    };
    let icon_size = 20.0;
    let icon_left = if compact {
        rect.center().x - icon_size / 2.0
    } else {
        rect.left() + 16.0
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
            rect.left_center() + Vec2::new(48.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            if active {
                theme::bold(13.5)
            } else {
                egui::FontId::proportional(13.5)
            },
            if active { palette.text } else { colour },
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
