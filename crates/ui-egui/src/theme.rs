//! The colours, spacing and type scale the window is built from.
//!
//! Every surface in the app comes from here, so a change of mood is a change of
//! one file rather than a hunt through five pages.

use egui::{Color32, Context, FontFamily, FontId, Margin, Rounding, Stroke, Vec2, Visuals};

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
    /// The accent under the pointer, a step brighter than the accent itself.
    pub accent_hover: Color32,
    /// Text and marks that sit *on* the accent: dark on gold, light on a dark
    /// accent, whichever the mood needs to stay readable.
    pub on_accent: Color32,
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

/// The dark mood, which the app opens in: obsidian and gold.
///
/// Near-black with a warm cast rather than a blue one, and the one colour the
/// eye lands on is gold. Everything else is grey with enough brown in it to sit
/// beside the accent instead of fighting it.
pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(10, 10, 11),
    panel: Color32::from_rgb(18, 18, 20),
    card: Color32::from_rgb(23, 23, 26),
    card_hover: Color32::from_rgb(31, 31, 35),
    line: Color32::from_rgb(44, 44, 50),
    text: Color32::from_rgb(242, 239, 232),
    muted: Color32::from_rgb(156, 150, 138),
    accent: Color32::from_rgb(224, 179, 65),
    accent_hover: Color32::from_rgb(240, 197, 90),
    on_accent: Color32::from_rgb(22, 18, 10),
    accent_soft: Color32::from_rgb(43, 36, 22),
    good: Color32::from_rgb(127, 192, 138),
    warn: Color32::from_rgb(232, 163, 60),
    bad: Color32::from_rgb(217, 104, 95),
    paper: Color32::from_rgb(20, 20, 23),
};

/// The light mood: the same stone in daylight.
///
/// The accent is a dark gold here rather than the bright one, because gold on
/// white is a highlighter and dark gold on white is ink.
pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(246, 243, 236),
    panel: Color32::from_rgb(255, 255, 255),
    card: Color32::from_rgb(255, 255, 255),
    card_hover: Color32::from_rgb(241, 237, 228),
    line: Color32::from_rgb(224, 217, 203),
    text: Color32::from_rgb(26, 24, 21),
    muted: Color32::from_rgb(109, 101, 88),
    accent: Color32::from_rgb(166, 122, 18),
    accent_hover: Color32::from_rgb(138, 100, 16),
    on_accent: Color32::from_rgb(255, 253, 246),
    accent_soft: Color32::from_rgb(245, 237, 216),
    good: Color32::from_rgb(47, 125, 79),
    warn: Color32::from_rgb(138, 100, 16),
    bad: Color32::from_rgb(176, 67, 58),
    paper: Color32::from_rgb(251, 249, 244),
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

/// The face that can draw Myanmar, embedded so a Myanmar keyboard can at least
/// show its own name. egui does not shape complex scripts (no reordering, no
/// ligatures), so what appears is the right characters in the right order but
/// not the printed form; a proper run needs a shaping engine. Tofu boxes are
/// worse than an approximation.
const MYANMAR: &[u8] = include_bytes!("../../../assets/fonts/NotoSansMyanmar-Regular.ttf");

/// The interface face. egui's own default is a light-weight Ubuntu that reads as
/// a 2011 desktop; Inter is drawn for screens and holds its shape at the sizes
/// this window actually uses.
const INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Its semibold, for the family the titles and buttons ask for by name.
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

/// The name of the heavier family, so a caller does not spell a string by hand.
pub const BOLD: &str = "bold";

/// Semibold at `size`, for anything that has to read as a heading rather than as
/// body text: egui's `strong()` only brightens the same weight, which is not what
/// a title needs.
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Registers the bundled faces, at the front of the queue where egui looks.
///
/// Called once per context: `set_fonts` rebuilds the atlas, so doing it every
/// frame would be a rebuild every frame.
pub fn install_fonts(context: &Context) {
    context.set_fonts(definitions());
}

/// The font definitions the window draws with: the bundled faces first, egui's
/// own behind them as fallbacks, Myanmar at the end of both text families.
pub fn definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("myanmar".to_owned(), egui::FontData::from_static(MYANMAR));
    fonts
        .font_data
        .insert("inter".to_owned(), egui::FontData::from_static(INTER));
    fonts.font_data.insert(
        "inter-semibold".to_owned(),
        egui::FontData::from_static(INTER_SEMIBOLD),
    );

    // The bundled faces go first and the ones egui ships stay behind them, so a
    // symbol Inter has no glyph for — an arrow, an emoji, a tick — still draws.
    let defaults = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut proportional = Vec::with_capacity(defaults.len() + 2);
    proportional.push("inter".to_owned());
    proportional.extend(defaults.iter().cloned());
    proportional.push("myanmar".to_owned());
    fonts
        .families
        .insert(FontFamily::Proportional, proportional);

    let mut heavy = Vec::with_capacity(defaults.len() + 2);
    heavy.push("inter-semibold".to_owned());
    heavy.extend(defaults.iter().cloned());
    heavy.push("myanmar".to_owned());
    fonts.families.insert(FontFamily::Name(BOLD.into()), heavy);

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("myanmar".to_owned());
    }
    fonts
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
    visuals.faint_bg_color = palette.card;
    visuals.override_text_color = Some(palette.text);
    visuals.window_stroke = Stroke::new(1.0_f32, palette.line);
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, palette.line);
    visuals.widgets.inactive.bg_fill = palette.card;
    visuals.widgets.inactive.weak_bg_fill = palette.card;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.line);
    visuals.widgets.hovered.bg_fill = palette.card_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.active.bg_fill = palette.accent_soft;
    visuals.widgets.active.weak_bg_fill = palette.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.muted);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.window_rounding = Rounding::same(14.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 120 } else { 28 }),
    };
    // The panel behind a text field, so a path field looks like a field instead
    // of a hole in the card.
    visuals.extreme_bg_color = if dark {
        Color32::from_rgb(14, 14, 16)
    } else {
        Color32::from_rgb(247, 244, 238)
    };
    context.style_mut(|style| {
        style.visuals = visuals;
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);
        style.spacing.menu_margin = Margin::same(8.0);
        style.spacing.interact_size.y = 26.0;
        // Text fields get the same rounded corners as everything else.
        style.visuals.text_cursor.stroke = Stroke::new(1.6_f32, palette.accent);
        // A scroll bar that is part of the surface: thin, floating, and the
        // colour of a hairline instead of a bright system grey.
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 7.0,
            floating: true,
            handle_min_length: 40.0,
            ..Default::default()
        };
        style.text_styles = [
            (egui::TextStyle::Heading, bold(25.0)),
            (egui::TextStyle::Body, FontId::proportional(14.5)),
            (egui::TextStyle::Button, bold(13.5)),
            (egui::TextStyle::Small, FontId::proportional(12.0)),
            (egui::TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The type scale names a family, so the definitions have to bind it: a
    /// family that is named but absent is a panic the moment a heading draws.
    #[test]
    fn every_named_family_is_bound() {
        let fonts = definitions();
        for family in [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name(BOLD.into()),
        ] {
            let bound = fonts.families.get(&family).map(Vec::as_slice);
            assert!(
                bound.is_some_and(|names| !names.is_empty()),
                "{family:?} has no fonts bound to it"
            );
        }
    }

    /// The Myanmar face has to remain reachable, otherwise a Burmese keyboard
    /// name draws as boxes.
    #[test]
    fn myanmar_is_a_fallback_for_text() {
        let fonts = definitions();
        let proportional = fonts
            .families
            .get(&FontFamily::Proportional)
            .expect("proportional");
        assert!(proportional.iter().any(|name| name == "myanmar"));
    }
}
