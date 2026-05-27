use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum MediaForgeError {
    #[error("ffmpeg not found at `{0}`. Install ffmpeg or set the path in Settings.")]
    FfmpegNotFound(PathBuf),

    #[error("ffmpeg process failed: {0}")]
    FfmpegProcess(String),

    #[error("ffprobe error: {0}")]
    FfprobeError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unsupported codec/container combination: {video_codec:?} + {container:?}")]
    IncompatibleCodec {
        video_codec: String,
        container: String,
    },

    #[error("No input files selected")]
    NoInputFiles,

    #[error("Encoding cancelled by user")]
    Cancelled,
}
