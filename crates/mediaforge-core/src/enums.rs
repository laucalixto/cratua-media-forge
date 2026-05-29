use serde::{Deserialize, Serialize};
use std::fmt;

// ── Video ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    VP9,
    AV1,
    SVTAV1,
    Copy,
}

impl VideoCodec {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::H265 => "libx265",
            VideoCodec::VP9 => "libvpx-vp9",
            VideoCodec::AV1 => "libaom-av1",
            VideoCodec::SVTAV1 => "libsvtav1",
            VideoCodec::Copy => "copy",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "H.264 (libx264)",
            VideoCodec::H265 => "H.265 / HEVC (libx265)",
            VideoCodec::VP9 => "VP9 (libvpx-vp9)",
            VideoCodec::AV1 => "AV1 (libaom-av1)",
            VideoCodec::SVTAV1 => "AV1 (libsvtav1)",
            VideoCodec::Copy => "Copy (no re-encode)",
        }
    }

    pub const ALL: &[VideoCodec] = &[
        VideoCodec::H264,
        VideoCodec::H265,
        VideoCodec::VP9,
        VideoCodec::AV1,
        VideoCodec::SVTAV1,
        VideoCodec::Copy,
    ];
}

// ── Audio ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Mp3,
    Opus,
    Vorbis,
    Flac,
    Copy,
}

impl AudioCodec {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            AudioCodec::Aac => "aac",
            AudioCodec::Mp3 => "libmp3lame",
            AudioCodec::Opus => "libopus",
            AudioCodec::Vorbis => "libvorbis",
            AudioCodec::Flac => "flac",
            AudioCodec::Copy => "copy",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AudioCodec::Aac => "AAC",
            AudioCodec::Mp3 => "MP3 (libmp3lame)",
            AudioCodec::Opus => "Opus (libopus)",
            AudioCodec::Vorbis => "Vorbis (libvorbis)",
            AudioCodec::Flac => "FLAC",
            AudioCodec::Copy => "Copy (no re-encode)",
        }
    }

    pub const ALL: &[AudioCodec] = &[
        AudioCodec::Aac,
        AudioCodec::Mp3,
        AudioCodec::Opus,
        AudioCodec::Vorbis,
        AudioCodec::Flac,
        AudioCodec::Copy,
    ];
}

// ── Container ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    Mp4,
    Mkv,
    Webm,
    Mov,
    Avi,
    Gif,
    Mp3,
}

impl Container {
    pub fn extension(&self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
            Container::Webm => "webm",
            Container::Mov => "mov",
            Container::Avi => "avi",
            Container::Gif => "gif",
            Container::Mp3 => "mp3",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Container::Mp4 => "MP4",
            Container::Mkv => "MKV",
            Container::Webm => "WebM",
            Container::Mov => "MOV",
            Container::Avi => "AVI",
            Container::Gif => "GIF",
            Container::Mp3 => "MP3 (audio only)",
        }
    }

    pub const ALL: &[Container] = &[
        Container::Mp4,
        Container::Mkv,
        Container::Webm,
        Container::Mov,
        Container::Avi,
        Container::Gif,
        Container::Mp3,
    ];
}

// ── Preset speed ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetSpeed {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

impl PresetSpeed {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            PresetSpeed::Ultrafast => "ultrafast",
            PresetSpeed::Superfast => "superfast",
            PresetSpeed::Veryfast => "veryfast",
            PresetSpeed::Faster => "faster",
            PresetSpeed::Fast => "fast",
            PresetSpeed::Medium => "medium",
            PresetSpeed::Slow => "slow",
            PresetSpeed::Slower => "slower",
            PresetSpeed::Veryslow => "veryslow",
        }
    }

    pub const ALL: &[PresetSpeed] = &[
        PresetSpeed::Ultrafast,
        PresetSpeed::Superfast,
        PresetSpeed::Veryfast,
        PresetSpeed::Faster,
        PresetSpeed::Fast,
        PresetSpeed::Medium,
        PresetSpeed::Slow,
        PresetSpeed::Slower,
        PresetSpeed::Veryslow,
    ];
}

// ── Profile ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    Baseline,
    Main,
    High,
}

impl Profile {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            Profile::Baseline => "baseline",
            Profile::Main => "main",
            Profile::High => "high",
        }
    }

    pub const ALL: &[Profile] = &[Profile::Baseline, Profile::Main, Profile::High];
}

// ── Pixel format ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Yuv420p,
    Yuv422p,
    Yuv444p,
    Nv12,
    Rgb24,
}

impl PixelFormat {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            PixelFormat::Yuv420p => "yuv420p",
            PixelFormat::Yuv422p => "yuv422p",
            PixelFormat::Yuv444p => "yuv444p",
            PixelFormat::Nv12 => "nv12",
            PixelFormat::Rgb24 => "rgb24",
        }
    }

    pub const ALL: &[PixelFormat] = &[
        PixelFormat::Yuv420p,
        PixelFormat::Yuv422p,
        PixelFormat::Yuv444p,
        PixelFormat::Nv12,
        PixelFormat::Rgb24,
    ];
}

// ── Deinterlace ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeinterlaceMethod {
    Yadif,
    Bwdif,
}

impl DeinterlaceMethod {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            DeinterlaceMethod::Yadif => "yadif",
            DeinterlaceMethod::Bwdif => "bwdif",
        }
    }
}

// ── Scale algorithm ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScaleAlgorithm {
    Bilinear,
    Bicubic,
    Lanczos,
}

impl ScaleAlgorithm {
    pub fn ffmpeg_flag(&self) -> &'static str {
        match self {
            ScaleAlgorithm::Bilinear => "bilinear",
            ScaleAlgorithm::Bicubic => "bicubic",
            ScaleAlgorithm::Lanczos => "lanczos",
        }
    }
}

// ── FPS ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FpsMode {
    SameAsSource,
    Fixed(u32),
}

impl fmt::Display for FpsMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FpsMode::SameAsSource => write!(f, "Same as source"),
            FpsMode::Fixed(fps) => write!(f, "{fps} fps"),
        }
    }
}

// ── MovFlag ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovFlag {
    FastStart,
    FragKeyframe,
}

impl MovFlag {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            MovFlag::FastStart => "faststart",
            MovFlag::FragKeyframe => "frag_keyframe",
        }
    }
}

// ── Video Filter ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoFilter {
    HFlip,
    VFlip,
    Rotate(i32),             // degrees: 90, 180, 270
    Crop { w: u32, h: u32, x: u32, y: u32 },
    Denoise,
    Grayscale,
    Brightness(f32),         // -1.0 to 1.0
    Contrast(f32),           // -2.0 to 2.0
    Saturation(f32),         // 0.0 to 3.0
}

impl VideoFilter {
    pub fn to_ffmpeg_string(&self) -> String {
        match self {
            VideoFilter::HFlip => "hflip".into(),
            VideoFilter::VFlip => "vflip".into(),
            VideoFilter::Rotate(deg) => {
                let rad = (*deg as f64).to_radians();
                format!("rotate={rad:.6}")
            }
            VideoFilter::Crop { w, h, x, y } => format!("crop={w}:{h}:{x}:{y}"),
            VideoFilter::Denoise => "hqdn3d".into(),
            VideoFilter::Grayscale => "format=gray".into(),
            VideoFilter::Brightness(v) => format!("eq=brightness={v:.2}"),
            VideoFilter::Contrast(v) => format!("eq=contrast={v:.2}"),
            VideoFilter::Saturation(v) => format!("eq=saturation={v:.2}"),
        }
    }

    pub fn label(&self) -> String {
        match self {
            VideoFilter::HFlip => "Flip Horizontal".into(),
            VideoFilter::VFlip => "Flip Vertical".into(),
            VideoFilter::Rotate(d) => format!("Rotate {d}°"),
            VideoFilter::Crop { w, h, x, y } => format!("Crop {w}x{h} @ ({x},{y})"),
            VideoFilter::Denoise => "Denoise (hqdn3d)".into(),
            VideoFilter::Grayscale => "Grayscale".into(),
            VideoFilter::Brightness(v) => format!("Brightness {v:+.2}"),
            VideoFilter::Contrast(v) => format!("Contrast {v:+.2}"),
            VideoFilter::Saturation(v) => format!("Saturation {v:.2}"),
        }
    }
}

// ── Audio Filter ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioFilter {
    Volume(f32),             // multiplier: 0.0 to 3.0
    Loudnorm,                // EBU R128
    Highpass(u32),           // Hz
    Lowpass(u32),            // Hz
}

impl AudioFilter {
    pub fn to_ffmpeg_string(&self) -> String {
        match self {
            AudioFilter::Volume(v) => format!("volume={v:.2}"),
            AudioFilter::Loudnorm => "loudnorm=I=-16:LRA=11:TP=-1.5".into(),
            AudioFilter::Highpass(f) => format!("highpass=f={f}"),
            AudioFilter::Lowpass(f) => format!("lowpass=f={f}"),
        }
    }

    pub fn label(&self) -> String {
        match self {
            AudioFilter::Volume(v) => format!("Volume {v:.2}x"),
            AudioFilter::Loudnorm => "Loudness Norm (EBU R128)".into(),
            AudioFilter::Highpass(f) => format!("Highpass {f}Hz"),
            AudioFilter::Lowpass(f) => format!("Lowpass {f}Hz"),
        }
    }
}

// ── UI Mode ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMode {
    Simple,
    Advanced,
}

// ── Theme ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    System,
}

// ── Preset category ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetCategory {
    Video,
    Audio,
    Image,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn video_codec_count() { assert_eq!(VideoCodec::ALL.len(), 6); }
    #[test] fn video_codec_names() { assert_eq!(VideoCodec::H264.ffmpeg_name(), "libx264"); assert_eq!(VideoCodec::Copy.ffmpeg_name(), "copy"); }
    #[test] fn audio_codec_count() { assert_eq!(AudioCodec::ALL.len(), 6); }
    #[test] fn audio_codec_names() { assert_eq!(AudioCodec::Aac.ffmpeg_name(), "aac"); assert_eq!(AudioCodec::Mp3.ffmpeg_name(), "libmp3lame"); }
    #[test] fn container_count() { assert_eq!(Container::ALL.len(), 7); }
    #[test] fn container_ext() { assert_eq!(Container::Mp4.extension(), "mp4"); assert_eq!(Container::Webm.extension(), "webm"); }
    #[test] fn preset_speed_count() { assert_eq!(PresetSpeed::ALL.len(), 9); }
    #[test] fn profile_count() { assert_eq!(Profile::ALL.len(), 3); }
    #[test] fn pixel_format_count() { assert_eq!(PixelFormat::ALL.len(), 5); }
    #[test] fn scale_algo_values() { let _ = ScaleAlgorithm::Bilinear; let _ = ScaleAlgorithm::Lanczos; }
    #[test] fn fps_display() { assert_eq!(format!("{}", FpsMode::SameAsSource), "Same as source"); assert_eq!(format!("{}", FpsMode::Fixed(30)), "30 fps"); }
    #[test] fn movflag_values() { let _ = MovFlag::FastStart; let _ = MovFlag::FragKeyframe; }
    #[test] fn deinterlace_values() { let _ = DeinterlaceMethod::Yadif; let _ = DeinterlaceMethod::Bwdif; }
}
