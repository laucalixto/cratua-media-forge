//! MediaForge visual theme — "Forge" (dark industrial with amber accents).

use egui::{Color32, Context, CornerRadius, Stroke, Style, Visuals};
use mediaforge_core::config::Config;
use mediaforge_core::enums::Theme;

// ── Forge Color Palette ──

pub mod colors {
    use egui::Color32;
    pub const BG_DEEP: Color32 = Color32::from_rgb(18, 18, 36);
    pub const BG_SURFACE: Color32 = Color32::from_rgb(28, 28, 52);
    pub const BG_CARD: Color32 = Color32::from_rgb(36, 36, 64);
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(44, 44, 76);
    pub const ACCENT: Color32 = Color32::from_rgb(240, 165, 0);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(180, 120, 0);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(224, 224, 235);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 150, 175);
    #[allow(dead_code)]
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 125);
    pub const BORDER: Color32 = Color32::from_rgb(60, 60, 90);
    #[allow(dead_code)]
    pub const SUCCESS: Color32 = Color32::from_rgb(76, 175, 80);
    pub const ERROR: Color32 = Color32::from_rgb(239, 83, 80);
    pub const WARNING: Color32 = Color32::from_rgb(255, 167, 38);
    #[allow(dead_code)]
    pub const INFO: Color32 = Color32::from_rgb(66, 165, 245);
    pub const LIGHT_BG: Color32 = Color32::from_rgb(245, 245, 250);
    pub const LIGHT_SURFACE: Color32 = Color32::from_rgb(255, 255, 255);
    pub const LIGHT_CARD: Color32 = Color32::from_rgb(248, 248, 254);
    pub const LIGHT_BORDER: Color32 = Color32::from_rgb(220, 220, 230);
    pub const LIGHT_TEXT: Color32 = Color32::from_rgb(30, 30, 50);
    pub const LIGHT_TEXT_SEC: Color32 = Color32::from_rgb(100, 100, 120);
}

// ── Theme application ──

pub fn apply_theme(ctx: &Context, config: &Config) {
    match config.theme {
        Theme::Dark => apply_dark(ctx),
        Theme::Light => apply_light(ctx),
        Theme::System => apply_dark(ctx),
    }
}

fn apply_dark(ctx: &Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(colors::TEXT_PRIMARY);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.window_fill = colors::BG_SURFACE;
    visuals.panel_fill = colors::BG_DEEP;
    visuals.faint_bg_color = colors::BG_CARD;
    visuals.extreme_bg_color = colors::BG_ELEVATED;
    visuals.code_bg_color = colors::BG_CARD;
    visuals.warn_fg_color = colors::WARNING;
    visuals.error_fg_color = colors::ERROR;
    visuals.hyperlink_color = colors::ACCENT;
    visuals.selection = egui::style::Selection {
        bg_fill: colors::ACCENT.linear_multiply(0.3),
        stroke: Stroke::new(1.0, colors::ACCENT),
    };
    visuals.widgets.noninteractive =
        make_widget(colors::BG_CARD, colors::TEXT_SECONDARY, colors::BORDER);
    visuals.widgets.inactive =
        make_widget(colors::BG_ELEVATED, colors::TEXT_PRIMARY, colors::BORDER);
    visuals.widgets.hovered =
        make_widget(colors::BG_ELEVATED, colors::ACCENT, colors::ACCENT);
    visuals.widgets.active =
        make_widget(colors::ACCENT.linear_multiply(0.15), colors::ACCENT, colors::ACCENT);
    visuals.widgets.open =
        make_widget(colors::BG_ELEVATED, colors::ACCENT, colors::ACCENT);
    ctx.set_visuals(visuals);

    let mut style: Style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.indent = 16.0;
    style.spacing.slider_width = 140.0;
    style.spacing.combo_width = 180.0;
    ctx.set_global_style(style);
}

fn apply_light(ctx: &Context) {
    let mut visuals = Visuals::light();
    visuals.override_text_color = Some(colors::LIGHT_TEXT);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.window_fill = colors::LIGHT_SURFACE;
    visuals.panel_fill = colors::LIGHT_BG;
    visuals.faint_bg_color = colors::LIGHT_CARD;
    visuals.extreme_bg_color = colors::LIGHT_SURFACE;
    visuals.code_bg_color = colors::LIGHT_CARD;
    visuals.warn_fg_color = colors::WARNING;
    visuals.error_fg_color = colors::ERROR;
    visuals.hyperlink_color = colors::ACCENT_DIM;
    visuals.selection = egui::style::Selection {
        bg_fill: colors::ACCENT.linear_multiply(0.15),
        stroke: Stroke::new(1.0, colors::ACCENT_DIM),
    };
    visuals.widgets.noninteractive =
        make_widget(colors::LIGHT_CARD, colors::LIGHT_TEXT_SEC, colors::LIGHT_BORDER);
    visuals.widgets.inactive =
        make_widget(colors::LIGHT_SURFACE, colors::LIGHT_TEXT, colors::LIGHT_BORDER);
    visuals.widgets.hovered =
        make_widget(colors::LIGHT_CARD, colors::ACCENT_DIM, colors::ACCENT_DIM);
    visuals.widgets.active =
        make_widget(colors::ACCENT.linear_multiply(0.08), colors::ACCENT_DIM, colors::ACCENT_DIM);
    visuals.widgets.open =
        make_widget(colors::LIGHT_CARD, colors::ACCENT_DIM, colors::ACCENT_DIM);
    ctx.set_visuals(visuals);

    let mut style: Style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.indent = 16.0;
    ctx.set_global_style(style);
}

fn make_widget(bg: Color32, fg: Color32, border: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, border),
        corner_radius: CornerRadius::same(6),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 2.0,
    }
}

// ── Utility functions for views ──

/// Draw a section card with a rounded background
#[allow(dead_code)]
pub fn section_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(colors::BG_CARD)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Vec2::splat(12.0))
        .outer_margin(egui::Vec2::new(0.0, 4.0))
        .show(ui, |ui| {
            if !title.is_empty() {
                ui.strong(title);
                ui.add_space(6.0);
            }
            add_contents(ui)
        })
        .inner
}

/// Draw a section header with accent color
pub fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.add_space(8.0);
    ui.colored_label(colors::ACCENT, label);
    ui.separator();
}
