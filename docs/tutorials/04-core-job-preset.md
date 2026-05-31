# Tutorial 04 — Job, Preset e Fila de Encoding

**Objetivo:** Implementar o modelo de dados para jobs de encoding e o
sistema de presets (built-in + custom).

**Tempo estimado:** 45 minutos

---

## 1. job.rs — O coração do modelo de dados

### EncodeParams

`EncodeParams` é o objeto central. Ele contém TODOS os parâmetros que
definem uma conversão de mídia. É serializado/deserializado entre Rust
e JavaScript via JSON (serde).

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::enums::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeParams {
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub width: u32,
    pub height: u32,
    pub crf: Option<u32>,           // 0-51, None = usar bitrate
    pub video_bitrate: Option<u32>,  // kbps
    pub max_bitrate: Option<u32>,
    pub bufsize: Option<u32>,
    pub preset: PresetSpeed,
    pub profile: Option<Profile>,
    pub pixel_format: PixelFormat,
    pub fps: FpsMode,
    pub scale_algorithm: ScaleAlgorithm,
    pub deinterlace: Option<DeinterlaceMethod>,
    pub audio_bitrate: u32,
    pub audio_channels: u32,
    pub sample_rate: u32,
    pub threads: u32,               // 0 = auto
    pub movflags: Vec<MovFlag>,
    pub video_filters: Vec<VideoFilter>,
    pub audio_filters: Vec<AudioFilter>,
    pub metadata: HashMap<String, String>,
    pub trim_start: Option<String>,  // ex: "00:01:30" ou "90"
    pub trim_end: Option<String>,
    pub extra_args: Vec<String>,     // argumentos ffmpeg extras
}

impl Default for EncodeParams {
    fn default() -> Self {
        Self {
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            container: Container::Mp4,
            width: 1920,
            height: 1080,
            crf: Some(19),
            video_bitrate: None,
            max_bitrate: None,
            bufsize: None,
            preset: PresetSpeed::Medium,
            profile: None,
            pixel_format: PixelFormat::Yuv420p,
            fps: FpsMode::SameAsSource,
            scale_algorithm: ScaleAlgorithm::Lanczos,
            deinterlace: None,
            audio_bitrate: 128,
            audio_channels: 2,
            sample_rate: 48000,
            threads: 0,
            movflags: vec![MovFlag::FastStart],
            video_filters: vec![],
            audio_filters: vec![],
            metadata: HashMap::new(),
            trim_start: None,
            trim_end: None,
            extra_args: vec![],
        }
    }
}
```

**Valores default e por quê:**
- `crf: Some(19)`: qualidade alta visível, arquivo razoável
- `sample_rate: 48000`: compatível com Opus (44100 causa erro)
- `movflags: [FastStart]`: moov atom no início → streaming friendly
- `threads: 0`: deixa o ffmpeg decidir (ótimo para CPUs modernas)

### ProgressInfo

Estrutura efêmera usada DURANTE o encoding para reportar progresso:

```rust
#[derive(Debug, Clone, Default)]
pub struct ProgressInfo {
    pub frame: u64,
    pub fps: f64,
    pub bitrate: String,
    pub total_size: u64,
    pub out_time_us: u64,   // microssegundos processados
    pub speed: f64,
    pub progress_pct: f64,  // 0.0 a 100.0
}
```

`progress_pct` é calculado como `out_time_us / total_duration * 100`.

### Job

```rust
use uuid::Uuid;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub params: EncodeParams,
    pub status: JobStatus,
    pub progress: f64,
    pub error: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Job {
    pub fn new(input: PathBuf, output: PathBuf, params: EncodeParams) -> Self {
        Self {
            id: Uuid::new_v4(),
            input_path: input,
            output_path: output,
            params,
            status: JobStatus::Pending,
            progress: 0.0,
            error: None,
            command: None,
        }
    }
}
```

**`Uuid::new_v4()`:** Gera um identificador único universal. Dois jobs
nunca terão o mesmo ID, mesmo em máquinas diferentes.

**Referência UUID:** https://docs.rs/uuid

### JobQueue

```rust
pub struct JobQueue {
    pub jobs: Vec<Job>,
}

impl JobQueue {
    pub fn new() -> Self { Self { jobs: vec![] } }

    pub fn add(&mut self, job: Job) { self.jobs.push(job); }

    pub fn remove(&mut self, id: &Uuid) {
        self.jobs.retain(|j| j.id != *id);
    }

    pub fn count(&self) -> usize { self.jobs.len() }

    pub fn overall_progress(&self) -> f64 {
        if self.jobs.is_empty() { return 0.0; }
        let total: f64 = self.jobs.iter().map(|j| j.progress).sum();
        total / self.jobs.len() as f64
    }
}
```

---

## 2. preset.rs — Sistema de Presets

### Estrutura

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,              // "web-h264", "audio-mp3"
    pub name: String,            // "Web Video (H.264)"
    pub description: String,     // tooltip descritivo
    pub category: PresetCategory,
    pub params: EncodeParams,
}
```

### Presets built-in

```rust
pub fn builtin_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "default".into(),
            name: "Default (H.264)".into(),
            description: "H.264, CRF 19, yuv420p, faststart".into(),
            category: PresetCategory::Video,
            params: EncodeParams::default(),
        },
        Preset {
            id: "web-h264".into(),
            name: "Web Video (H.264)".into(),
            description: "Optimizado para web — H.264, yuv420p, faststart, CRF 23".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                crf: Some(23),
                ..EncodeParams::default()
            },
        },
        Preset {
            id: "web-vp9".into(),
            name: "Web Video (VP9)".into(),
            description: "Codec aberto — VP9 + Opus em WebM".into(),
            category: PresetCategory::Video,
            params: EncodeParams {
                video_codec: VideoCodec::VP9,
                audio_codec: AudioCodec::Opus,
                container: Container::Webm,
                crf: Some(30),
                sample_rate: 48000,  // Opus requer 48000!
                movflags: vec![],     // WebM não suporta faststart
                ..EncodeParams::default()
            },
        },
        Preset {
            id: "audio-mp3".into(),
            name: "Audio MP3".into(),
            description: "Extrai áudio como MP3 192kbps".into(),
            category: PresetCategory::Audio,
            params: EncodeParams {
                video_codec: VideoCodec::Copy,  // strip video
                audio_codec: AudioCodec::Mp3,
                container: Container::Mp3,       // será .mp3
                audio_bitrate: 192,
                ..EncodeParams::default()
            },
        },
        // Adicione: web-h265, archive-h264, audio-aac, audio-opus, gif
    ]
}
```

**Por que presets como dados e não como código?** Porque podem ser
serializados, enviados ao frontend, e o usuário pode criar custom
presets que são persistidos em JSON.

---

## 3. Testes

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_params_default_values() {
        let p = EncodeParams::default();
        assert_eq!(p.video_codec, VideoCodec::H264);
        assert_eq!(p.width, 1920);
        assert_eq!(p.sample_rate, 48000);
    }

    #[test]
    fn job_new_has_unique_id() {
        let j1 = Job::new("a.mp4".into(), "b.mp4".into(), EncodeParams::default());
        let j2 = Job::new("a.mp4".into(), "b.mp4".into(), EncodeParams::default());
        assert_ne!(j1.id, j2.id);  // UUIDs são únicos
    }

    #[test]
    fn job_queue_overall_progress() {
        let mut q = JobQueue::new();
        let mut j1 = Job::new("a".into(), "b".into(), EncodeParams::default());
        j1.progress = 50.0;
        let mut j2 = Job::new("c".into(), "d".into(), EncodeParams::default());
        j2.progress = 100.0;
        q.add(j1); q.add(j2);
        assert_eq!(q.overall_progress(), 75.0);
    }

    #[test]
    fn builtin_presets_have_unique_ids() {
        let presets = builtin_presets();
        let ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        let mut unique = ids.clone();
        unique.sort(); unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }
}
```

---

## 4. Verificação

```bash
cargo test -p mediaforge-core
```

Deve passar ~20 testes (job + preset + enums + error + config).

---

## Próximo passo

Tutorial 05 — `ffmpeg.rs` Parte 1: `build_command()` — construir comandos
ffmpeg sem concatenar strings.
