// Suppress console window on Windows (GUI app, no terminal needed)
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod i18n;
mod theme;
mod views;

use mediaforge_core::config::Config;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let config = Config::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([800.0, 500.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "MediaForge",
        options,
        Box::new(|cc| {
            theme::apply_theme(&cc.egui_ctx, &config);
            Ok(Box::new(app::MediaForgeApp::new(config.clone())))
        }),
    )
}
