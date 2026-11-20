//! The window: a rail, a header, one page, and whatever is floating above them.

use egui::{Color32, Context, Margin, Rounding, Stroke, Vec2};

use km_session::{Command, Event, Route, Session, ToastKind};
use km_store::Theme;

use crate::pages;
use crate::theme::{self, Palette};
use crate::widgets::{self, Icon};

/// A message in the corner that fades away on its own.
#[derive(Debug, Clone)]
struct Toast {
    kind: ToastKind,
    text: String,
    born: f64,
}

const TOAST_SECONDS: f64 = 4.5;

/// The state the window keeps that the session has no opinion about.
pub struct Ui {
    palette: Palette,
    dark: bool,
    toasts: Vec<Toast>,
    palette_open: bool,
    palette_query: String,
    import_path: String,
    search: String,
    library_confirm: Option<String>,
    /** When set, the next frame focuses the command palette field. */
    focus_palette: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            palette: theme::DARK,
            dark: true,
            toasts: Vec::new(),
            palette_open: false,
            palette_query: String::new(),
            import_path: String::new(),
            search: String::new(),
            library_confirm: None,
            focus_palette: false,
        }
    }
}

impl Ui {
    /// A window in its opening state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws one frame and returns the commands the user asked for.
    pub fn show(&mut self, context: &Context, session: &mut Session) {
        self.absorb(session);
        let dark = theme::is_dark(context, session.settings().theme);
        if dark != self.dark {
            self.dark = dark;
            self.palette = theme::palette(dark);
        }
        theme::apply(context, &self.palette, self.dark);
        let palette = self.palette;

        self.shortcuts(context, session);
        self.rail(context, session, &palette);
        self.header(context, session, &palette);
        self.status(context, session, &palette);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(palette.bg).inner_margin(Margin {
                left: 26.0,
                right: 26.0,
                top: 18.0,
                bottom: 18.0,
            }))
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        self.page(ui, session, &palette);
                    });
            });

        self.palette_overlay(context, session);
        self.toasts(context, &palette);
    }

    fn page(&mut self, ui: &mut egui::Ui, session: &mut Session, palette: &Palette) {
        match session.route() {
            Route::Library => pages::library::show(
                ui,
                session,
                palette,
                &mut self.search,
                &mut self.import_path,
                &mut self.library_confirm,
            ),
            Route::TryIt => pages::try_it::show(ui, session, palette),
            Route::Layout => pages::layout::show(ui, session, palette),
            Route::Source => pages::source::show(ui, session, palette),
            Route::Settings => pages::settings::show(ui, session, palette),
        }
    }

    /// Turns session events into toasts and repaints for their fade.
    fn absorb(&mut self, session: &mut Session) {
        let now = session_time();
        for event in session.drain() {
            if let Event::Toast { kind, text } = event {
                self.toasts.push(Toast {
                    kind,
                    text,
                    born: now,
                });
                if self.toasts.len() > 4 {
                    self.toasts.remove(0);
                }
            }
        }
        self.toasts.retain(|toast| now - toast.born < TOAST_SECONDS);
    }

    fn shortcuts(&mut self, context: &Context, session: &mut Session) {
        let (palette_keys, escape, save) = context.input(|input| {
            (
                (input.modifiers.command || input.modifiers.ctrl)
                    && input.key_pressed(egui::Key::K),
                input.key_pressed(egui::Key::Escape),
                (input.modifiers.command || input.modifiers.ctrl)
                    && input.key_pressed(egui::Key::S),
            )
        });
        if palette_keys {
            self.palette_open = !self.palette_open;
            self.palette_query.clear();
            self.focus_palette = self.palette_open;
        }
        if escape && self.palette_open {
            self.palette_open = false;
        }
        if save {
            session.execute(Command::Save);
        }
        for (index, route) in Route::ALL.iter().enumerate() {
            let key = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
            ][index];
            let pressed = context.input(|input| {
                (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(key)
            });
            if pressed {
                session.execute(Command::ShowRoute { route: *route });
            }
        }
    }

    fn rail(&mut self, context: &Context, session: &mut Session, palette: &Palette) {
        egui::SidePanel::left("rail")
            .exact_width(216.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(palette.panel)
                    .inner_margin(Margin {
                        left: 12.0,
                        right: 12.0,
                        top: 16.0,
                        bottom: 14.0,
                    }),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, Rounding::same(9.0), palette.accent);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "R",
                        egui::FontId::proportional(17.0),
                        Color32::WHITE,
                    );
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("RustMagic")
                                .size(15.5)
                                .strong()
                                .color(palette.text),
                        );
                        ui.label(
                            egui::RichText::new("keyboard engine")
                                .size(10.5)
                                .color(palette.muted),
                        );
                    });
                });
                ui.add_space(18.0);
                for (index, route) in Route::ALL.iter().enumerate() {
                    let icon = Icon::ALL[index];
                    let active = session.route() == *route;
                    let response = widgets::nav_row(ui, palette, active, icon, route.label());
                    if response.clicked() && !active {
                        session.execute(Command::ShowRoute { route: *route });
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    let label = match session.settings().theme {
                        Theme::System => "System theme",
                        Theme::Light => "Light theme",
                        Theme::Dark => "Dark theme",
                    };
                    if widgets::quiet_button(ui, palette, label).clicked() {
                        let next = match session.settings().theme {
                            Theme::System => Theme::Light,
                            Theme::Light => Theme::Dark,
                            Theme::Dark => Theme::System,
                        };
                        session.execute(Command::SetTheme { theme: next });
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
            });
    }

    fn header(&mut self, context: &Context, session: &mut Session, palette: &Palette) {
        egui::TopBottomPanel::top("header")
            .exact_height(64.0)
            .frame(egui::Frame::none().fill(palette.bg).inner_margin(Margin {
                left: 26.0,
                right: 26.0,
                top: 10.0,
                bottom: 10.0,
            }))
            .show(context, |ui| {
                ui.horizontal_centered(|ui| {
                    match session.active() {
                        Some(active) => {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&active.keyboard.meta.name)
                                        .size(16.0)
                                        .strong()
                                        .color(palette.text),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} rules · {} variables",
                                        active.keyboard.rule_count(),
                                        active.keyboard.variables.len()
                                    ))
                                    .size(11.5)
                                    .color(palette.muted),
                                );
                            });
                            ui.add_space(8.0);
                            widgets::pill(ui, palette, "active", palette.good);
                        }
                        None => {
                            ui.label(
                                egui::RichText::new("No keyboard selected")
                                    .size(15.0)
                                    .color(palette.muted),
                            );
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::quiet_button(ui, palette, "⌘K  Commands").clicked() {
                            self.palette_open = true;
                            self.palette_query.clear();
                            self.focus_palette = true;
                        }
                        ui.add_space(4.0);
                        if widgets::quiet_button(ui, palette, "Try it").clicked() {
                            session.execute(Command::ShowRoute {
                                route: Route::TryIt,
                            });
                        }
                    });
                });
            });
    }

    fn status(&mut self, context: &Context, session: &Session, palette: &Palette) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(30.0)
            .frame(
                egui::Frame::none()
                    .fill(palette.panel)
                    .inner_margin(Margin {
                        left: 16.0,
                        right: 16.0,
                        top: 4.0,
                        bottom: 4.0,
                    }),
            )
            .show(context, |ui| {
                ui.horizontal_centered(|ui| {
                    let text = match session.active() {
                        Some(active) => format!(
                            "{} · {} · {} rules loaded",
                            active.id,
                            session.route().label(),
                            active.keyboard.rule_count()
                        ),
                        None => "library empty".to_owned(),
                    };
                    ui.label(egui::RichText::new(text).size(11.0).color(palette.muted));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} keyboard{} installed",
                                session.entries().len(),
                                if session.entries().len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ))
                            .size(11.0)
                            .color(palette.muted),
                        );
                    });
                });
            });
    }

    fn palette_overlay(&mut self, context: &Context, session: &mut Session) {
        if !self.palette_open {
            return;
        }
        let palette = self.palette;
        egui::Area::new(egui::Id::new("command-palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(220.0, 96.0))
            .show(context, |ui| {
                egui::Frame::none()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0_f32, palette.accent))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(Margin::same(12.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 10.0),
                        blur: 30.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(120),
                    })
                    .show(ui, |ui| {
                        ui.set_width(430.0);
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.palette_query)
                                .hint_text("Run a command…")
                                .desired_width(f32::INFINITY),
                        );
                        if self.focus_palette {
                            field.request_focus();
                            self.focus_palette = false;
                        }
                        ui.add_space(6.0);
                        let query = self.palette_query.to_lowercase();
                        let mut run = None;
                        let mut shown = 0;
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                for spec in km_session::SPECS {
                                    if !query.is_empty()
                                        && !spec.id.to_lowercase().contains(&query)
                                        && !spec.label.to_lowercase().contains(&query)
                                    {
                                        continue;
                                    }
                                    shown += 1;
                                    let response = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(format!(
                                                "{}   {}",
                                                spec.id, spec.label
                                            ))
                                            .size(12.5)
                                            .color(palette.text),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .rounding(Rounding::same(8.0)),
                                    );
                                    if response.clicked() {
                                        run = Some(*spec);
                                    }
                                }
                                if shown == 0 {
                                    ui.label(
                                        egui::RichText::new("no command matches")
                                            .size(12.0)
                                            .color(palette.muted),
                                    );
                                }
                            });
                        if let Some(spec) = run {
                            self.palette_open = false;
                            let params = default_params(spec.id);
                            if let Err(error) = session.execute_id(spec.id, &params) {
                                self.toasts.push(Toast {
                                    kind: ToastKind::Warning,
                                    text: format!("{} needs a parameter", spec.id),
                                    born: session_time(),
                                });
                                let _ = error;
                            }
                        }
                    });
            });
    }

    fn toasts(&mut self, context: &Context, palette: &Palette) {
        let now = session_time();
        let mut index = 0;
        for toast in &self.toasts {
            let age = now - toast.born;
            let alpha = (1.0 - (age / TOAST_SECONDS).powi(2)).clamp(0.0, 1.0) as f32;
            let tint = match toast.kind {
                ToastKind::Info => palette.good,
                ToastKind::Warning => palette.warn,
                ToastKind::Error => palette.bad,
            };
            let id = egui::Id::new(("toast", index));
            index += 1;
            egui::Area::new(id)
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(250.0, 84.0 + 44.0 * (index as f32)))
                .show(context, |ui| {
                    egui::Frame::none()
                        .fill(palette.card)
                        .stroke(Stroke::new(1.0_f32, tint.gamma_multiply(alpha)))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(14.0, 9.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&toast.text)
                                    .size(12.5)
                                    .color(palette.text.gamma_multiply(alpha)),
                            );
                        });
                });
        }
        if !self.toasts.is_empty() {
            context.request_repaint();
        }
    }
}

/// Parameters that make a registry command runnable with no typing.
fn default_params(id: &str) -> Vec<(&'static str, String)> {
    match id {
        "appearance.animate" => vec![("on", "false".to_owned())],
        "appearance.theme" => vec![("theme", "dark".to_owned())],
        "view.route" => vec![("route", "try-it".to_owned())],
        "sandbox.type" => vec![("text", "mingalaba".to_owned())],
        _ => Vec::new(),
    }
}

/// Seconds since the process started, for the fades.
fn session_time() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_secs_f64()
}
