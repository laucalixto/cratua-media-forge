use egui;
use mediaforge_core::enums::DeinterlaceMethod;
use rfd;

use crate::app::MediaForgeApp;
use crate::i18n;
use crate::theme;

impl MediaForgeApp {
    pub fn render_simple_mode(&mut self, ui: &mut egui::Ui) {
        // ── Preset selector ──
        let preset_label = i18n::t(self.lang, "label-preset");
        ui.horizontal(|ui| {
            ui.label(preset_label);
            let presets = self.presets.clone();
            let selected = presets
                .iter()
                .find(|p| p.id == self.selected_preset)
                .map(|p| p.name.as_str())
                .unwrap_or("Select...");
            egui::ComboBox::from_id_salt("preset_select")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for p in &presets {
                        if ui
                            .selectable_value(&mut self.selected_preset, p.id.clone(), &p.name)
                            .clicked()
                        {
                            self.apply_preset(&p.id);
                        }
                    }
                });
        });

        ui.add_space(6.0);
        theme::section_header(ui, &i18n::t(self.lang, "tab-video"));

        // ── Video settings grid ──
        egui::Grid::new("simple_video_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .min_col_width(120.0)
            .show(ui, |ui| {
                // Resolution
                ui.label(i18n::t(self.lang, "label-resolution"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.params.width)
                            .range(64..=7680)
                            .speed(16),
                    );
                    ui.label("×");
                    ui.add(
                        egui::DragValue::new(&mut self.params.height)
                            .range(64..=4320)
                            .speed(16),
                    );
                });
                ui.end_row();

                // Quality CRF
                ui.label(i18n::t(self.lang, "label-quality"));
                let mut crf = self.params.crf.unwrap_or(19);
                ui.horizontal(|ui| {
                    if ui.add(egui::Slider::new(&mut crf, 0..=51).text("CRF")).changed() {
                        self.params.crf = Some(crf);
                    }
                    ui.weak("(0=lossless, 51=worst)");
                });
                if self.params.crf.is_none() {
                    self.params.crf = Some(crf);
                }
                ui.end_row();

                // Audio bitrate
                ui.label(i18n::t(self.lang, "label-audio-quality"));
                ui.add(egui::Slider::new(&mut self.params.audio_bitrate, 32..=320).text("kbps"));
                ui.end_row();

                // Deinterlace
                ui.label(i18n::t(self.lang, "label-deinterlace"));
                let mut deint = self.params.deinterlace.is_some();
                if ui.checkbox(&mut deint, "").changed() {
                    self.params.deinterlace = if deint {
                        Some(DeinterlaceMethod::Yadif)
                    } else {
                        None
                    };
                }
                ui.end_row();
            });

        ui.add_space(4.0);
        theme::section_header(ui, &i18n::t(self.lang, "label-output-dir"));

        // ── Output ──
        ui.horizontal(|ui| {
            ui.monospace(self.output_dir.to_string_lossy().to_string());
            if ui.button("...").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = dir;
                }
            }
        });
    }
}
