use egui::{Context, CornerRadius};
use mediaforge_core::config::Config;
use mediaforge_core::enums::Theme;

pub fn apply_theme(ctx: &Context, config: &Config) {
    match config.theme {
        Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        Theme::System => {
            ctx.set_visuals(egui::Visuals::dark());
        }
    }

    let mut style = (*ctx.global_style()).clone();
    style.visuals.window_corner_radius = CornerRadius::same(8);
    ctx.set_global_style(style);
}
