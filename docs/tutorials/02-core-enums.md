# Tutorial 02 — Tipos de Domínio (enums.rs)

**Objetivo:** Definir todos os tipos de dados que representam o domínio
do conversor de mídia. Este arquivo é a fundação sobre a qual todo o
resto do app é construído.

**Tempo estimado:** 45 minutos

---

## 1. O princípio: make invalid states unrepresentable

Em vez de usar strings para representar codecs ("h264", "libx264", "H.264"),
usamos **enums** do Rust. O compilador garante que nenhum valor inválido
existe em runtime. Se o código compila, todos os codecs são válidos.

**Exemplo do que NÃO fazer:**
```rust
fn encode(codec: &str) {  // pode ser "h264", "H.264", "libx264", "xyz"...
    // runtime error se codec for inválido
}
```

**Exemplo do que fazer:**
```rust
fn encode(codec: VideoCodec) {  // compilador garante que é válido
    let ffmpeg_name = codec.ffmpeg_name();
}
```

---

## 2. Derive macros

Todo enum usa estas derive macros:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
```

| Macro | Crate | Propósito |
|-------|-------|-----------|
| `Debug` | std | Permite `println!("{:?}", codec)` |
| `Clone` | std | Permite `.clone()` (cópia explícita) |
| `Copy` | std | Permite cópia implícita (só para tipos pequenos) |
| `PartialEq` | std | Permite `codec1 == codec2` |
| `Eq` | std | Garante que `a == a` sempre (sem NaN) |
| `Serialize` | serde | Converte para JSON (envio ao frontend) |
| `Deserialize` | serde | Converte de JSON (recebido do frontend) |

**Referência serde:** https://serde.rs/derive.html

---

## 3. VideoCodec

```rust
pub enum VideoCodec {
    H264,
    H265,
    VP9,
    AV1,
    SVTAV1,
    Copy,
}

impl VideoCodec {
    /// Retorna o nome do codec usado na linha de comando do ffmpeg
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

    /// Nome amigável para exibição na UI
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
}
```

**Explicação de cada codec:**

| Codec | Encoder ffmpeg | Uso típico |
|-------|---------------|------------|
| H264 | `libx264` | Compatibilidade universal, MP4, web |
| H265 | `libx265` | 50% mais eficiente que H.264, 4K/8K |
| VP9 | `libvpx-vp9` | Codec aberto Google, YouTube, WebM |
| AV1 | `libaom-av1` | Codec aberto, 30% melhor que VP9, futuro |
| SVTAV1 | `libsvtav1` | Encoder AV1 mais rápido (Intel/Netflix) |
| Copy | `copy` | Stream copy, sem re-encode (remux) |

**Referências:**
- https://trac.ffmpeg.org/wiki/Encode/H.264
- https://trac.ffmpeg.org/wiki/Encode/H.265
- https://trac.ffmpeg.org/wiki/Encode/VP9
- https://trac.ffmpeg.org/wiki/Encode/AV1

---

## 4. AudioCodec

```rust
pub enum AudioCodec {
    Aac,     // → "aac" (padrão MP4)
    Mp3,     // → "libmp3lame" (legado universal)
    Opus,    // → "libopus" (melhor qualidade/bitrate)
    Vorbis,  // → "libvorbis" (aberto, WebM)
    Flac,    // → "flac" (lossless)
    Copy,    // → "copy"
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
}
```

**⚠ PITFALL — Opus e sample rate:** O encoder `libopus` rejeita 44100 Hz.
Só aceita 48000, 24000, 16000, 12000, 8000 Hz. Vamos implementar validação
em 3 camadas (HTML `<select>` default, JS `fixOpusSampleRate()`, Rust
`build_command()`). Isso será abordado nos tutoriais 06 e 12.

**Referência:** https://ffmpeg.org/ffmpeg-codecs.html#libopus

---

## 5. PresetSpeed

```rust
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
}
```

Controle de velocidade/qualidade do encoder x264/x265:

| Preset | Velocidade relativa | Tamanho do arquivo |
|--------|--------------------|--------------------|
| Ultrafast | ~10x mais rápido | ~30% maior |
| Medium | 1x (referência) | 1x (referência) |
| Veryslow | ~5x mais lento | ~20% menor |

Para VP9 e AV1, o mapeamento é diferente (ver Tutorial 06).

---

## 6. Demais enums

Seguindo o mesmo padrão, defina:

- `Container`: Mp4, Mkv, Webm, Mov, Avi, Gif — com `fn ext(&self)` que
  retorna a extensão (ex: `"mp4"`, `"webm"`)
- `PixelFormat`: Yuv420p (padrão, compatível), Yuv422p, Yuv444p, Nv12, Rgb24
- `DeinterlaceMethod`: Yadif, Bwdif
- `Profile`: Baseline, Main, High (apenas H.264)
- `ScaleAlgorithm`: Bilinear, Bicubic, Lanczos
- `MovFlag`: FastStart, FragKeyframe
- `FpsMode`: SameAsSource, Fixed(u32)
- `Theme`: Dark, Light
- `UiMode`: Simple, Advanced
- `PresetCategory`: Video, Audio, Image

---

## 7. Testes

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_codec_names() {
        assert_eq!(VideoCodec::H264.ffmpeg_name(), "libx264");
        assert_eq!(VideoCodec::Copy.ffmpeg_name(), "copy");
    }

    #[test]
    fn video_codec_count() {
        assert_eq!(VideoCodec::ALL.len(), 6);
    }

    #[test]
    fn container_ext() {
        assert_eq!(Container::Mp4.ext(), "mp4");
        assert_eq!(Container::Webm.ext(), "webm");
    }
}
```

---

## 8. Verificação

```bash
cargo test -p mediaforge-core
```

Deve passar todos os testes de enum.

---

## Próximo passo

Tutorial 03 — `error.rs` e `config.rs`: tratamento de erros tipado e
persistência de configuração.
