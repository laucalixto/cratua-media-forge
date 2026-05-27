// Suppress console window on Windows (GUI app, no terminal needed)
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod i18n;
mod theme;
mod views;

use mediaforge_core::config::Config;

/// Load the application icon from embedded PNG bytes
fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../../../assets/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon.png from assets/")
        .to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let config = Config::load();
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([800.0, 500.0])
            .with_drag_and_drop(true)
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Cratua Media Forge",
        options,
        Box::new(|cc| {
            theme::apply_theme(&cc.egui_ctx, &config);
            Ok(Box::new(app::MediaForgeApp::new(config.clone())))
        }),
    )
}
