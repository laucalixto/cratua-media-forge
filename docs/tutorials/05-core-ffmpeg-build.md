# Tutorial 05 — ffmpeg: Construção de Comandos

**Objetivo:** Implementar `build_command()` — a função que transforma
`EncodeParams` em um comando ffmpeg executável, usando `std::process::Command`
em vez de concatenação de strings.

**Tempo estimado:** 1 hora

---

## 1. Por que `std::process::Command` e não string?

**ERRADO (concatenação de strings):**
```rust
let cmd = format!("ffmpeg -i {} -c:v {} output.mp4", input, codec);
// Problema: se input = "file with spaces.mp4", o comando quebra
// Problema: vulnerável a injection se input vier do usuário
```

**CERTO (`std::process::Command`):**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.arg("-i").arg(input);
cmd.arg("-c:v").arg(codec);
cmd.arg("output.mp4");
// Cada .arg() é um argumento separado. O SO gerencia escaping.
```

**Referência:** https://doc.rust-lang.org/std/process/struct.Command.html

---

## 2. Estrutura básica

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_command(params: &EncodeParams, input: &Path, output: &Path) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // 1. Overwrite + input (SEMPRE primeiro)
    cmd.arg("-y")
       .arg("-i").arg(input);

    // 2. Codec de vídeo
    cmd.arg("-c:v").arg(params.video_codec.ffmpeg_name());

    // 3. Controle de qualidade
    if let Some(crf) = params.crf {
        cmd.arg("-crf").arg(crf.to_string());
    }
    if let Some(bitrate) = params.video_bitrate {
        cmd.arg("-b:v").arg(format!("{}k", bitrate));
    }

    // 4. Preset (velocidade vs qualidade)
    cmd.arg("-preset").arg(params.preset.ffmpeg_name());

    // 5. Perfil (apenas H.264)
    if let Some(profile) = &params.profile {
        cmd.arg("-profile:v").arg(profile.ffmpeg_name());
    }

    // ... continua ...
}
```

**Ordem dos argumentos importa:**
1. `-y` (overwrite) deve vir ANTES de `-i` — se não, ffmpeg pergunta
   "Overwrite? [y/N]" e trava
2. `-i input` antes dos codecs — ffmpeg precisa saber o que abrir
3. Codecs antes dos parâmetros específicos (CRF, bitrate, preset)
4. Output path é SEMPRE o último argumento

---

## 3. Video filters

Filtros de vídeo são concatenados com `,` e passados como `-vf`:

```rust
let mut vf_parts: Vec<String> = Vec::new();

// Deinterlace (se ativo)
if let Some(method) = &params.deinterlace {
    vf_parts.push(method.ffmpeg_name().to_string());
}

// Scale (sempre presente — redimensionamento)
vf_parts.push(format!(
    "scale={}:{}:flags={}",
    params.width,
    params.height,
    params.scale_algorithm.ffmpeg_flag()
));

// Video filters do usuário (hflip, denoise, rotate, etc.)
for filter in &params.video_filters {
    vf_parts.push(filter.to_ffmpeg_string());
}

if !vf_parts.is_empty() {
    cmd.arg("-vf").arg(vf_parts.join(","));
}
```

**Exemplo de saída:** `-vf yadif,scale=1920:1080:flags=lanczos,hflip`

---

## 4. GIF — caso especial

Container GIF requer um comando completamente diferente:

```rust
if params.container == Container::Gif {
    let fps = match params.fps {
        FpsMode::Fixed(f) => f,
        _ => 10,  // default 10 fps para GIF
    };

    // Palette filter chain: gera paleta de 256 cores otimizada
    let vf = format!(
        "fps={},scale={}:{}:flags={},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
        fps,
        params.width,
        params.height,
        params.scale_algorithm.ffmpeg_flag()
    );

    cmd.arg("-vf").arg(vf);
    cmd.arg("-progress").arg("pipe:2").arg("-nostats");
    cmd.arg(output);
    return cmd;  // EARLY RETURN — não adiciona codecs/áudio
}
```

**Por que `return cmd`?** GIF não usa `-c:v` (usa encoder interno),
não tem áudio, não tem pixel format. O early return evita que o código
abaixo adicione flags inválidas (ex: `-c:v libx264` em GIF causa erro).

**Referência:** https://trac.ffmpeg.org/wiki/Create%20a%20thumbnail%20image%20sequence#GIF

---

## 5. Codec-specific flags

VP9, AV1 e SVTAV1 não usam `-preset medium`. Cada um tem seu próprio
mecanismo:

```rust
match params.video_codec {
    VideoCodec::VP9 => {
        cmd.arg("-deadline").arg("good");
        cmd.arg("-cpu-used")
            .arg(params.preset.vp9_cpu_used().to_string());
    }
    VideoCodec::AV1 => {
        cmd.arg("-cpu-used")
            .arg(params.preset.av1_cpu_used().to_string());
    }
    VideoCodec::SVTAV1 => {
        cmd.arg("-preset")
            .arg(params.preset.svtav1_preset().to_string());
    }
    _ => {
        cmd.arg("-preset").arg(params.preset.ffmpeg_name());
    }
}
```

Mapeamento de `PresetSpeed` para cada codec (implemente em `enums.rs`):

```rust
impl PresetSpeed {
    pub fn vp9_cpu_used(&self) -> u8 {
        match self {
            PresetSpeed::Ultrafast => 5,
            PresetSpeed::Medium => 2,
            PresetSpeed::Veryslow => 0,
            // ... mapear todos
        }
    }
    pub fn av1_cpu_used(&self) -> u8 {
        match self {
            PresetSpeed::Ultrafast => 8,
            PresetSpeed::Medium => 3,
            PresetSpeed::Veryslow => 0,
            // ...
        }
    }
    pub fn svtav1_preset(&self) -> u8 {
        match self {
            PresetSpeed::Ultrafast => 13,  // ordem INVERTIDA!
            PresetSpeed::Medium => 5,
            PresetSpeed::Veryslow => 0,
            // ...
        }
    }
}
```

**Referências:**
- https://trac.ffmpeg.org/wiki/Encode/VP9
- https://trac.ffmpeg.org/wiki/Encode/AV1

---

## 6. Áudio, metadata, trim, flags

```rust
// Codec de áudio
cmd.arg("-c:a").arg(params.audio_codec.ffmpeg_name());

// Bitrate, channels, sample rate
cmd.arg("-b:a").arg(format!("{}k", params.audio_bitrate));
if params.audio_channels > 0 {
    cmd.arg("-ac").arg(params.audio_channels.to_string());
}

// ⚠ Sample rate: Opus rejeita 44100
let sample_rate = if params.audio_codec == AudioCodec::Opus
    && ![48000, 24000, 16000, 12000, 8000].contains(&params.sample_rate)
{
    48000  // fallback seguro
} else {
    params.sample_rate
};
cmd.arg("-ar").arg(sample_rate.to_string());

// Pixel format
cmd.arg("-pix_fmt").arg(params.pixel_format.ffmpeg_name());

// Movflags
if !params.movflags.is_empty() {
    let flags: Vec<&str> = params.movflags.iter()
        .map(|f| f.ffmpeg_name()).collect();
    cmd.arg("-movflags").arg(flags.join("+"));
}

// Threads (0 = auto)
if params.threads > 0 {
    cmd.arg("-threads").arg(params.threads.to_string());
}

// Trim
if let Some(ref start) = params.trim_start {
    cmd.arg("-ss").arg(start);
}
if let Some(ref end) = params.trim_end {
    cmd.arg("-to").arg(end);
}

// Metadata
for (key, value) in &params.metadata {
    cmd.arg("-metadata").arg(format!("{}={}", key, value));
}

// Extra args (avançado)
for arg in &params.extra_args {
    cmd.arg(arg);
}

// Progress reporting (SEMPRE por último, antes do output)
cmd.arg("-progress").arg("pipe:2")
   .arg("-nostats");

// Output path (SEMPRE o último argumento)
cmd.arg(output);

cmd
```

---

## 7. Versão com ffmpeg_path customizado

```rust
pub fn build_command_with_ffmpeg(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
    ffmpeg_path: Option<&Path>,
) -> Command {
    let mut cmd = if let Some(path) = ffmpeg_path {
        Command::new(path)
    } else {
        Command::new("ffmpeg")
    };
    // ... resto igual a build_command(), mas em vez de
    // Command::new("ffmpeg") usa a variável cmd
}
```

---

## 8. Windows: esconder janela CMD

```rust
#[cfg(target_os = "windows")]
{
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
```

**⚠ Coloque isso DEPOIS de `Command::new()` e ANTES de qualquer
`return cmd;` (como no early return do GIF). Se um `return` executar
antes do `creation_flags`, o ffmpeg abrirá uma janela CMD.**

---

## 9. command_to_string — preview na UI

```rust
pub fn command_to_string(
    params: &EncodeParams,
    input: &Path,
    output: &Path,
) -> String {
    let cmd = build_command(params, input, output);
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd.get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    format!("{} {}", program, args.join(" "))
}
```

---

## 10. Testes

Escreva testes para cada codec, cada parâmetro, cada edge case:

```rust
#[test]
fn codec_h264() {
    let p = EncodeParams { video_codec: VideoCodec::H264, ..Default::default() };
    let s = cmd_str(&p);
    assert!(s.contains("libx264"));
}

#[test]
fn gif_no_video_codec() {
    let p = EncodeParams {
        container: Container::Gif,
        fps: FpsMode::Fixed(10),
        width: 480, height: 270,
        ..Default::default()
    };
    let s = cmd_str(&p);
    assert!(!s.contains("-c:v"), "GIF não deve ter codec de vídeo");
    assert!(s.contains("palettegen"));
}

#[test]
fn webm_vp9_uses_deadline() {
    let p = EncodeParams {
        video_codec: VideoCodec::VP9,
        container: Container::Webm,
        ..Default::default()
    };
    let s = cmd_str(&p);
    assert!(s.contains("-deadline good"));
    assert!(s.contains("-cpu-used"));
    assert!(!s.contains("-preset "), "VP9 não usa -preset");
}
```

---

## 11. Verificação

```bash
cargo test -p mediaforge-core
```

Deve passar ~80 testes só de ffmpeg (codecs, presets, formatos, filtros,
trim, metadata, GIF, VP9, sample rate, edge cases).

---

## Próximo passo

Tutorial 06 — `ffmpeg.rs` Parte 2: execução com progresso, timeout e
cancelamento.
