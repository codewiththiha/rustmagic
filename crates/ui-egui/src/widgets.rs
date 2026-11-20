//! Small pieces the pages are assembled from.
//!
//! Each one takes the palette rather than reaching for global style, so a page
//! can be drawn against the dark mood and the light mood in the same test.

use egui::{Align, Align2, Color32, Layout, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

use km_session::KeyCap;

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
pub fn keycap(ui: &mut Ui, palette: &Palette, cap: &KeyCap) {
    let size = Vec2::new(74.0, 64.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let hovered = response.hovered();
    ui.painter().rect_filled(
        rect,
        Rounding::same(11.0),
        if hovered {
            palette.card_hover
        } else {
            palette.card
        },
    );
    ui.painter().rect_stroke(
        rect,
        Rounding::same(11.0),
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
        rect.left_top() + Vec2::new(11.0, 9.0),
        Align2::LEFT_TOP,
        &cap.label,
        egui::FontId::proportional(11.5),
        palette.muted,
    );
    ui.painter().text(
        rect.center_bottom() - Vec2::new(0.0, 12.0),
        Align2::CENTER_BOTTOM,
        glyph,
        egui::FontId::proportional(24.0),
        palette.text,
    );
    if cap.shifted != cap.plain {
        ui.painter().text(
            rect.right_top() + Vec2::new(-11.0, 9.0),
            Align2::RIGHT_TOP,
            &cap.shifted,
            egui::FontId::proportional(12.5),
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

/// A line of an icon, in fractions of its box: `((from_x, from_y), (to_x, to_y))`.
type Segment = ((f32, f32), (f32, f32));

/// The one glyph a nav item needs, drawn rather than imported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Installed keyboards.
    Library,
    /// The typing sandbox.
    Keyboard,
    /// The key-by-key view.
    Grid,
    /// The source text.
    Code,
    /// Settings.
    Sliders,
    /// Search.
    Search,
}

impl Icon {
    /// All icons, in navigation order.
    pub const ALL: [Self; 5] = [
        Self::Library,
        Self::Keyboard,
        Self::Grid,
        Self::Code,
        Self::Sliders,
    ];

    fn lines(self) -> &'static [Segment] {
        match self {
            Self::Library => &[
                ((0.22, 0.25), (0.78, 0.25)),
                ((0.22, 0.5), (0.62, 0.5)),
                ((0.22, 0.75), (0.7, 0.75)),
            ],
            Self::Keyboard => &[
                ((0.18, 0.3), (0.82, 0.3)),
                ((0.18, 0.3), (0.18, 0.7)),
                ((0.82, 0.3), (0.82, 0.7)),
                ((0.18, 0.7), (0.82, 0.7)),
                ((0.35, 0.52), (0.65, 0.52)),
            ],
            Self::Grid => &[
                ((0.2, 0.2), (0.45, 0.2)),
                ((0.55, 0.2), (0.8, 0.2)),
                ((0.2, 0.5), (0.45, 0.5)),
                ((0.55, 0.5), (0.8, 0.5)),
                ((0.2, 0.8), (0.45, 0.8)),
                ((0.55, 0.8), (0.8, 0.8)),
            ],
            Self::Code => &[
                ((0.38, 0.22), (0.2, 0.5)),
                ((0.2, 0.5), (0.38, 0.78)),
                ((0.62, 0.22), (0.8, 0.5)),
                ((0.8, 0.5), (0.62, 0.78)),
            ],
            Self::Sliders => &[
                ((0.2, 0.3), (0.8, 0.3)),
                ((0.2, 0.7), (0.8, 0.7)),
                ((0.4, 0.3), (0.4, 0.3)),
                ((0.65, 0.7), (0.65, 0.7)),
            ],
            Self::Search => &[
                ((0.25, 0.25), (0.62, 0.25)),
                ((0.25, 0.25), (0.25, 0.62)),
                ((0.75, 0.75), (0.6, 0.6)),
            ],
        }
    }
}

/// Draws an icon centred in `rect`.
pub fn icon(ui: &Ui, rect: Rect, kind: Option<Icon>, colour: Color32) {
    let painter = ui.painter();
    match kind {
        Some(kind) => {
            for (from, to) in kind.lines() {
                let start = rect.min + Vec2::new(rect.width() * from.0, rect.height() * from.1);
                let end = rect.min + Vec2::new(rect.width() * to.0, rect.height() * to.1);
                if (from.0 - to.0).abs() < 0.001 && (from.1 - to.1).abs() < 0.001 {
                    painter.circle_filled(start, 2.2, colour);
                } else {
                    painter.line_segment([start, end], Stroke::new(1.6_f32, colour));
                }
            }
        }
        None => {
            painter.circle_filled(rect.center(), 5.0, colour);
        }
    }
}

/// A sidebar row: icon, label, selection state.
pub fn nav_row(ui: &mut Ui, palette: &Palette, active: bool, kind: Icon, label: &str) -> Response {
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
    let icon_rect = Rect::from_min_size(rect.left_top() + Vec2::new(14.0, 9.0), Vec2::splat(20.0));
    icon(
        ui,
        icon_rect,
        Some(kind),
        if active {
            palette.accent
        } else {
            palette.muted
        },
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(46.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        if active { palette.text } else { palette.muted },
    );
    response
}
