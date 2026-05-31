use mediaforge_core::config::Config;
use mediaforge_core::ffmpeg;
use mediaforge_core::job::{EncodeParams, Job, Preset, ProgressInfo};
use mediaforge_core::preset;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;

// ── Cancellation ──
static CANCEL_FLAG: std::sync::OnceLock<Mutex<Option<Arc<AtomicBool>>>> = std::sync::OnceLock::new();

// ── History file lock ──
static HISTORY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ── Custom presets file lock ──
static PRESET_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ── Lightweight job data for IPC ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobData {
    pub input_path: String,
    pub output_path: String,
    pub params: EncodeParams,
}

// ── Job history entry (persisted) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub input_path: String,
    pub output_path: String,
    pub status: String,
    pub error: Option<String>,
    pub created_at: u64,
    pub duration_secs: Option<f64>,
}

fn history_path() -> PathBuf {
    Config::config_dir().join("mediaforge").join("history.json")
}

fn load_history() -> Vec<HistoryEntry> {
    let _lock = HISTORY_LOCK.lock().unwrap();
    let path = history_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    }
}

fn save_history(entries: &[HistoryEntry]) {
    let _lock = HISTORY_LOCK.lock().unwrap();
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Custom presets persistence ──

fn custom_presets_path() -> PathBuf {
    Config::config_dir().join("mediaforge").join("presets.json")
}

fn load_custom_presets() -> Vec<Preset> {
    let _lock = PRESET_LOCK.lock().unwrap();
    let path = custom_presets_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    }
}

fn save_custom_presets(presets: &[Preset]) {
    let _lock = PRESET_LOCK.lock().unwrap();
    let path = custom_presets_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(presets) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Config ──

#[tauri::command]
pub fn get_config() -> Config {
    Config::load()
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

// ── Presets ──

#[tauri::command]
pub fn get_presets() -> Vec<Preset> {
    let mut presets = preset::builtin_presets();
    let custom = load_custom_presets();
    presets.extend(custom);
    presets
}

#[tauri::command]
pub fn get_builtin_preset_ids() -> Vec<String> {
    preset::builtin_presets().iter().map(|p| p.id.clone()).collect()
}

#[tauri::command]
pub fn create_preset(preset: Preset) -> Result<(), String> {
    let builtin = preset::builtin_presets();
    let builtin_ids: Vec<&str> = builtin.iter().map(|p| p.id.as_str()).collect();
    if builtin_ids.contains(&preset.id.as_str()) {
        return Err(format!("Cannot overwrite built-in preset '{}'", preset.id));
    }
    let mut custom = load_custom_presets();
    custom.retain(|p| p.id != preset.id);
    custom.push(preset);
    save_custom_presets(&custom);
    Ok(())
}

#[tauri::command]
pub fn delete_preset(id: String) -> Result<(), String> {
    let builtin = preset::builtin_presets();
    let builtin_ids: Vec<&str> = builtin.iter().map(|p| p.id.as_str()).collect();
    if builtin_ids.contains(&id.as_str()) {
        return Err(format!("Cannot delete built-in preset '{}'", id));
    }
    let mut custom = load_custom_presets();
    custom.retain(|p| p.id != id);
    save_custom_presets(&custom);
    Ok(())
}

#[tauri::command]
pub fn get_default_output_dir() -> String {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("output").to_string_lossy().to_string()
}

#[tauri::command]
pub fn check_output_overwrite(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| std::path::Path::new(p).exists())
        .collect()
}

// ── ffmpeg ──

#[tauri::command]
pub fn detect_ffmpeg(custom_path: Option<String>) -> Option<String> {
    if let Some(ref path) = custom_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(path.clone());
        }
    }
    ffmpeg::detect_ffmpeg().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn build_command_preview(params: EncodeParams) -> String {
    let dummy_in = std::path::Path::new("input.mp4");
    let dummy_out = std::path::Path::new("output.mp4");
    ffmpeg::command_to_string(&params, dummy_in, dummy_out)
}

// ── Encoding ──

#[tauri::command]
pub async fn start_encoding(
    job_data: Vec<JobData>,
    ffmpeg_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    let ffmpeg_path = ffmpeg_path.map(PathBuf::from);
    let jobs: Vec<Job> = job_data
        .into_iter()
        .map(|d| {
            Job::new(
                PathBuf::from(d.input_path),
                PathBuf::from(d.output_path),
                d.params,
            )
        })
        .collect();

    // Validate: prevent overwriting source file
    for job in &jobs {
        if job.input_path == job.output_path {
            return Err(format!(
                "Output path is the same as input: {} — this would overwrite the source file.",
                job.input_path.display()
            ));
        }
    }

    // Dragon Trap: validate at least one rate control method is set
    // Skip when video codec is Copy — no re-encoding, no rate control needed
    for job in &jobs {
        if job.params.video_codec != mediaforge_core::enums::VideoCodec::Copy
            && job.params.crf.is_none()
            && job.params.video_bitrate.is_none()
        {
            return Err(
                "Dragon Trap: neither CRF nor video bitrate is set.".into()
            );
        }
    }

    // Create output directories
    for job in &jobs {
        if let Some(parent) = job.output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    // Probe durations for progress tracking
    let durations: Vec<Option<u64>> = jobs
        .iter()
        .map(|j| ffmpeg::probe_duration(&j.input_path, ffmpeg_path.as_deref()))
        .collect();

    // Diagnostic: log probe results
    for (i, d) in durations.iter().enumerate() {
        let name = jobs[i].input_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        eprintln!("[mediaforge] probe_duration({}): {:?}", name, d.map(|us| format!("{:.1}s", us as f64 / 1_000_000.0)));
        let _ = window.emit("job:diag", serde_json::json!({
            "file": name,
            "probe_ok": d.is_some(),
        }));
    }

    // Create a fresh cancel flag
    let cancel_flag = Arc::new(AtomicBool::new(false));
    *CANCEL_FLAG
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap() = Some(cancel_flag.clone());

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for (i, job) in jobs.into_iter().enumerate() {
                let job_id = job.id.to_string();
                let duration_us = durations.get(i).copied().flatten();
                let start = std::time::Instant::now();

                let result = ffmpeg::run_with_progress_and_ffmpeg_cancellable(
                    &job.params,
                    &job.input_path,
                    &job.output_path,
                    ffmpeg_path.as_deref(),
                    Some(cancel_flag.clone()),
                    duration_us,
                    |info: &ProgressInfo| {
                        let _ = window.emit(
                            "job:progress",
                            serde_json::json!({
                                "job_id": job_id,
                                "progress_pct": info.progress_pct,
                                "speed": info.speed,
                                "fps": info.fps,
                                "out_time_us": info.out_time_us,
                            }),
                        );
                    },
                );

                let duration = start.elapsed().as_secs_f64();

                let entry = match &result {
                    Ok(cmd_str) => {
                        let _ = window.emit(
                            "job:done",
                            serde_json::json!({
                                "job_id": job_id,
                                "status": "completed",
                                "error": null,
                                "command": cmd_str,
                            }),
                        );
                        HistoryEntry {
                            id: job_id.clone(),
                            input_path: job.input_path.to_string_lossy().to_string(),
                            output_path: job.output_path.to_string_lossy().to_string(),
                            status: "completed".into(),
                            error: None,
                            created_at: timestamp,
                            duration_secs: Some(duration),
                        }
                    }
                    Err(e) => {
                        let status = if matches!(e, mediaforge_core::MediaForgeError::Cancelled) {
                            "cancelled"
                        } else {
                            "failed"
                        };
                        let _ = window.emit(
                            "job:done",
                            serde_json::json!({
                                "job_id": job_id,
                                "status": status,
                                "error": e.to_string(),
                            }),
                        );
                        HistoryEntry {
                            id: job_id.clone(),
                            input_path: job.input_path.to_string_lossy().to_string(),
                            output_path: job.output_path.to_string_lossy().to_string(),
                            status: status.into(),
                            error: Some(e.to_string()),
                            created_at: timestamp,
                            duration_secs: Some(duration),
                        }
                    }
                };

                // Append to history (lock-protected)
                let mut history = load_history();
                history.push(entry);
                if history.len() > 200 {
                    history.drain(0..history.len() - 200);
                }
                save_history(&history);
            }
        }));

        // Handle panic
        if let Err(panic_err) = result {
            let msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".into()
            };
            eprintln!("Encoding thread panicked: {}", msg);
        }

        // Always clear cancel flag
        if let Ok(mut guard) = CANCEL_FLAG.get_or_init(|| Mutex::new(None)).lock() {
            *guard = None;
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_encoding() {
    if let Ok(guard) = CANCEL_FLAG
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        if let Some(ref flag) = *guard {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

// ── History ──

#[tauri::command]
pub fn get_history() -> Vec<HistoryEntry> {
    load_history()
}

#[tauri::command]
pub fn clear_history() {
    let path = history_path();
    let _ = std::fs::remove_file(&path);
}
