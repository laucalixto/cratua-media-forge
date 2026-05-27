use egui;
use egui::emath::Align2;

use crate::app::MediaForgeApp;
use crate::i18n;

impl MediaForgeApp {
    pub fn render_about(&mut self, ctx: &egui::Context) {
        let title = i18n::t(self.lang, "about-title");
        let text = i18n::t(self.lang, "about-text");
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(text);
                if ui.button("OK").clicked() {
                    self.show_about = false;
                }
            });
    }
}
