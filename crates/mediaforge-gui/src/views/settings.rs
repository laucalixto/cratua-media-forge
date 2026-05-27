use std::path::PathBuf;

use egui;
use egui::emath::Align2;
use mediaforge_core::enums::Theme;
use mediaforge_core::i18n::Language;

use crate::app::MediaForgeApp;
use crate::i18n;
use crate::theme;

impl MediaForgeApp {
    pub fn render_settings(&mut self, ctx: &egui::Context) {
        let title = i18n::t(self.lang, "settings-title");
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let lang_label = i18n::t(self.lang, "settings-language");
                ui.horizontal(|ui| {
                    ui.label(lang_label);
                    egui::ComboBox::from_id_salt("lang_select")
                        .selected_text(self.lang.label())
                        .show_ui(ui, |ui| {
                            for lang in Language::ALL {
                                ui.selectable_value(&mut self.lang, *lang, lang.label());
                            }
                        });
                });

                let theme_label = i18n::t(self.lang, "settings-theme");
                ui.horizontal(|ui| {
                    ui.label(theme_label);
                    egui::ComboBox::from_id_salt("theme_select")
                        .selected_text(match self.config.theme {
                            Theme::Dark => "Dark",
                            Theme::Light => "Light",
                            Theme::System => "System",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.config.theme, Theme::Dark, "Dark");
                            ui.selectable_value(&mut self.config.theme, Theme::Light, "Light");
                            ui.selectable_value(&mut self.config.theme, Theme::System, "System");
                        });
                });

                let ffmpeg_label = i18n::t(self.lang, "settings-ffmpeg-path");
                ui.horizontal(|ui| {
                    ui.label(ffmpeg_label);
                    let mut path_str = self
                        .config
                        .ffmpeg_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    ui.text_edit_singleline(&mut path_str);
                    if !path_str.is_empty() {
                        self.config.ffmpeg_path = Some(PathBuf::from(&path_str));
                    } else {
                        self.config.ffmpeg_path = None;
                    }
                });

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        let _ = self.config.save();
                        theme::apply_theme(ctx, &self.config);
                        self.show_settings = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.lang = self.config.language;
                        self.show_settings = false;
                    }
                });
            });
    }
}
