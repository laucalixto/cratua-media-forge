use std::path::PathBuf;
use std::sync::mpsc;

use egui;

use mediaforge_core::config::Config;
use mediaforge_core::ffmpeg;
use mediaforge_core::job::{EncodeParams, JobQueue, JobStatus, ProgressInfo};
use mediaforge_core::preset;
use uuid::Uuid;

use crate::i18n;

// ── Enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedTab {
    Video,
    Audio,
    Filters,
    Output,
    Metadata,
}

/// Events sent from the encoding background task to the GUI
#[derive(Debug, Clone)]
pub enum JobEvent {
    Progress { job_id: Uuid, info: ProgressInfo },
    Completed { job_id: Uuid },
    Failed { job_id: Uuid, error: String },
}

// ── App State ──

/// Main application state
pub struct MediaForgeApp {
    pub mode: mediaforge_core::enums::UiMode,
    pub job_queue: JobQueue,
    pub presets: Vec<mediaforge_core::job::Preset>,
    pub selected_preset: String,
    pub selected_files: Vec<PathBuf>,
    pub output_dir: PathBuf,

    /// Canonical encode params (shared by simple and advanced modes)
    pub params: EncodeParams,

    /// Advanced mode tab
    pub advanced_tab: AdvancedTab,

    /// Temp fields for metadata input
    pub new_meta_key: String,
    pub new_meta_val: String,

    /// Pending file dialog flag (for Ctrl+O shortcut)
    pub pending_open_dialog: bool,

    /// Manual file path input (fallback when native dialog unavailable)
    pub manual_path_input: String,

    // Encoding state
    pub is_encoding: bool,
    pub event_tx: Option<mpsc::Sender<JobEvent>>,
    pub event_rx: mpsc::Receiver<JobEvent>,

    // ffmpeg location
    pub ffmpeg_path: Option<PathBuf>,

    // UI state
    pub show_settings: bool,
    pub show_about: bool,
    pub show_overwrite_dialog: bool,
    pub status_message: String,

    /// Pending files that conflict (input, output) — awaiting user overwrite decision
    pub overwrite_files: Vec<(PathBuf, PathBuf)>,
    /// Parameters to use when user confirms overwrite
    pub overwrite_params: Option<EncodeParams>,

    // Config
    pub config: Config,
    pub lang: mediaforge_core::i18n::Language,
}

// ── Core methods (state, events, helpers) ──

impl MediaForgeApp {
    pub fn new(config: Config) -> Self {
        let lang = config.language;
        let presets = preset::builtin_presets();
        let output_dir = config
            .output_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let (event_tx, event_rx) = mpsc::channel();

        // Detect ffmpeg: config override first, then auto-detect
        let ffmpeg_path = config
            .ffmpeg_path
            .as_ref()
            .filter(|p| p.exists())
            .cloned()
            .or_else(|| ffmpeg::detect_ffmpeg());

        // Start with web-h264 preset params
        let params = presets
            .iter()
            .find(|p| p.id == "web-h264")
            .map(|p| p.params.clone())
            .unwrap_or_default();

        Self {
            mode: config.default_mode,
            job_queue: JobQueue::default(),
            presets,
            selected_preset: "web-h264".into(),
            selected_files: Vec::new(),
            output_dir,
            params,
            advanced_tab: AdvancedTab::Video,
            new_meta_key: String::new(),
            new_meta_val: String::new(),
            pending_open_dialog: false,
            manual_path_input: String::new(),
            is_encoding: false,
            event_tx: Some(event_tx),
            event_rx,
            ffmpeg_path,
            show_settings: false,
            show_about: false,
            show_overwrite_dialog: false,
            status_message: i18n::t(lang, "status-ready"),
            overwrite_files: Vec::new(),
            overwrite_params: None,
            config,
            lang,
        }
    }

    /// Apply a preset's params, keeping width/height if preset doesn't specify
    pub fn apply_preset(&mut self, preset_id: &str) {
        if let Some(p) = self.presets.iter().find(|p| p.id == preset_id) {
            let mut new_params = p.params.clone();
            if new_params.width == 1920 && new_params.height == 1080 && self.params.width != 1920 {
                new_params.width = self.params.width;
                new_params.height = self.params.height;
            }
            self.params = new_params;
            self.selected_preset = preset_id.to_string();
        }
    }

    pub fn build_params(&self) -> EncodeParams {
        self.params.clone()
    }

    /// Process any pending job events from the encoding task
    pub fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                JobEvent::Progress { job_id, info } => {
                    if let Some(job) = self.job_queue.jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = JobStatus::Running(info);
                    }
                }
                JobEvent::Completed { job_id } => {
                    if let Some(job) = self.job_queue.jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = JobStatus::Completed;
                    }
                }
                JobEvent::Failed { job_id, error } => {
                    if let Some(job) = self.job_queue.jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = JobStatus::Failed(error);
                    }
                }
            }
        }

        if self.is_encoding
            && self.job_queue.jobs.iter().all(|j| {
                matches!(
                    j.status,
                    JobStatus::Completed | JobStatus::Failed(_) | JobStatus::Cancelled
                )
            })
        {
            self.is_encoding = false;
            let completed = self
                .job_queue
                .jobs
                .iter()
                .filter(|j| matches!(j.status, JobStatus::Completed))
                .count();
            self.status_message = match self.lang {
                mediaforge_core::i18n::Language::EnUs => {
                    format!("Complete — {completed} file(s) processed")
                }
                mediaforge_core::i18n::Language::PtBr => {
                    format!("Concluído — {completed} arquivo(s) processado(s)")
                }
            };
            self.job_queue.jobs.clear();
            self.selected_files.clear();
        }
    }

    pub fn output_path(&self, input: &std::path::Path) -> PathBuf {
        let ext = self.params.container.extension();
        let stem = input.file_stem().unwrap_or_default();
        self.output_dir.join("export").join(stem).with_extension(ext)
    }

    /// Add a file path from the manual input field
    pub fn add_manual_path(&mut self) {
        let path = self.manual_path_input.trim().to_string();
        if !path.is_empty() {
            let p = PathBuf::from(&path);
            if p.exists() {
                self.selected_files.push(p);
            }
            self.manual_path_input.clear();
        }
    }
}

// ── eframe::App (orchestration) ──

impl eframe::App for MediaForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();

        let ctx = ui.ctx().clone();
        if self.is_encoding {
            ctx.request_repaint();
        }

        // Handle drag & drop
        if !self.is_encoding {
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    let files: Vec<PathBuf> = i
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|f| f.path.clone())
                        .collect();
                    if !files.is_empty() {
                        self.selected_files.extend(files);
                    }
                }
            });
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.pending_open_dialog = true;
            }
            if i.modifiers.ctrl
                && i.key_pressed(egui::Key::Enter)
                && !self.job_queue.jobs.is_empty()
                && !self.is_encoding
            {
                self.start_encoding();
            }
            if i.key_pressed(egui::Key::Escape) && self.is_encoding {
                self.cancel_encoding();
            }
        });

        // Handle pending file dialog
        if self.pending_open_dialog {
            self.pending_open_dialog = false;
            if let Some(files) = rfd::FileDialog::new()
                .add_filter(
                    "Media",
                    &[
                        "mp4", "mkv", "mov", "avi", "webm", "mp3", "wav", "flac", "m4a", "ogg",
                    ],
                )
                .pick_files()
            {
                self.selected_files.extend(files);
            }
        }

        // Top menu bar
        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let settings_label = i18n::t(self.lang, "btn-settings");
                ui.menu_button(settings_label, |ui| {
                    if ui.button(i18n::t(self.lang, "btn-settings")).clicked() {
                        self.show_settings = true;
                        ui.close();
                    }
                    if ui.button(i18n::t(self.lang, "btn-about")).clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let simple = i18n::t(self.lang, "simple-mode");
                    let advanced = i18n::t(self.lang, "advanced-mode");
                    ui.selectable_value(
                        &mut self.mode,
                        mediaforge_core::enums::UiMode::Simple,
                        simple,
                    );
                    ui.selectable_value(
                        &mut self.mode,
                        mediaforge_core::enums::UiMode::Advanced,
                        advanced,
                    );
                });
            });
        });

        // Main content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let files_label = i18n::t(self.lang, "label-files");
            let add_files_label = i18n::t(self.lang, "btn-add-files");
            let clear_label = i18n::t(self.lang, "btn-clear-files");

            ui.horizontal(|ui| {
                ui.label(files_label);
                if ui
                    .add_enabled(!self.is_encoding, egui::Button::new(add_files_label))
                    .clicked()
                {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter(
                            "Media",
                            &[
                                "mp4", "mkv", "mov", "avi", "webm", "mp3", "wav", "flac", "m4a",
                                "ogg",
                            ],
                        )
                        .pick_files()
                    {
                        self.selected_files.extend(files);
                    }
                }
                if !self.selected_files.is_empty()
                    && !self.is_encoding
                    && ui.button(clear_label).clicked()
                {
                    self.selected_files.clear();
                }
            });

            // Manual path input (fallback when native file dialog is unavailable, e.g. WSL)
            ui.horizontal(|ui| {
                ui.label("Path:");
                let resp = ui.add_sized(
                    [ui.available_width() - 40.0, 20.0],
                    egui::TextEdit::singleline(&mut self.manual_path_input)
                        .hint_text("/path/to/file.mp4"),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.add_manual_path();
                }
                if ui.button("Add").clicked() {
                    self.add_manual_path();
                }
            });

            // Show selected files
            if !self.selected_files.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(60.0)
                    .show(ui, |ui| {
                        for file in &self.selected_files.clone() {
                            ui.label(file.display().to_string());
                        }
                    });
            }

            ui.separator();

            match self.mode {
                mediaforge_core::enums::UiMode::Simple => self.render_simple_mode(ui),
                mediaforge_core::enums::UiMode::Advanced => self.render_advanced_mode(ui),
            }

            ui.separator();

            self.render_queue(ui);
        });

        // Status bar
        egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{} files | {} jobs",
                        self.selected_files.len(),
                        self.job_queue.total_count()
                    ));
                });
            });
        });

        // Dialogs
        if self.show_settings {
            self.render_settings(&ctx);
        }
        if self.show_about {
            self.render_about(&ctx);
        }
        if self.show_overwrite_dialog {
            self.render_overwrite_dialog(&ctx);
        }
    }
}
