use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::enums::DeinterlaceMethod;
use crate::error::MediaForgeError;
use crate::job::{EncodeParams, ProgressInfo};

/// Detect ffprobe binary alongside ffmpeg
pub fn detect_ffmpeg() -> Option<PathBuf> {
    // 1. Env var override
    if let Ok(path) = std::env::var("MEDIAFORGE_FFMPEG_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Some(p);
        }
    }
    // 2. Bundled with the app (same directory as executable, or ./ffmpeg/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Check for ffmpeg next to the executable
            #[cfg(target_os = "windows")]
            let ffmpeg_name = "ffmpeg.exe";
            #[cfg(not(target_os = "windows"))]
            let ffmpeg_name = "ffmpeg";

            let bundled = exe_dir.join(ffmpeg_name);
            if bundled.exists() {
                return Some(bundled);
            }
            let bundled_sub = exe_dir.join("ffmpeg").join(ffmpeg_name);
            if bundled_sub.exists() {
                return Some(bundled_sub);
            }
        }
    }
    // 3. System PATH
    which::which("ffmpeg").ok()
}

/// Build a std::process::Command for ffmpeg from EncodeParams
/// If ffmpeg_path is Some, uses that; otherwise uses "ffmpeg" from PATH
pub fn build_command(params: &EncodeParams, input: &Path, output: &Path) -> Command {
    build_command_with_ffmpeg(params, input, output, None)
}

pub fn build_command_with_ffmpeg(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    ffmpeg_path: Option<&Path>,
) -> Command {
    let ffmpeg = ffmpeg_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let mut cmd = Command::new(&ffmpeg);

    cmd.arg("-y") // overwrite without prompting
        .arg("-i")
        .arg(input);

    // ── GIF: special handling (no video codec, palette filter, no audio) ──
    if params.container == crate::enums::Container::Gif {
        let fps = match params.fps {
            crate::enums::FpsMode::Fixed(f) => f,
            _ => 10,
        };
        let mut vf = format!(
            "fps={},scale={}:{}:flags={},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            fps, params.width, params.height, params.scale_algorithm.ffmpeg_flag()
        );
        for f in &params.video_filters {
            vf.push_str(&format!(",{}", f.to_ffmpeg_string()));
        }
        cmd.arg("-vf").arg(vf);
        cmd.arg("-progress").arg("pipe:2").arg("-nostats");
        cmd.arg(output);
        return cmd;
    }

    // Trim
    if let Some(ss) = &params.trim_start {
        cmd.arg("-ss").arg(ss);
    }
    if let Some(to) = &params.trim_end {
        cmd.arg("-to").arg(to);
    }

    // Video codec
    cmd.arg("-c:v").arg(params.video_codec.ffmpeg_name());

    // CRF or bitrate
    if let Some(crf) = params.crf {
        cmd.arg("-crf").arg(crf.to_string());
    }
    if let Some(bitrate) = params.video_bitrate {
        cmd.arg("-b:v").arg(format!("{}k", bitrate));
    }
    if let Some(maxrate) = params.max_bitrate {
        cmd.arg("-maxrate").arg(format!("{}k", maxrate));
    }
    if let Some(bufsize) = params.bufsize {
        cmd.arg("-bufsize").arg(format!("{}k", bufsize));
    }

    // Preset (codec-dependent: VP9/AV1 use -cpu-used + -deadline, not -preset)
    match params.video_codec {
        crate::enums::VideoCodec::VP9 => {
            cmd.arg("-deadline").arg("good");
            cmd.arg("-cpu-used").arg(params.preset.vp9_cpu_used().to_string());
        }
        crate::enums::VideoCodec::AV1 => {
            cmd.arg("-cpu-used").arg(params.preset.av1_cpu_used().to_string());
        }
        crate::enums::VideoCodec::SVTAV1 => {
            cmd.arg("-preset").arg(params.preset.svtav1_preset().to_string());
        }
        _ => {
            cmd.arg("-preset").arg(params.preset.ffmpeg_name());
        }
    }

    // Profile (skip for VP9/AV1/SVTAV1 — not supported)
    if let Some(profile) = &params.profile {
        use crate::enums::VideoCodec;
        if !matches!(params.video_codec, VideoCodec::VP9 | VideoCodec::AV1 | VideoCodec::SVTAV1) {
            cmd.arg("-profile:v").arg(profile.ffmpeg_name());
        }
    }

    // Video filters
    let mut vf_parts: Vec<String> = Vec::new();

    if let Some(deint) = &params.deinterlace {
        vf_parts.push(match deint {
            DeinterlaceMethod::Yadif => "yadif".to_string(),
            DeinterlaceMethod::Bwdif => "bwdif".to_string(),
        });
    }

    vf_parts.push(format!(
        "scale={}:{}:flags={}",
        params.width,
        params.height,
        params.scale_algorithm.ffmpeg_flag()
    ));

    for filter in &params.video_filters {
        vf_parts.push(filter.to_ffmpeg_string());
    }

    if !vf_parts.is_empty() {
        cmd.arg("-vf").arg(vf_parts.join(","));
    }

    // Pixel format
    cmd.arg("-pix_fmt").arg(params.pixel_format.ffmpeg_name());

    // FPS
    if let crate::enums::FpsMode::Fixed(fps) = params.fps {
        cmd.arg("-r").arg(fps.to_string());
    }

    // Audio codec
    cmd.arg("-c:a").arg(params.audio_codec.ffmpeg_name());
    cmd.arg("-b:a").arg(format!("{}k", params.audio_bitrate));
    cmd.arg("-ac").arg(params.audio_channels.to_string());

    // Sample rate — Opus only supports 48000, 24000, 16000, 12000, 8000 Hz
    let sample_rate = if params.audio_codec == crate::enums::AudioCodec::Opus
        && ![48000, 24000, 16000, 12000, 8000].contains(&params.sample_rate)
    {
        log::warn!(
            "Opus does not support {} Hz sample rate; falling back to 48000 Hz",
            params.sample_rate
        );
        48000
    } else {
        params.sample_rate
    };
    cmd.arg("-ar").arg(sample_rate.to_string());

    // Audio filters
    if !params.audio_filters.is_empty() {
        let af: Vec<String> = params
            .audio_filters
            .iter()
            .map(|f| f.to_ffmpeg_string())
            .collect();
        cmd.arg("-af").arg(af.join(","));
    }

    // Movflags
    if !params.movflags.is_empty() {
        let flags: Vec<&str> = params
            .movflags
            .iter()
            .map(|f| f.ffmpeg_name())
            .collect();
        cmd.arg("-movflags").arg(flags.join("+"));
    }

    // Threads
    if params.threads > 0 {
        cmd.arg("-threads").arg(params.threads.to_string());
    }

    // Metadata
    for (key, value) in &params.metadata {
        cmd.arg("-metadata").arg(format!("{key}={value}"));
    }

    // Extra args
    for arg in &params.extra_args {
        cmd.arg(arg);
    }

    // Progress to stderr
    cmd.arg("-progress").arg("pipe:2");
    cmd.arg("-nostats");

    cmd.arg(output);

    cmd
}

/// Build the ffmpeg command as a displayable string
pub fn command_to_string(params: &EncodeParams, input: &Path, output: &Path) -> String {
    command_to_string_with_ffmpeg(params, input, output, None)
}

pub fn command_to_string_with_ffmpeg(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    ffmpeg_path: Option<&Path>,
) -> String {
    let cmd = build_command_with_ffmpeg(params, input, output, ffmpeg_path);
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| {
            let s = a.to_string_lossy();
            if s.contains(' ') {
                format!("\"{s}\"")
            } else {
                s.to_string()
            }
        })
        .collect();
    format!(
        "{} {}",
        ffmpeg_path.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "ffmpeg".into()),
        args.join(" ")
    )
}

/// Parse a single progress line from ffmpeg stderr
pub fn parse_progress_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, '=');
    let key = parts.next()?.trim().to_string();
    let value = parts.next()?.trim().to_string();
    Some((key, value))
}

/// Parse ffmpeg stderr to extract progress info
pub fn parse_progress_output(stderr: impl BufRead) -> ProgressInfo {
    let mut info = ProgressInfo::default();
    let mut duration_us: u64 = 0;

    for line in stderr.lines().map_while(Result::ok) {
        if let Some((key, value)) = parse_progress_line(&line) {
            match key.as_str() {
                "frame" => {
                    info.frame = value.parse().unwrap_or(0);
                }
                "fps" => {
                    info.fps = value.parse().unwrap_or(0.0);
                }
                "bitrate" => {
                    info.bitrate = value;
                }
                "total_size" => {
                    info.total_size = value.parse().unwrap_or(0);
                }
                "out_time_us" => {
                    info.out_time_us = value.parse().unwrap_or(0);
                    duration_us = info.out_time_us;
                }
                "speed" => {
                    if let Some(s) = value.strip_suffix('x') {
                        info.speed = s.parse().unwrap_or(0.0);
                    }
                }
                "out_time" => {
                    // ffmpeg sometimes outputs "out_time=00:00:05.000000"
                    // We also get out_time_us which is more reliable
                    info.progress_pct = 0.0; // fallback to out_time_us below
                }
                _ => {}
            }
        }
    }

    // The progress percentage is tricky without knowing total duration in advance.
    // We estimate from speed × time or just report that we're running.
    // For now, out_time_us is the position — callers should combine with source duration.
    info.out_time_us = duration_us;

    info
}

/// Probe input file duration in microseconds using ffprobe (bundled alongside ffmpeg).
pub fn probe_duration(input: &Path, ffmpeg_path: Option<&Path>) -> Option<u64> {
    // Find ffprobe: same directory as ffmpeg, or detect
    let ffprobe = ffmpeg_path.map_or_else(
        || {
            detect_ffmpeg().map(|ff| {
                let mut pb = ff;
                pb.set_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
                pb
            })
        },
        |p| {
            let mut pb = p.to_path_buf();
            pb.set_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
            Some(pb)
        },
    );

    let ffprobe = ffprobe?;
    if !ffprobe.exists() {
        // Fallback: try PATH
        let name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
        if which::which(name).is_err() {
            return None;
        }
    }

    let mut cmd = Command::new(&ffprobe);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.args([
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
        ])
        .arg(input);

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let secs: f64 = stdout.trim().parse().ok()?;
    Some((secs * 1_000_000.0) as u64)
}

/// Run ffmpeg with progress reporting via callback
pub fn run_with_progress<F>(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    on_progress: F,
) -> Result<(), MediaForgeError>
where
    F: FnMut(&ProgressInfo),
{
    run_with_progress_and_ffmpeg(params, input, output, None, on_progress)
}

pub fn run_with_progress_and_ffmpeg<F>(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    ffmpeg_path: Option<&Path>,
    on_progress: F,
) -> Result<(), MediaForgeError>
where
    F: FnMut(&ProgressInfo),
{
    run_with_progress_and_ffmpeg_cancellable(params, input, output, ffmpeg_path, None, None, on_progress)
        .map(|_| ())
}

/// Run ffmpeg with optional cancellation via Arc<AtomicBool>.
/// When the flag is set to true, the child process is killed and Err(Cancelled) is returned.
pub fn run_with_progress_and_ffmpeg_cancellable<F>(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    ffmpeg_path: Option<&Path>,
    cancel_flag: Option<Arc<AtomicBool>>,
    total_duration_us: Option<u64>,
    mut on_progress: F,
) -> Result<String, MediaForgeError>
where
    F: FnMut(&ProgressInfo),
{
    // Auto-detect ffmpeg if not explicitly provided
    let resolved_path = match ffmpeg_path {
        Some(p) => Some(p.to_path_buf()),
        None => detect_ffmpeg(),
    };
    let resolved = resolved_path.as_deref();

    let mut cmd = build_command_with_ffmpeg(params, input, output, resolved);
    let cmd_str = command_to_string_with_ffmpeg(params, input, output, resolved);
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| MediaForgeError::FfmpegProcess(format!("spawn failed: {e}")))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| MediaForgeError::FfmpegProcess("cannot capture stderr".into()))?;

    let reader = std::io::BufReader::new(stderr);
    let mut stderr_lines: Vec<String> = Vec::new();

    for line in reader.lines() {
        // Check cancellation
        if let Some(ref flag) = cancel_flag {
            if flag.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(MediaForgeError::Cancelled);
            }
        }

        let line = line.unwrap_or_default();
        let line_clone = line.clone();
        stderr_lines.push(line_clone);

        if let Some((key, value)) = parse_progress_line(&line) {
            let mut info = ProgressInfo::default();
            let mut should_emit = true;
            match key.as_str() {
                "out_time_us" => {
                    info.out_time_us = value.parse().unwrap_or(0);
                    if let Some(total) = total_duration_us {
                        if total > 0 {
                            info.progress_pct = (info.out_time_us as f64 / total as f64 * 100.0).min(99.9);
                        }
                    }
                }
                "progress" => {
                    if value == "end" {
                        info.progress_pct = 100.0;
                    } else {
                        should_emit = false; // progress=continue — don't emit
                    }
                }
                // Only emit for out_time_us and progress=end — all other keys
                // (speed, fps, frame, bitrate, total_size) carry progress_pct=0.0
                // which would overwrite the real value in the frontend.
                _ => should_emit = false,
            }
            if should_emit {
                on_progress(&info);
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| MediaForgeError::FfmpegProcess(e.to_string()))?;

    if !status.success() {
        let stderr_tail: String = stderr_lines
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(MediaForgeError::FfmpegProcess(format!(
            "ffmpeg exited with code {}\nCommand: {}\nStderr (last 20 lines):\n{}",
            status.code().unwrap_or(-1),
            cmd_str,
            stderr_tail,
        )));
    }

    Ok(cmd_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::*;
    use crate::job::EncodeParams;
    use std::path::Path;

    fn cmd_str(p: &EncodeParams) -> String {
        let cmd = build_command(p, Path::new("in.mp4"), Path::new("out.mp4"));
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        args.join(" ")
    }

    #[test]
    fn codec_h264() { let p = EncodeParams { video_codec: VideoCodec::H264, ..Default::default() }; assert!(cmd_str(&p).contains("libx264")); }
    #[test]
    fn codec_h265() { let p = EncodeParams { video_codec: VideoCodec::H265, ..Default::default() }; assert!(cmd_str(&p).contains("libx265")); }
    #[test]
    fn codec_vp9() { let p = EncodeParams { video_codec: VideoCodec::VP9, ..Default::default() }; assert!(cmd_str(&p).contains("libvpx-vp9")); }
    #[test]
    fn codec_av1() { let p = EncodeParams { video_codec: VideoCodec::AV1, ..Default::default() }; assert!(cmd_str(&p).contains("libaom-av1")); }
    #[test]
    fn codec_svtav1() { let p = EncodeParams { video_codec: VideoCodec::SVTAV1, ..Default::default() }; assert!(cmd_str(&p).contains("libsvtav1")); }
    #[test]
    fn codec_copy() { let p = EncodeParams { video_codec: VideoCodec::Copy, ..Default::default() }; assert!(cmd_str(&p).contains("-c:v")); assert!(cmd_str(&p).contains("copy")); }
    #[test]
    fn crf_present() { let p = EncodeParams { crf: Some(23), ..Default::default() }; assert!(cmd_str(&p).contains("-crf 23")); }
    #[test]
    fn crf_none() { let p = EncodeParams { crf: None, ..Default::default() }; assert!(!cmd_str(&p).contains("-crf")); }
    #[test]
    fn bitrate_present() { let p = EncodeParams { video_bitrate: Some(5000), ..Default::default() }; assert!(cmd_str(&p).contains("-b:v 5000k")); }
    #[test]
    fn crf_and_bitrate_both() { let p = EncodeParams { crf: Some(20), video_bitrate: Some(3000), ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("-crf 20")); assert!(s.contains("-b:v 3000k")); }
    #[test]
    fn scale_in_vf() { let p = EncodeParams { width: 640, height: 360, ..Default::default() }; assert!(cmd_str(&p).contains("scale=640:360")); }
    #[test]
    fn deinterlace_yadif_in_vf() { let p = EncodeParams { deinterlace: Some(DeinterlaceMethod::Yadif), ..Default::default() }; assert!(cmd_str(&p).contains("yadif")); }
    #[test]
    fn deinterlace_bwdif_in_vf() { let p = EncodeParams { deinterlace: Some(DeinterlaceMethod::Bwdif), ..Default::default() }; assert!(cmd_str(&p).contains("bwdif")); }
    #[test]
    fn no_deinterlace_when_none() { let p = EncodeParams { deinterlace: None, ..Default::default() }; let s = cmd_str(&p); assert!(!s.contains("yadif")); assert!(!s.contains("bwdif")); }
    #[test]
    fn pix_fmt_yuv420p() { let p = EncodeParams { pixel_format: PixelFormat::Yuv420p, ..Default::default() }; assert!(cmd_str(&p).contains("yuv420p")); }
    #[test]
    fn pix_fmt_yuv444p() { let p = EncodeParams { pixel_format: PixelFormat::Yuv444p, ..Default::default() }; assert!(cmd_str(&p).contains("yuv444p")); }
    #[test]
    fn pix_fmt_nv12() { let p = EncodeParams { pixel_format: PixelFormat::Nv12, ..Default::default() }; assert!(cmd_str(&p).contains("nv12")); }
    #[test]
    fn pix_fmt_rgb24() { let p = EncodeParams { pixel_format: PixelFormat::Rgb24, ..Default::default() }; assert!(cmd_str(&p).contains("rgb24")); }
    #[test]
    fn preset_slow() { let p = EncodeParams { preset: PresetSpeed::Slow, ..Default::default() }; assert!(cmd_str(&p).contains("-preset slow")); }
    #[test]
    fn preset_ultrafast() { let p = EncodeParams { preset: PresetSpeed::Ultrafast, ..Default::default() }; assert!(cmd_str(&p).contains("-preset ultrafast")); }
    #[test]
    fn preset_superfast() { let p = EncodeParams { preset: PresetSpeed::Superfast, ..Default::default() }; assert!(cmd_str(&p).contains("-preset superfast")); }
    #[test]
    fn preset_veryslow() { let p = EncodeParams { preset: PresetSpeed::Veryslow, ..Default::default() }; assert!(cmd_str(&p).contains("-preset veryslow")); }
    #[test]
    fn profile_baseline() { let p = EncodeParams { profile: Some(Profile::Baseline), ..Default::default() }; assert!(cmd_str(&p).contains("-profile:v baseline")); }
    #[test]
    fn profile_main() { let p = EncodeParams { profile: Some(Profile::Main), ..Default::default() }; assert!(cmd_str(&p).contains("-profile:v main")); }
    #[test]
    fn profile_high() { let p = EncodeParams { profile: Some(Profile::High), ..Default::default() }; assert!(cmd_str(&p).contains("-profile:v high")); }
    #[test]
    fn profile_none() { let p = EncodeParams { profile: None, ..Default::default() }; assert!(!cmd_str(&p).contains("-profile:v")); }
    #[test]
    fn fps_fixed_30() { let p = EncodeParams { fps: FpsMode::Fixed(30), ..Default::default() }; assert!(cmd_str(&p).contains("-r 30")); }
    #[test]
    fn fps_fixed_60() { let p = EncodeParams { fps: FpsMode::Fixed(60), ..Default::default() }; assert!(cmd_str(&p).contains("-r 60")); }
    #[test]
    fn fps_same_no_r() { let p = EncodeParams { fps: FpsMode::SameAsSource, ..Default::default() }; assert!(!cmd_str(&p).contains(" -r ")); }
    #[test]
    fn scale_algorithm_bilinear() { let p = EncodeParams { scale_algorithm: ScaleAlgorithm::Bilinear, ..Default::default() }; assert!(cmd_str(&p).contains("bilinear")); }
    #[test]
    fn scale_algorithm_bicubic() { let p = EncodeParams { scale_algorithm: ScaleAlgorithm::Bicubic, ..Default::default() }; assert!(cmd_str(&p).contains("bicubic")); }
    #[test]
    fn threads_4() { let p = EncodeParams { threads: 4, ..Default::default() }; assert!(cmd_str(&p).contains("-threads 4")); }
    #[test]
    fn threads_0_auto() { let p = EncodeParams { threads: 0, ..Default::default() }; assert!(!cmd_str(&p).contains("-threads")); }
    #[test]
    fn trim_flags() { let p = EncodeParams { trim_start: Some("00:01:00".into()), trim_end: Some("00:05:00".into()), ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("-ss 00:01:00")); assert!(s.contains("-to 00:05:00")); }
    #[test]
    fn trim_start_only() { let p = EncodeParams { trim_start: Some("00:00:10".into()), trim_end: None, ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("-ss 00:00:10")); assert!(!s.contains("-to")); }
    #[test]
    fn trim_end_only() { let p = EncodeParams { trim_start: None, trim_end: Some("00:01:00".into()), ..Default::default() }; let s = cmd_str(&p); assert!(!s.contains("-ss")); assert!(s.contains("-to 00:01:00")); }
    #[test]
    fn audio_codec_aac() { let p = EncodeParams { audio_codec: AudioCodec::Aac, ..Default::default() }; assert!(cmd_str(&p).contains("-c:a aac")); }
    #[test]
    fn audio_codec_mp3() { let p = EncodeParams { audio_codec: AudioCodec::Mp3, ..Default::default() }; assert!(cmd_str(&p).contains("-c:a libmp3lame")); }
    #[test]
    fn audio_codec_opus() { let p = EncodeParams { audio_codec: AudioCodec::Opus, ..Default::default() }; assert!(cmd_str(&p).contains("libopus")); }
    #[test]
    fn audio_codec_vorbis() { let p = EncodeParams { audio_codec: AudioCodec::Vorbis, ..Default::default() }; assert!(cmd_str(&p).contains("libvorbis")); }
    #[test]
    fn audio_codec_flac() { let p = EncodeParams { audio_codec: AudioCodec::Flac, ..Default::default() }; assert!(cmd_str(&p).contains("-c:a flac")); }
    #[test]
    fn audio_codec_copy() { let p = EncodeParams { audio_codec: AudioCodec::Copy, ..Default::default() }; assert!(cmd_str(&p).contains("-c:a copy")); }
    #[test]
    fn audio_bitrate_192() { let p = EncodeParams { audio_bitrate: 192, ..Default::default() }; assert!(cmd_str(&p).contains("-b:a 192k")); }
    #[test]
    fn audio_channels() { let p = EncodeParams { audio_channels: 2, ..Default::default() }; assert!(cmd_str(&p).contains("-ac 2")); }
    #[test]
    fn audio_channels_1() { let p = EncodeParams { audio_channels: 1, ..Default::default() }; assert!(cmd_str(&p).contains("-ac 1")); }
    #[test]
    fn sample_rate_48k() { let p = EncodeParams { sample_rate: 48000, ..Default::default() }; assert!(cmd_str(&p).contains("-ar 48000")); }
    #[test]
    fn sample_rate_44100() { let p = EncodeParams { sample_rate: 44100, ..Default::default() }; assert!(cmd_str(&p).contains("-ar 44100")); }
    #[test]
    fn movflags_faststart() { let p = EncodeParams { movflags: vec![MovFlag::FastStart], ..Default::default() }; assert!(cmd_str(&p).contains("-movflags faststart")); }
    #[test]
    fn movflags_both() { let p = EncodeParams { movflags: vec![MovFlag::FastStart, MovFlag::FragKeyframe], ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("faststart")); assert!(s.contains("frag_keyframe")); }
    #[test]
    fn movflags_empty() { let p = EncodeParams { movflags: vec![], ..Default::default() }; assert!(!cmd_str(&p).contains("-movflags")); }
    #[test]
    fn metadata_present() { let mut p = EncodeParams::default(); p.metadata.insert("title".into(), "Test".into()); assert!(cmd_str(&p).contains("-metadata title=Test")); }
    #[test]
    fn metadata_multiple_keys() { let mut p = EncodeParams::default(); p.metadata.insert("title".into(), "Test".into()); p.metadata.insert("artist".into(), "Artist".into()); let s = cmd_str(&p); assert!(s.contains("-metadata title=Test")); assert!(s.contains("-metadata artist=Artist")); }
    #[test]
    fn container_mp4_output() { let p = EncodeParams { container: Container::Mp4, ..Default::default() }; assert!(cmd_str(&p).ends_with("out.mp4")); }
    #[test]
    fn container_webm_present() { let p = EncodeParams { container: Container::Webm, ..Default::default() }; let s = cmd_str(&p); assert!(s.ends_with("out.mp4")); }
    #[test]
    fn max_bitrate_and_bufsize() { let p = EncodeParams { max_bitrate: Some(8000), bufsize: Some(16000), ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("-maxrate 8000k")); assert!(s.contains("-bufsize 16000k")); }
    #[test]
    fn max_bitrate_none() { let p = EncodeParams { max_bitrate: None, ..Default::default() }; assert!(!cmd_str(&p).contains("-maxrate")); }
    #[test]
    fn bufsize_none() { let p = EncodeParams { bufsize: None, ..Default::default() }; assert!(!cmd_str(&p).contains("-bufsize")); }
    #[test]
    fn extra_args_present() { let p = EncodeParams { extra_args: vec!["-an".into()], ..Default::default() }; assert!(cmd_str(&p).contains("-an")); }
    #[test]
    fn video_filter_hflip() { let p = EncodeParams { video_filters: vec![VideoFilter::HFlip], ..Default::default() }; assert!(cmd_str(&p).contains("hflip")); }
    #[test]
    fn video_filter_vflip() { let p = EncodeParams { video_filters: vec![VideoFilter::VFlip], ..Default::default() }; assert!(cmd_str(&p).contains("vflip")); }
    #[test]
    fn video_filter_rotate_90() { let p = EncodeParams { video_filters: vec![VideoFilter::Rotate(90)], ..Default::default() }; assert!(cmd_str(&p).contains("rotate=")); }
    #[test]
    fn video_filter_denoise() { let p = EncodeParams { video_filters: vec![VideoFilter::Denoise], ..Default::default() }; assert!(cmd_str(&p).contains("hqdn3d")); }
    #[test]
    fn video_filter_grayscale() { let p = EncodeParams { video_filters: vec![VideoFilter::Grayscale], ..Default::default() }; assert!(cmd_str(&p).contains("format=gray")); }
    #[test]
    fn video_filters_multiple() { let p = EncodeParams { video_filters: vec![VideoFilter::HFlip, VideoFilter::Denoise], ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("hflip")); assert!(s.contains("hqdn3d")); }
    #[test]
    fn no_video_filters_when_empty() { let p = EncodeParams { video_filters: vec![], deinterlace: None, ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("scale=")); assert!(!s.contains("hflip")); }
    #[test]
    fn audio_filter_volume() { let p = EncodeParams { audio_filters: vec![AudioFilter::Volume(2.0)], ..Default::default() }; assert!(cmd_str(&p).contains("volume=2.00")); }
    #[test]
    fn audio_filter_loudnorm() { let p = EncodeParams { audio_filters: vec![AudioFilter::Loudnorm], ..Default::default() }; assert!(cmd_str(&p).contains("loudnorm")); }
    #[test]
    fn audio_filter_highpass() { let p = EncodeParams { audio_filters: vec![AudioFilter::Highpass(100)], ..Default::default() }; assert!(cmd_str(&p).contains("highpass=f=100")); }
    #[test]
    fn audio_filter_lowpass() { let p = EncodeParams { audio_filters: vec![AudioFilter::Lowpass(3000)], ..Default::default() }; assert!(cmd_str(&p).contains("lowpass=f=3000")); }
    #[test]
    fn audio_filters_multiple() { let p = EncodeParams { audio_filters: vec![AudioFilter::Volume(2.0), AudioFilter::Loudnorm], ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("volume=2.00")); assert!(s.contains("loudnorm")); }
    #[test]
    fn no_audio_filters_when_empty() { let p = EncodeParams { audio_filters: vec![], ..Default::default() }; assert!(!cmd_str(&p).contains("-af")); }
    #[test]
    fn no_overwrite_flag() { let p = EncodeParams::default(); assert!(cmd_str(&p).contains("-y")); }
    #[test]
    fn progress_pipe_stderr() { let p = EncodeParams::default(); assert!(cmd_str(&p).contains("-progress pipe:2")); }
    #[test]
    fn command_to_string_format() { let p = EncodeParams::default(); let s = command_to_string(&p, Path::new("in.mp4"), Path::new("out.mp4")); assert!(s.starts_with("ffmpeg")); assert!(s.contains("-i")); }
    #[test]
    fn ffmpeg_path_custom_in_command_string() { let s = command_to_string_with_ffmpeg(&EncodeParams::default(), Path::new("in.mp4"), Path::new("out.mp4"), Some(Path::new("/custom/ffmpeg"))); assert!(s.starts_with("/custom/ffmpeg")); }
    #[test]
    fn parse_progress_valid() { let r = parse_progress_line("out_time_us=123456"); assert!(r.is_some()); let (k, v) = r.unwrap(); assert_eq!(k, "out_time_us"); assert_eq!(v, "123456"); }
    #[test]
    fn parse_progress_invalid() { assert!(parse_progress_line("not a progress line").is_none()); }
    #[test]
    fn parse_progress_empty() { let data: &[u8] = b""; let info = parse_progress_output(&data[..]); assert_eq!(info.frame, 0); assert_eq!(info.fps, 0.0); assert_eq!(info.out_time_us, 0); }
    #[test]
    fn parse_progress_output_fields() {
        let data = b"frame=100\nfps=25.0\nbitrate=1200.5kbits/s\ntotal_size=500000\nout_time_us=4000000\nspeed=2.5x\nprogress=continue\n";
        let info = parse_progress_output(&data[..]);
        assert_eq!(info.frame, 100);
        assert_eq!(info.fps, 25.0);
        assert!(info.bitrate.contains("1200.5"));
        assert_eq!(info.total_size, 500000);
        assert_eq!(info.out_time_us, 4000000);
        assert_eq!(info.speed, 2.5);
    }
    #[test]
    fn build_command_with_ffmpeg_path() {
        let cmd = build_command_with_ffmpeg(&EncodeParams::default(), Path::new("in.mp4"), Path::new("out.mp4"), Some(Path::new("/my/ffmpeg")));
        let prog = cmd.get_program().to_string_lossy();
        assert_eq!(prog, "/my/ffmpeg");
    }
    #[test]
    fn probe_duration_requires_input() { assert!(probe_duration(Path::new("nonexistent.mp4"), None).is_none()); }
    #[test]
    fn gif_no_video_codec() { let p = EncodeParams { container: Container::Gif, fps: FpsMode::Fixed(10), width: 480, height: 270, ..Default::default() }; let s = cmd_str(&p); assert!(!s.contains("-c:v"), "GIF: {s}"); assert!(s.contains("palettegen")); assert!(!s.contains("-c:a")); }
    #[test]
    fn webm_vp9_uses_deadline() { let p = EncodeParams { video_codec: VideoCodec::VP9, container: Container::Webm, ..Default::default() }; let s = cmd_str(&p); assert!(s.contains("-deadline good")); assert!(s.contains("-cpu-used")); assert!(!s.contains("-preset ")); }
    #[test]
    fn webm_no_profile() { let p = EncodeParams { video_codec: VideoCodec::VP9, profile: Some(Profile::High), container: Container::Webm, ..Default::default() }; assert!(!cmd_str(&p).contains("-profile:v")); }

    // ── TDD: progress event filtering ──
    // RED: This test documents the expected behavior — only out_time_us and
    // progress=end should trigger callbacks. Lines like speed/fps/frame/bitrate
    // must NOT overwrite the progress_pct in the frontend.

    /// Simulates the stderr-reading loop from run_with_progress_and_ffmpeg_cancellable.
    /// Returns (call_count, progress_pct_values_received).
    fn simulate_progress_events(lines: &[&str], total_duration_us: Option<u64>) -> (usize, Vec<f64>) {
        let mut call_count = 0;
        let mut pct_values: Vec<f64> = Vec::new();
        let mut on_progress = |info: &ProgressInfo| {
            call_count += 1;
            pct_values.push(info.progress_pct);
        };
        for &line in lines {
            if let Some((key, value)) = parse_progress_line(line) {
                let mut info = ProgressInfo::default();
                let mut should_emit = true;
                match key.as_str() {
                    "out_time_us" => {
                        info.out_time_us = value.parse().unwrap_or(0);
                        if let Some(total) = total_duration_us {
                            if total > 0 {
                                info.progress_pct = (info.out_time_us as f64 / total as f64 * 100.0).min(99.9);
                            }
                        }
                    }
                    "progress" => {
                        if value == "end" {
                            info.progress_pct = 100.0;
                        } else {
                            should_emit = false;
                        }
                    }
                    _ => should_emit = false,
                }
                if should_emit {
                    on_progress(&info);
                }
            }
        }
        (call_count, pct_values)
    }

    #[test]
    fn progress_filter_only_emits_out_time_and_end() {
        // Simulate a 10-second video (10_000_000 us)
        let total_us = Some(10_000_000u64);
        let lines = [
            "frame=0",
            "fps=0.0",
            "out_time_us=0",
            "speed=0.0x",
            "frame=100",
            "fps=30.0",
            "out_time_us=2500000",     // 25%
            "speed=3.5x",
            "frame=200",
            "bitrate=1200.5kbits/s",
            "out_time_us=5000000",     // 50%
            "total_size=500000",
            "frame=300",
            "out_time_us=7500000",     // 75%
            "speed=2.0x",
            "progress=continue",
            "out_time_us=9990000",     // 99.9% (capped)
            "progress=end",            // 100%
        ];
        let (count, pcts) = simulate_progress_events(&lines, total_us);

        // ── Assertions ──
        // Only out_time_us and progress=end lines should trigger callbacks
        assert_eq!(count, 6, "expected 6 callbacks (5 out_time_us + 1 progress=end)");

        // Verify the progression: 0% → 25% → 50% → 75% → 99.9% → 100%
        assert_eq!(pcts[0], 0.0,  "first: 0%");
        assert_eq!(pcts[1], 25.0, "second: 25%");
        assert_eq!(pcts[2], 50.0, "third: 50%");
        assert_eq!(pcts[3], 75.0, "fourth: 75%");
        assert_eq!(pcts[4], 99.9, "fifth: ~99.9% (capped)");
        assert_eq!(pcts[5], 100.0, "sixth: 100% (end)");
    }

    #[test]
    fn progress_filter_no_zero_overwrite_from_frame_lines() {
        // frame/speed/fps/bitrate lines must NOT emit — they carry progress_pct=0
        let total_us = Some(10_000_000u64);
        let lines = [
            "out_time_us=5000000",   // 50% — emitted
            "frame=123",             // NOT emitted
            "fps=25.0",              // NOT emitted
            "speed=2.5x",            // NOT emitted
            "bitrate=1200k",         // NOT emitted
            "total_size=99999",      // NOT emitted
            "out_time_us=8000000",   // 80% — emitted
            "progress=end",          // 100% — emitted
        ];
        let (count, pcts) = simulate_progress_events(&lines, total_us);

        assert_eq!(count, 3, "only 3 callbacks: 2 out_time_us + 1 progress=end");
        assert_eq!(pcts, vec![50.0, 80.0, 100.0], "no zero-overwrites from frame/speed/fps lines");
    }

    #[test]
    fn progress_filter_no_duration_still_emits_end() {
        // Without probe duration, progress_pct stays 0.0 but progress=end still fires
        let lines = [
            "out_time_us=1000000",
            "out_time_us=5000000",
            "progress=end",
        ];
        let (count, pcts) = simulate_progress_events(&lines, None);
        assert_eq!(count, 3);
        // No total_duration → progress_pct stays 0.0 until progress=end
        assert_eq!(pcts[0], 0.0);
        assert_eq!(pcts[1], 0.0);
        assert_eq!(pcts[2], 100.0);
    }
}