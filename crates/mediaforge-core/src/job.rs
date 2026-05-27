use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::enums::{
    AudioCodec, AudioFilter, Container, DeinterlaceMethod, FpsMode, MovFlag, PixelFormat,
    PresetCategory, PresetSpeed, Profile, ScaleAlgorithm, VideoCodec, VideoFilter,
};

/// Complete set of encoding parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeParams {
    // Video
    pub video_codec: VideoCodec,
    pub width: u32,
    pub height: u32,
    pub scale_algorithm: ScaleAlgorithm,
    pub fps: FpsMode,
    pub crf: Option<u8>,
    pub video_bitrate: Option<u32>,
    pub max_bitrate: Option<u32>,
    pub bufsize: Option<u32>,
    pub preset: PresetSpeed,
    pub profile: Option<Profile>,
    pub pixel_format: PixelFormat,
    pub deinterlace: Option<DeinterlaceMethod>,
    pub video_filters: Vec<VideoFilter>,

    // Audio
    pub audio_codec: AudioCodec,
    pub audio_bitrate: u32,
    pub audio_channels: u8,
    pub sample_rate: u32,
    pub audio_filters: Vec<AudioFilter>,

    // Output
    pub container: Container,
    pub movflags: Vec<MovFlag>,
    pub threads: u8,

    // Metadata & trim
    pub metadata: HashMap<String, String>,
    pub trim_start: Option<String>,
    pub trim_end: Option<String>,

    // Extra ffmpeg args (advanced users only)
    pub extra_args: Vec<String>,
}

impl Default for EncodeParams {
    fn default() -> Self {
        Self {
            video_codec: VideoCodec::H264,
            width: 1920,
            height: 1080,
            scale_algorithm: ScaleAlgorithm::Lanczos,
            fps: FpsMode::SameAsSource,
            crf: Some(19),
            video_bitrate: None,
            max_bitrate: None,
            bufsize: None,
            preset: PresetSpeed::Medium,
            profile: None,
            pixel_format: PixelFormat::Yuv420p,
            deinterlace: None,
            video_filters: vec![],
            audio_codec: AudioCodec::Aac,
            audio_bitrate: 128,
            audio_channels: 2,
            sample_rate: 44100,
            audio_filters: vec![],
            container: Container::Mp4,
            movflags: vec![MovFlag::FastStart],
            threads: 0,
            metadata: HashMap::new(),
            trim_start: None,
            trim_end: None,
            extra_args: vec![],
        }
    }
}

/// A preset is a named, reusable EncodeParams template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PresetCategory,
    pub params: EncodeParams,
}

/// Progress reported by ffmpeg during encoding
#[derive(Debug, Clone, Default)]
pub struct ProgressInfo {
    pub frame: u64,
    pub fps: f64,
    pub bitrate: String,
    pub total_size: u64,
    pub out_time_us: u64,
    pub speed: f64,
    pub progress_pct: f64,
}

/// Status of a single encode job
#[derive(Debug, Clone)]
pub enum JobStatus {
    Pending,
    Running(ProgressInfo),
    Completed,
    Failed(String),
    Cancelled,
}

/// A single encode job
#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub params: EncodeParams,
    pub status: JobStatus,
    pub ffmpeg_command: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Job {
    pub fn new(input_path: PathBuf, output_path: PathBuf, params: EncodeParams) -> Self {
        Self {
            id: Uuid::new_v4(),
            input_path,
            output_path,
            params,
            status: JobStatus::Pending,
            ffmpeg_command: String::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn progress_pct(&self) -> f64 {
        match &self.status {
            JobStatus::Running(info) => info.progress_pct,
            JobStatus::Completed => 100.0,
            _ => 0.0,
        }
    }
}

/// Queue of encode jobs
#[derive(Debug, Clone, Default)]
pub struct JobQueue {
    pub jobs: Vec<Job>,
}

impl JobQueue {
    pub fn add(&mut self, job: Job) {
        self.jobs.push(job);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.jobs.retain(|j| j.id != id);
    }

    pub fn pending_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| matches!(j.status, JobStatus::Pending))
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn overall_progress(&self) -> f64 {
        if self.jobs.is_empty() {
            return 0.0;
        }
        let total: f64 = self.jobs.iter().map(|j| j.progress_pct()).sum();
        total / self.jobs.len() as f64
    }
}
