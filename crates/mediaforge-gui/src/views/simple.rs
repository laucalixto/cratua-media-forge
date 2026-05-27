use egui;
use mediaforge_core::enums::DeinterlaceMethod;
use rfd;

use crate::app::MediaForgeApp;
use crate::i18n;

impl MediaForgeApp {
    pub fn render_simple_mode(&mut self, ui: &mut egui::Ui) {
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

        ui.add_space(8.0);

        let res_label = i18n::t(self.lang, "label-resolution");
        ui.horizontal(|ui| {
            ui.label(res_label);
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
        // Quality CRF
        let quality_label = i18n::t(self.lang, "label-quality");
        let mut crf = self.params.crf.unwrap_or(19);
        ui.horizontal(|ui| {
            ui.label(quality_label);
            if ui.add(egui::Slider::new(&mut crf, 0..=51).text("CRF")).changed() {
                self.params.crf = Some(crf);
            }
            ui.label("(0=lossless, 51=worst)");
        });
        // Ensure CRF is never None when leaving simple mode
        if self.params.crf.is_none() {
            self.params.crf = Some(crf);
        }

        let audio_label = i18n::t(self.lang, "label-audio-quality");
        ui.horizontal(|ui| {
            ui.label(audio_label);
            ui.add(egui::Slider::new(&mut self.params.audio_bitrate, 32..=320).text("kbps"));
        });

        let deint_label = i18n::t(self.lang, "label-deinterlace");
        let mut deint = self.params.deinterlace.is_some();
        if ui.checkbox(&mut deint, deint_label).changed() {
            self.params.deinterlace = if deint { Some(DeinterlaceMethod::Yadif) } else { None };
        }

        ui.add_space(8.0);

        let output_label = i18n::t(self.lang, "label-output-dir");
        ui.horizontal(|ui| {
            ui.label(output_label);
            ui.monospace(self.output_dir.to_string_lossy().to_string());
            if ui.button("...").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = dir;
                }
            }
        });
    }
}
