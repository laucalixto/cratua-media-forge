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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_params_default_values() {
        let p = EncodeParams::default();
        assert_eq!(p.video_codec, crate::enums::VideoCodec::H264);
        assert_eq!(p.width, 1920);
        assert_eq!(p.height, 1080);
        assert_eq!(p.crf, Some(19));
        assert_eq!(p.audio_bitrate, 128);
        assert_eq!(p.audio_channels, 2);
        assert_eq!(p.sample_rate, 44100);
        assert_eq!(p.threads, 0);
        assert!(p.video_bitrate.is_none());
        assert!(p.profile.is_none());
    }

    #[test]
    fn job_new_has_unique_id() {
        let j1 = Job::new("in1.mp4".into(), "out1.mp4".into(), EncodeParams::default());
        let j2 = Job::new("in2.mp4".into(), "out2.mp4".into(), EncodeParams::default());
        assert_ne!(j1.id, j2.id);
    }

    #[test]
    fn job_progress_pct_pending_is_zero() {
        let j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        assert_eq!(j.progress_pct(), 0.0);
    }

    #[test]
    fn job_progress_pct_completed_is_100() {
        let mut j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        j.status = JobStatus::Completed;
        assert_eq!(j.progress_pct(), 100.0);
    }

    #[test]
    fn job_progress_pct_running_reflects_info() {
        let mut j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        j.status = JobStatus::Running(ProgressInfo { progress_pct: 45.5, ..Default::default() });
        assert_eq!(j.progress_pct(), 45.5);
    }

    #[test]
    fn job_progress_pct_failed_is_zero() {
        let mut j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        j.status = JobStatus::Failed("error".into());
        assert_eq!(j.progress_pct(), 0.0);
    }

    #[test]
    fn job_progress_pct_cancelled_is_zero() {
        let mut j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        j.status = JobStatus::Cancelled;
        assert_eq!(j.progress_pct(), 0.0);
    }

    #[test]
    fn job_queue_add_and_count() {
        let mut q = JobQueue::default();
        assert_eq!(q.pending_count(), 0);
        q.add(Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default()));
        assert_eq!(q.pending_count(), 1);
        assert_eq!(q.total_count(), 1);
    }

    #[test]
    fn job_queue_remove() {
        let mut q = JobQueue::default();
        let j = Job::new("in.mp4".into(), "out.mp4".into(), EncodeParams::default());
        let id = j.id;
        q.add(j);
        q.remove(id);
        assert_eq!(q.total_count(), 0);
    }

    #[test]
    fn job_queue_overall_progress_empty() {
        let q = JobQueue::default();
        assert_eq!(q.overall_progress(), 0.0);
    }

    #[test]
    fn job_queue_overall_progress_averages() {
        let mut q = JobQueue::default();
        let mut j1 = Job::new("in1.mp4".into(), "out1.mp4".into(), EncodeParams::default());
        j1.status = JobStatus::Completed;
        let mut j2 = Job::new("in2.mp4".into(), "out2.mp4".into(), EncodeParams::default());
        j2.status = JobStatus::Running(ProgressInfo { progress_pct: 50.0, ..Default::default() });
        q.add(j1);
        q.add(j2);
        assert!((q.overall_progress() - 75.0).abs() < 0.01);
    }

    #[test]
    fn preset_serialization_roundtrip() {
        let preset = Preset {
            id: "test".into(),
            name: "Test".into(),
            description: "desc".into(),
            category: crate::enums::PresetCategory::Video,
            params: EncodeParams::default(),
        };
        let json = serde_json::to_string(&preset).unwrap();
        let p2: Preset = serde_json::from_str(&json).unwrap();
        assert_eq!(p2.id, "test");
        assert_eq!(p2.params.crf, Some(19));
    }
}

