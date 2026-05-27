use std::path::PathBuf;

use egui;
use mediaforge_core::ffmpeg;
use mediaforge_core::job::{Job, JobStatus, ProgressInfo};

use crate::app::{JobEvent, MediaForgeApp};
use crate::i18n;

impl MediaForgeApp {
    pub fn render_queue(&mut self, ui: &mut egui::Ui) {
        let queue_label = i18n::t(self.lang, "label-queue");
        let add_label = i18n::t(self.lang, "btn-add-to-queue");

        ui.horizontal(|ui| {
            ui.heading(queue_label);

            let can_add = !self.selected_files.is_empty() && !self.is_encoding;

            if ui
                .add_enabled(can_add, egui::Button::new(add_label))
                .clicked()
            {
                let params = self.build_params();
                // Separate files: safe (no overwrite) vs conflicting (output already exists)
                let all_files: Vec<PathBuf> = self.selected_files.drain(..).collect();
                let mut safe_files = Vec::new();
                let mut conflicts = Vec::new();

                for file in all_files {
                    let output = self.output_path(&file);
                    if output.exists() {
                        conflicts.push((file, output));
                    } else {
                        safe_files.push(file);
                    }
                }

                // Add safe files straight to queue
                let ffmpeg_path = self.ffmpeg_path.clone();
                for file in safe_files {
                    let output = self.output_path(&file);
                    let mut job = Job::new(file, output, params.clone());
                    job.ffmpeg_command = ffmpeg::command_to_string_with_ffmpeg(
                        &params,
                        &job.input_path,
                        &job.output_path,
                        ffmpeg_path.as_deref(),
                    );
                    self.job_queue.add(job);
                }

                // If there are conflicts, show the overwrite confirmation dialog
                if !conflicts.is_empty() {
                    self.overwrite_files = conflicts;
                    self.overwrite_params = Some(params);
                    self.show_overwrite_dialog = true;
                }
            }

            let can_start = !self.job_queue.jobs.is_empty() && !self.is_encoding;
            let start_label = if self.is_encoding {
                i18n::t(self.lang, "status-encoding")
            } else {
                i18n::t(self.lang, "btn-start")
            };

            if ui
                .add_enabled(can_start, egui::Button::new(start_label))
                .clicked()
            {
                self.start_encoding();
            }

            if self.is_encoding
                && ui.button(i18n::t(self.lang, "btn-cancel")).clicked()
            {
                self.cancel_encoding();
            }
        });

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                if self.job_queue.jobs.is_empty() {
                    ui.weak("No jobs in queue. Add files and click \"Add to Queue\".");
                }

                for job in &self.job_queue.jobs {
                    let in_name = job
                        .input_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let out_name = job
                        .output_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();

                    ui.horizontal(|ui| {
                        match &job.status {
                            JobStatus::Pending => {
                                ui.label(format!("{in_name} → {out_name}"))
                                    .on_hover_text(&job.ffmpeg_command);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.weak("pending");
                                    },
                                );
                            }
                            JobStatus::Running(info) => {
                                let pct = info.progress_pct.max(0.0).min(100.0);
                                ui.add_sized(
                                    [ui.available_width(), 20.0],
                                    egui::ProgressBar::new(pct as f32 / 100.0)
                                        .text(format!("{in_name} — {pct:.0}%")),
                                );
                            }
                            JobStatus::Completed => {
                                ui.label(format!("✓ {in_name} → {out_name}"));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.colored_label(egui::Color32::GREEN, "done");
                                    },
                                );
                            }
                            JobStatus::Failed(err) => {
                                ui.label(format!("✗ {in_name}"));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.colored_label(egui::Color32::RED, err);
                                    },
                                );
                            }
                            JobStatus::Cancelled => {
                                ui.label(format!("⊘ {in_name}"));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.weak("cancelled");
                                    },
                                );
                            }
                        }
                    });
                }
            });
    }

    pub fn start_encoding(&mut self) {
        self.is_encoding = true;
        self.status_message = i18n::t(self.lang, "status-encoding");

        // Create output directories before encoding starts
        for job in &self.job_queue.jobs {
            if matches!(job.status, JobStatus::Pending) {
                if let Some(parent) = job.output_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }

        // Clone sender so it stays alive for next encoding session
        let tx = self.event_tx.as_ref().unwrap().clone();
        let jobs: Vec<Job> = self.job_queue.jobs.clone();
        let ffmpeg_path = self.ffmpeg_path.clone();

        std::thread::spawn(move || {
            for job in jobs {
                if !matches!(job.status, JobStatus::Pending) {
                    continue;
                }
                let job_id = job.id;
                let params = job.params.clone();
                let input = job.input_path.clone();
                let output = job.output_path.clone();
                let tx = tx.clone();
                let ffmpeg_path = ffmpeg_path.clone();

                let result = ffmpeg::run_with_progress_and_ffmpeg(
                    &params,
                    &input,
                    &output,
                    ffmpeg_path.as_deref(),
                    |info| {
                        let _ = tx.send(JobEvent::Progress {
                            job_id,
                            info: ProgressInfo {
                                frame: info.frame,
                                fps: info.fps,
                                bitrate: info.bitrate.clone(),
                                total_size: info.total_size,
                                out_time_us: info.out_time_us,
                                speed: info.speed,
                                progress_pct: info.progress_pct,
                            },
                        });
                    },
                );

                match result {
                    Ok(()) => {
                        let _ = tx.send(JobEvent::Completed { job_id });
                    }
                    Err(e) => {
                        let _ = tx.send(JobEvent::Failed {
                            job_id,
                            error: e.to_string(),
                        });
                    }
                }
            }
        });
    }

    pub fn cancel_encoding(&mut self) {
        self.is_encoding = false;
        for job in &mut self.job_queue.jobs {
            if matches!(job.status, JobStatus::Pending | JobStatus::Running(_)) {
                job.status = JobStatus::Cancelled;
            }
        }
    }
}
