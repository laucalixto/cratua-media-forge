use crate::enums::{
    AudioCodec, Container, DeinterlaceMethod, MovFlag, PixelFormat, PresetCategory,
    PresetSpeed, VideoCodec,
};
use crate::job::{EncodeParams, Preset};

/// Built-in presets shipped with MediaForge
pub fn builtin_presets() -> Vec<Preset> {
    vec![
        // ── Legacy: exact replica of the original .bat ──
        Preset {
            id: "default".into(),
            name: "Padrão (H.264 CRF 19)".into(),
            description: "Configuração padrão: H.264, yadif deinterlace, faststart, CRF 19, 128k audio".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::H264,
                width: 1920,
                height: 1080,
                crf: Some(19),
                deinterlace: Some(DeinterlaceMethod::Yadif),
                pixel_format: PixelFormat::Yuv420p,
                movflags: vec![MovFlag::FastStart],
                threads: 2,
                preset: PresetSpeed::Medium,
                audio_bitrate: 128,
                ..Default::default()
            },
        },
        // ── Web Video ──
        Preset {
            id: "web-h264".into(),
            name: "Web Video (H.264)".into(),
            description: "Optimized for web streaming: H.264, yuv420p, faststart, CRF 23".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::H264,
                crf: Some(23),
                pixel_format: PixelFormat::Yuv420p,
                movflags: vec![MovFlag::FastStart],
                threads: 0,
                ..Default::default()
            },
        },
        Preset {
            id: "web-h265".into(),
            name: "Web Video (H.265)".into(),
            description: "Modern HEVC for web: H.265, yuv420p, faststart, CRF 28".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::H265,
                crf: Some(28),
                pixel_format: PixelFormat::Yuv420p,
                movflags: vec![MovFlag::FastStart],
                threads: 0,
                ..Default::default()
            },
        },
        Preset {
            id: "web-vp9".into(),
            name: "Web Video (VP9)".into(),
            description: "Royalty-free VP9 for WebM: CRF 30, best for YouTube/Firefox".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::VP9,
                container: Container::Webm,
                crf: Some(30),
                pixel_format: PixelFormat::Yuv420p,
                threads: 0,
                ..Default::default()
            },
        },
        // ── High Quality ──
        Preset {
            id: "archive-h264".into(),
            name: "Archive (H.264 HQ)".into(),
            description: "High quality archival: CRF 16, slow preset, no filtering".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::H264,
                crf: Some(16),
                preset: PresetSpeed::Slow,
                pixel_format: PixelFormat::Yuv420p,
                threads: 0,
                ..Default::default()
            },
        },
        // ── Audio ──
        Preset {
            id: "audio-mp3".into(),
            name: "Audio MP3 (192k)".into(),
            description: "Extract/convert to MP3: 192kbps, 44.1kHz, stereo".into(),
            category: PresetCategory::Audio,
            params: EncodeParams {
                video_codec: VideoCodec::Copy,
                audio_codec: AudioCodec::Mp3,
                container: Container::Mp3,
                audio_bitrate: 192,
                sample_rate: 44100,
                threads: 0,
                ..Default::default()
            },
        },
        Preset {
            id: "audio-aac".into(),
            name: "Audio AAC (128k)".into(),
            description: "Extract/convert to AAC: 128kbps, 44.1kHz, stereo".into(),
            category: PresetCategory::Audio,
            params: EncodeParams {
                video_codec: VideoCodec::Copy,
                audio_codec: AudioCodec::Aac,
                container: Container::Mp4,
                audio_bitrate: 128,
                sample_rate: 44100,
                threads: 0,
                ..Default::default()
            },
        },
        Preset {
            id: "audio-opus".into(),
            name: "Audio Opus (96k)".into(),
            description: "High-efficiency Opus: 96kbps, best quality-per-bit".into(),
            category: PresetCategory::Audio,
            params: EncodeParams {
                video_codec: VideoCodec::Copy,
                audio_codec: AudioCodec::Opus,
                container: Container::Mkv,
                audio_bitrate: 96,
                sample_rate: 48000,
                threads: 0,
                ..Default::default()
            },
        },
        // ── GIF ──
        Preset {
            id: "gif".into(),
            name: "Animated GIF".into(),
            description: "Convert video to GIF: 480p, 15fps, optimized palette".into(),
            category: PresetCategory::Image,
            params: EncodeParams {
                video_codec: VideoCodec::Copy, // handled by ffmpeg gif muxer
                container: Container::Gif,
                width: 480,
                height: 270,
                crf: None,
                pixel_format: PixelFormat::Yuv420p,
                threads: 0,
                ..Default::default()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_count() { assert_eq!(builtin_presets().len(), 9); }

    #[test]
    fn ids_are_unique() {
        let presets = builtin_presets();
        let mut ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), presets.len());
    }

    #[test]
    fn every_preset_has_rate_control() {
        for p in builtin_presets() {
            if p.id == "gif" { continue; } // GIF uses remux, no rate control needed
            assert!(p.params.crf.is_some() || p.params.video_bitrate.is_some(),
                "Preset {} has neither CRF nor bitrate", p.id);
        }
    }

    #[test]
    fn preset_default_exists() {
        assert!(builtin_presets().iter().any(|p| p.id == "default"));
    }

    #[test]
    fn categories_valid() {
        for p in builtin_presets() {
            match p.category {
                PresetCategory::Video | PresetCategory::Audio | PresetCategory::Image => {}
            }
        }
    }
}
