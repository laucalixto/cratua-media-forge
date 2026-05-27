use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::enums::DeinterlaceMethod;
use crate::error::MediaForgeError;
use crate::job::{EncodeParams, ProgressInfo};

/// Detect ffmpeg binary: env var MEDIAFORGE_FFMPEG_PATH, then bundled, then PATH
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

    // Preset
    cmd.arg("-preset").arg(params.preset.ffmpeg_name());

    // Profile
    if let Some(profile) = &params.profile {
        cmd.arg("-profile:v").arg(profile.ffmpeg_name());
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
    cmd.arg("-ar").arg(params.sample_rate.to_string());

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

    // Suppress console window on Windows (child ffmpeg process)
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

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
    format!("ffmpeg {}", args.join(" "))
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
    mut on_progress: F,
) -> Result<(), MediaForgeError>
where
    F: FnMut(&ProgressInfo),
{
    let mut cmd = build_command_with_ffmpeg(params, input, output, ffmpeg_path);
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| MediaForgeError::FfmpegProcess(e.to_string()))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| MediaForgeError::FfmpegProcess("cannot capture stderr".into()))?;

    let reader = std::io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap_or_default();
        if let Some((key, value)) = parse_progress_line(&line) {
            let mut info = ProgressInfo::default();
            match key.as_str() {
                "out_time_us" => info.out_time_us = value.parse().unwrap_or(0),
                "speed" => {
                    if let Some(s) = value.strip_suffix('x') {
                        info.speed = s.parse().unwrap_or(0.0);
                    }
                }
                "fps" => info.fps = value.parse().unwrap_or(0.0),
                "frame" => info.frame = value.parse().unwrap_or(0),
                _ => {}
            }
            on_progress(&info);
        }
    }

    let status = child
        .wait()
        .map_err(|e| MediaForgeError::FfmpegProcess(e.to_string()))?;

    if !status.success() {
        return Err(MediaForgeError::FfmpegProcess(format!(
            "ffmpeg exited with code {}",
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}
