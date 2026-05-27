use egui;
use egui::emath::Align2;
use mediaforge_core::ffmpeg;
use mediaforge_core::job::{EncodeParams, Job};

use crate::app::MediaForgeApp;
use crate::i18n;

impl MediaForgeApp {
    pub fn render_overwrite_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new(i18n::t(self.lang, "overwrite-title"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(i18n::t(self.lang, "overwrite-message"));
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (input, output) in &self.overwrite_files {
                            ui.label(format!(
                                "{} -> {}",
                                input.file_name().unwrap_or_default().to_string_lossy(),
                                output.display()
                            ));
                        }
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(i18n::t(self.lang, "overwrite-yes")).clicked() {
                        self.confirm_overwrite();
                    }
                    if ui.button(i18n::t(self.lang, "overwrite-no")).clicked() {
                        self.show_overwrite_dialog = false;
                        self.overwrite_files.clear();
                        self.overwrite_params = None;
                    }
                });
            });
    }

    /// User confirmed overwrite — add conflicting files to queue
    pub fn confirm_overwrite(&mut self) {
        let params = self
            .overwrite_params
            .take()
            .unwrap_or_else(EncodeParams::default);
        let ffmpeg_path = self.ffmpeg_path.clone();

        for (input, output) in self.overwrite_files.drain(..) {
            // Create parent dirs (e.g. export/)
            if let Some(parent) = output.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut job = Job::new(input, output, params.clone());
            job.ffmpeg_command = ffmpeg::command_to_string_with_ffmpeg(
                &params,
                &job.input_path,
                &job.output_path,
                ffmpeg_path.as_deref(),
            );
            self.job_queue.add(job);
        }

        self.show_overwrite_dialog = false;
        self.overwrite_params = None;
    }
}
