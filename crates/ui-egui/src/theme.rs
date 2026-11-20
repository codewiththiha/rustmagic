//! The colours, spacing and type scale the window is built from.
//!
//! Every surface in the app comes from here, so a change of mood is a change of
//! one file rather than a hunt through five pages.

use egui::{Color32, Context, Margin, Rounding, Stroke, Vec2, Visuals};

/// The colours of one mood.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Behind everything.
    pub bg: Color32,
    /// Sidebar and header.
    pub panel: Color32,
    /// Cards and rows.
    pub card: Color32,
    /// A card under the pointer.
    pub card_hover: Color32,
    /// Hairline separators.
    pub line: Color32,
    /// Body text.
    pub text: Color32,
    /// Secondary text.
    pub muted: Color32,
    /// The one colour the eye should land on.
    pub accent: Color32,
    /// Accent at low strength, for fills.
    pub accent_soft: Color32,
    /// Success.
    pub good: Color32,
    /// Something to look at.
    pub warn: Color32,
    /// Something wrong.
    pub bad: Color32,
    /// The paper the try-it page writes on.
    pub paper: Color32,
}

/// The dark mood, which the app opens in.
pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(14, 16, 23),
    panel: Color32::from_rgb(19, 22, 31),
    card: Color32::from_rgb(25, 29, 40),
    card_hover: Color32::from_rgb(32, 37, 51),
    line: Color32::from_rgb(41, 47, 63),
    text: Color32::from_rgb(232, 236, 245),
    muted: Color32::from_rgb(140, 150, 172),
    accent: Color32::from_rgb(124, 140, 255),
    accent_soft: Color32::from_rgb(40, 46, 78),
    good: Color32::from_rgb(104, 211, 145),
    warn: Color32::from_rgb(240, 190, 100),
    bad: Color32::from_rgb(242, 118, 128),
    paper: Color32::from_rgb(30, 34, 46),
};

/// The light mood.
pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(246, 247, 251),
    panel: Color32::from_rgb(255, 255, 255),
    card: Color32::from_rgb(255, 255, 255),
    card_hover: Color32::from_rgb(243, 245, 252),
    line: Color32::from_rgb(222, 226, 238),
    text: Color32::from_rgb(26, 30, 42),
    muted: Color32::from_rgb(104, 113, 133),
    accent: Color32::from_rgb(80, 96, 224),
    accent_soft: Color32::from_rgb(232, 235, 255),
    good: Color32::from_rgb(38, 158, 96),
    warn: Color32::from_rgb(176, 128, 26),
    bad: Color32::from_rgb(204, 66, 82),
    paper: Color32::from_rgb(250, 250, 253),
};

/// The palette for a mood.
pub fn palette(dark: bool) -> Palette {
    if dark { DARK } else { LIGHT }
}

/// Whether the window should paint dark right now.
pub fn is_dark(context: &Context, preference: km_store::Theme) -> bool {
    match preference {
        km_store::Theme::Dark => true,
        km_store::Theme::Light => false,
        km_store::Theme::System => context
            .input(|input| input.raw.system_theme)
            .is_none_or(|theme| matches!(theme, egui::Theme::Dark)),
    }
}

/// Installs the palette and the type scale on a context.
pub fn apply(context: &Context, palette: &Palette, dark: bool) {
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = palette.paper;
    visuals.faint_bg_color = palette.card;
    visuals.override_text_color = Some(palette.text);
    visuals.window_stroke = Stroke::new(1.0_f32, palette.line);
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.inactive.bg_fill = palette.card;
    visuals.widgets.inactive.weak_bg_fill = palette.card;
    visuals.widgets.hovered.bg_fill = palette.card_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.line);
    visuals.widgets.active.bg_fill = palette.accent_soft;
    visuals.widgets.active.weak_bg_fill = palette.accent_soft;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.window_rounding = Rounding::same(14.0);
    context.style_mut(|style| {
        style.visuals = visuals;
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.menu_margin = Margin::same(8.0);
        style.spacing.interact_size.y = 26.0;
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::proportional(24.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(14.5)),
            (egui::TextStyle::Button, egui::FontId::proportional(14.5)),
            (egui::TextStyle::Small, egui::FontId::proportional(12.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(13.0)),
        ]
        .into();
    });
}
