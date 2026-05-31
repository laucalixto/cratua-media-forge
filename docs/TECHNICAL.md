# Cratua Media Forge — Documentação Técnica Completa

> Curso de pós-graduação: do `Cargo.toml` ao binário cross-compilado.
> Cada linha de código explicada, cada decisão de design justificada.

**Versão:** 0.4.1 | **Autor:** laucalixto | **Licença:** MIT

---

## Índice

1. [Arquitetura Geral](#1-arquitetura-geral)
2. [Cargo Workspace](#2-cargo-workspace)
3. [Crate: mediaforge-core](#3-crate-mediaforge-core)
   - [3.1 enums.rs — Tipos de Domínio](#31-enumsrs)
   - [3.2 error.rs — Tratamento de Erros](#32-errorrs)
   - [3.3 config.rs — Persistência de Configuração](#33-configrs)
   - [3.4 i18n.rs — Internacionalização](#34-i18nrs)
   - [3.5 job.rs — Modelo de Job e Fila](#35-jobrs)
   - [3.6 preset.rs — Sistema de Presets](#36-presetrs)
   - [3.7 ffmpeg.rs — O Coração do App](#37-ffmpegr)
4. [Crate: mediaforge-tauri](#4-crate-mediaforge-tauri)
   - [4.1 lib.rs — Entry Point Tauri](#41-librs)
   - [4.2 commands.rs — Ponte IPC](#42-commandsrs)
   - [4.3 main.rs — Binary Entry](#43-mainrs)
   - [4.4 build.rs — Build Script](#44-buildrs)
5. [Frontend: HTML + CSS](#5-frontend-html--css)
   - [5.1 index.html — Estrutura da Interface](#51-indexhtml)
   - [5.2 input.css / bundle.css — Tailwind v4](#52-inputcss--bundlecss)
6. [Frontend: JavaScript](#6-frontend-javascript)
   - [6.1 Estado Global (S)](#61-estado-global-s)
   - [6.2 Inicialização e Splash](#62-inicialização-e-splash)
   - [6.3 Modos Simple/Advanced](#63-modos-simpleadvanced)
   - [6.4 Sliders e Inputs Vinculados](#64-sliders-e-inputs-vinculados)
   - [6.5 Presets: CRUD e Sync](#65-presets-crud-e-sync)
   - [6.6 Filtros e Metadata](#66-filtros-e-metadata)
   - [6.7 Command Preview](#67-command-preview)
   - [6.8 Arquivos: Add, Drag-Drop, Render](#68-arquivos-add-drag-drop-render)
   - [6.9 Fila e Encoding](#69-fila-e-encoding)
   - [6.10 Eventos: job:diag, job:progress, job:done](#610-eventos)
   - [6.11 Settings, About, Tema, Atalhos](#611-settings-about-tema-atalhos)
7. [Build Pipeline](#7-build-pipeline)
   - [7.1 npm scripts](#71-npm-scripts)
   - [7.2 scripts/check.sh](#72-scriptschecksh)
   - [7.3 scripts/release.sh](#73-scriptsreleasesh)
8. [Cross-Compilação: Linux → Windows](#8-cross-compilação)
9. [Estratégia de Testes](#9-estratégia-de-testes)
10. [Pitfalls e Lições Aprendidas](#10-pitfalls-e-lições-aprendidas)

---

## 1. Arquitetura Geral

```
┌─────────────────────────────────────────────────┐
│                  Tauri Shell                      │
│  ┌──────────────┐  ┌──────────────────────────┐ │
│  │  main.rs      │  │  ui/                      │ │
│  │  (binary)     │  │  ├── index.html           │ │
│  └──────┬───────┘  │  ├── bundle.js (esbuild)   │ │
│         │           │  ├── bundle.css (Tailwind) │ │
│  ┌──────▼───────┐  │  └── src/styles/input.css  │ │
│  │  lib.rs       │  └──────────────────────────┘ │
│  │  (invoke      │                                │
│  │   handler)    │                                │
│  └──────┬───────┘                                │
│         │                                         │
│  ┌──────▼───────┐                                │
│  │  commands.rs  │  ◄── IPC (invoke/events) ──►  │
│  │  (Tauri       │                                │
│  │   commands)   │                                │
│  └──────┬───────┘                                │
│         │                                         │
│  ┌──────▼───────┐                                │
│  │  mediaforge-  │                                │
│  │  core         │                                │
│  │  (ffmpeg,     │                                │
│  │   preset,     │                                │
│  │   config,     │                                │
│  │   job)        │                                │
│  └──────────────┘                                │
└─────────────────────────────────────────────────┘
```

**Tauri v2** (https://v2.tauri.app) é o framework que une o frontend web
(HTML/CSS/JS) com o backend Rust. Diferente do Electron, o Tauri não embute
um Chromium inteiro — ele usa o WebView nativo do sistema operacional
(WebKitGTK no Linux, WebView2 no Windows).

**Por que Tauri e não Electron?**
- Binário ~5MB vs ~150MB do Electron
- Memória: ~50MB vs ~300MB+ do Electron
- Single binary, sem `node_modules` de 500MB
- Rust como backend (zero-cost abstractions, sem GC)

**Por que vanilla JS e não React/Vue/Svelte?**
- O app tem ~250 linhas de JS. React adicionaria 40KB+ de runtime.
- Sem build step complexo (só esbuild para bundle + minify)
- Tailwind v4 já resolve 90% da complexidade de CSS
- Menos dependências = menos superfície de ataque e breaking changes

---

## 2. Cargo Workspace

**Arquivo:** `Cargo.toml` (raiz)

```toml
[workspace]
members = ["crates/mediaforge-core", "crates/mediaforge-tauri"]
resolver = "2"
```

O workspace é o mecanismo do Cargo para gerenciar múltiplos crates que
compartilham dependências e versão. O `resolver = "2"` ativa o resolver
de features da edição 2021, necessário para compilação condicional e
features como `serde/derive`.

**Por que separar em `mediaforge-core` e `mediaforge-tauri`?**

Separação de responsabilidades:
- `mediaforge-core`: zero dependências de UI. Pode ser testado sem Tauri,
  sem WebView, sem janela. Contém toda a lógica de ffmpeg, presets,
  configuração e jobs.
- `mediaforge-tauri`: thin wrapper que expõe as funções do core como
  comandos Tauri. Lida com IPC, eventos, e a janela.

Essa separação permite testar o core com `cargo test -p mediaforge-core`
(124 testes, ~0.2s) sem compilar Tauri.

**Referências:**
- Cargo Workspaces: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Resolver v2: https://doc.rust-lang.org/cargo/reference/resolver.html

---

## 3. Crate: mediaforge-core

### 3.1 enums.rs

**Arquivo:** `crates/mediaforge-core/src/enums.rs` (474 linhas)

Este arquivo define TODOS os tipos de domínio usados pelo app. A abordagem
é "make invalid states unrepresentable" — usar enums do Rust em vez de
strings que podem conter valores inválidos.

#### VideoCodec

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    VP9,
    AV1,
    SVTAV1,
    Copy,
}
```

Cada variante mapeia para um codec ffmpeg via `ffmpeg_name()`:
- `H264` → `"libx264"` (encoder software H.264 mais usado no mundo)
- `H265` → `"libx265"` (HEVC, ~50% mais eficiente que H.264)
- `VP9` → `"libvpx-vp9"` (codec aberto do Google, alternativa ao HEVC)
- `AV1` → `"libaom-av1"` (codec da Alliance for Open Media, ~30% melhor que VP9)
- `SVTAV1` → `"libsvtav1"` (encoder AV1 mais rápido, Intel/Netflix)
- `Copy` → `"copy"` (stream copy, sem re-encode — útil para remux)

**Por que `Copy` existe?** Para operações de remux (trocar container sem
re-encode). Ex: converter MKV → MP4 mantendo codecs originais. Isso é
extremamente rápido (I/O bound, não CPU bound).

#### AudioCodec

```rust
pub enum AudioCodec {
    Aac,    // → "aac" (padrão MP4, compatibilidade máxima)
    Mp3,    // → "libmp3lame" (legado, mas universal)
    Opus,   // → "libopus" (melhor qualidade/bitrate, WebM)
    Vorbis, // → "libvorbis" (aberto, WebM)
    Flac,   // → "flac" (lossless, arquivamento)
    Copy,   // → "copy"
}
```

**Pitfall com Opus:** `libopus` rejeita sample rates não suportados.
Apenas 48000, 24000, 16000, 12000, 8000 Hz são aceitos. 44100 Hz (padrão
CD) causa `exit code -22 (EINVAL)`. O app tem validação em 3 camadas:
- HTML: `<select>` default é 48000
- JS: `fixOpusSampleRate()` ajusta ao selecionar Opus
- Rust: `build_command()` faz fallback para 48000 com warning

**Referência:** https://ffmpeg.org/ffmpeg-codecs.html#libopus

#### PresetSpeed

```rust
pub enum PresetSpeed {
    Ultrafast, Superfast, Veryfast, Faster, Fast,
    Medium, Slow, Slower, Veryslow,
}
```

Controla a relação velocidade/qualidade do encoder x264/x265. `Ultrafast`
é ~10x mais rápido que `Veryslow`, mas produz arquivos ~30% maiores para
a mesma qualidade percebida. O default é `Medium`.

Para VP9/AV1/SVTAV1, o mapeamento é diferente — esses codecs não usam
`-preset`, usam `-cpu-used` (VP9/AV1) ou `-preset` numérico (SVTAV1).
Veja `build_command()` em ffmpeg.rs.

#### Container

```rust
pub enum Container {
    Mp4, Mkv, Webm, Mov, Avi, Gif,
}
```

Determina a extensão do arquivo de saída. O ffmpeg infere o container
pela extensão — não emitimos `-f` explicitamente. Exceção: GIF precisa
de tratamento especial (palette filter chain).

#### FpsMode

```rust
pub enum FpsMode {
    SameAsSource,
    Fixed(u32),
}
```

`SameAsSource` não adiciona `-r` ao comando. `Fixed(n)` adiciona `-r n`.

**Por que enum e não `Option<u32>`?** Clareza semântica. `None` poderia
significar "same as source" ou "não especificado". O enum torna explícito.

#### Demais enums

- `PixelFormat`: Yuv420p (padrão, compatível), Yuv422p, Yuv444p (mais
  qualidade de cor), Nv12 (hardware), Rgb24 (sem subsampling)
- `DeinterlaceMethod`: Yadif (padrão), Bwdif (mais suave em movimento)
- `Profile`: Baseline, Main, High (H.264 profiles — complexidade de
  features do decoder)
- `ScaleAlgorithm`: Bilinear (rápido), Bicubic, Lanczos (melhor qualidade)
- `MovFlag`: FastStart (moov atom no início), FragKeyframe (streaming)
- `VideoFilter`: HFlip, VFlip, Rotate(n), Denoise, Grayscale,
  Brightness(n), Contrast(n), Saturation(n)
- `AudioFilter`: Volume(n), Loudnorm, Highpass(n), Lowpass(n)
- `Theme`: Dark, Light
- `UiMode`: Simple, Advanced
- `PresetCategory`: Video, Audio, Image

Todos implementam `Serialize + Deserialize` via serde para passar entre
Rust e JavaScript via JSON.

### 3.2 error.rs

**Arquivo:** `crates/mediaforge-core/src/error.rs` (55 linhas)

```rust
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
    IncompatibleCodec { video_codec: String, container: String },

    #[error("No input files selected")]
    NoInputFiles,

    #[error("Encoding cancelled by user")]
    Cancelled,

    #[error("ffmpeg timed out after {0}s without output")]
    FfmpegTimeout(u64),
}
```

**Crate usada:** `thiserror` (https://docs.rs/thiserror) — derive macro que
implementa `std::error::Error` e `Display` automaticamente a partir dos
atributos `#[error("...")]`.

**`#[from]` em `Io`:** Permite usar `?` em funções que retornam
`Result<T, MediaForgeError>` com erros de I/O. O compilador converte
automaticamente `std::io::Error` em `MediaForgeError::Io`.

**`FfmpegTimeout`:** Adicionado no code review. Detecta hangs do ffmpeg
(30s sem output no stderr). Veja seção 3.7 para detalhes.

### 3.3 config.rs

**Arquivo:** `crates/mediaforge-core/src/config.rs` (108 linhas)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ffmpeg_path: Option<PathBuf>,
    pub language: Language,
    pub theme: Theme,
    pub default_mode: UiMode,
    pub output_dir: Option<PathBuf>,
    pub default_preset: Option<String>,
    pub parallel_jobs: u32,
}
```

Persistência via TOML em `~/.config/mediaforge/config.toml` (Linux) ou
`%APPDATA%/mediaforge/config.toml` (Windows).

**Por que TOML e não JSON?** TOML é mais legível para humanos editarem
manualmente, e o crate `toml` é mais leve que `serde_json` para
configuração. A crate `serde_json` já está presente para IPC com o
frontend (Tauri serializa comandos como JSON).

**Detecção de config_dir sem crate `dirs`:**
```rust
fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok()
                .map(|h| PathBuf::from(h).join(".config")))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }
}
```

**Por que não usar o crate `dirs`?** O crate `dirs` é simples (1 arquivo),
mas adiciona uma dependência desnecessária quando podemos resolver com
3 linhas de `std::env::var`. Menos dependências = compilação mais rápida
e menos risco de supply chain.

### 3.4 i18n.rs

**Arquivo:** `crates/mediaforge-core/src/i18n.rs`

Define o enum `Language` (EnUs, PtBr) com suporte a Fluent
(https://projectfluent.org). Os arquivos de tradução ficam em
`i18n/en-US/main.ftl` e `i18n/pt-BR/main.ftl`.

O sistema Fluent é usado pela Mozilla no Firefox. Diferente de gettext
(chaves opacas como `msgid "btn-start"`), Fluent usa chaves semânticas
com suporte a pluralização e variáveis:

```fluent
status-encoding = Convertendo... { $current } de { $total }
```

### 3.5 job.rs

**Arquivo:** `crates/mediaforge-core/src/job.rs`

Define o modelo de dados para um job de encoding:

```rust
pub struct Job {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub params: EncodeParams,
    pub status: JobStatus,
    pub progress: f64,       // 0.0 a 100.0
    pub error: Option<String>,
    pub command: Option<String>, // ffmpeg command line usado
}
```

`EncodeParams` é o objeto central que contém TODOS os parâmetros de
encoding. É o que transita entre frontend (coletado dos campos HTML),
Rust (validação e construção do comando), e ffmpeg (execução).

```rust
pub struct EncodeParams {
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub width: u32,
    pub height: u32,
    pub crf: Option<u32>,        // 0-51, None = usar bitrate
    pub video_bitrate: Option<u32>, // kbps
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
    pub threads: u32,         // 0 = auto
    pub movflags: Vec<MovFlag>,
    pub video_filters: Vec<VideoFilter>,
    pub audio_filters: Vec<AudioFilter>,
    pub metadata: HashMap<String, String>,
    pub trim_start: Option<String>,
    pub trim_end: Option<String>,
    pub extra_args: Vec<String>,
}
```

**ProgressInfo:** Estrutura efêmera usada durante o encoding para reportar
progresso via callback:

```rust
pub struct ProgressInfo {
    pub frame: u64,
    pub fps: f64,
    pub bitrate: String,
    pub total_size: u64,
    pub out_time_us: u64,    // microssegundos de vídeo processado
    pub speed: f64,           // velocidade relativa (1.0 = tempo real)
    pub progress_pct: f64,    // calculado: out_time_us / total_duration
}
```

### 3.7 ffmpeg.rs

**Arquivo:** `crates/mediaforge-core/src/ffmpeg.rs` (855 linhas)
**Este é o arquivo mais importante do projeto.**

#### build_command()

```rust
pub fn build_command(params: &EncodeParams, input: &Path, output: &Path) -> Command
```

Constrói um `std::process::Command` com todos os argumentos do ffmpeg.
**NUNCA usa concatenação de strings** — sempre `cmd.arg("valor")`, que
escapa automaticamente espaços e caracteres especiais.

**Estrutura do comando gerado:**
```
ffmpeg -y -i input.mp4 -c:v libx264 -crf 23 -vf scale=1920:1080
       -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart
       -progress pipe:2 -nostats output.mp4
```

**`-progress pipe:2`:** Faz o ffmpeg escrever informações de progresso no
stderr em formato `key=value`. Essencial para a barra de progresso.

**`-nostats`:** Suprime as estatísticas padrão do ffmpeg (que poluiriam
o parsing do progresso).

**Ordem de argumentos importa:**
1. `-y` (overwrite) deve vir ANTES de `-i` — se viesse depois, ffmpeg
   poderia perguntar "Overwrite? [y/N]" e travar
2. `-i input` deve vir antes dos codecs — ffmpeg precisa saber o que abrir
3. Codecs de vídeo e áudio antes dos parâmetros específicos
4. Filtros (`-vf`, `-af`) antes do output
5. `-progress pipe:2` e `-nostats` por último
6. Output path é sempre o último argumento

#### GIF handling

Container GIF requer um comando completamente diferente:
```rust
if params.container == Container::Gif {
    let fps = match params.fps { FpsMode::Fixed(f) => f, _ => 10 };
    let vf = format!(
        "fps={},scale={}:{}:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
        fps, params.width, params.height
    );
    cmd.arg("-vf").arg(vf);
    cmd.arg("-progress").arg("pipe:2").arg("-nostats");
    cmd.arg(output);
    return cmd;
}
```

O filter chain `palettegen/paletteuse` gera uma paleta otimizada de 256
cores para o GIF. Sem isso, o GIF teria banding severo de cores.

**Por que `return cmd` dentro do bloco GIF?** Porque o GIF não usa codec
de vídeo (`-c:v`), nem áudio, nem pixel format. O early return evita que
o código abaixo adicione flags inválidas.

#### VP9/AV1/SVTAV1 codec-specific flags

```rust
match params.video_codec {
    VideoCodec::VP9 => {
        cmd.arg("-deadline").arg("good");
        cmd.arg("-cpu-used").arg(params.preset.vp9_cpu_used().to_string());
    }
    VideoCodec::AV1 => {
        cmd.arg("-cpu-used").arg(params.preset.av1_cpu_used().to_string());
    }
    VideoCodec::SVTAV1 => {
        cmd.arg("-preset").arg(params.preset.svtav1_preset().to_string());
    }
    _ => {
        cmd.arg("-preset").arg(params.preset.ffmpeg_name());
    }
}
```

Cada codec tem seu próprio mecanismo de controle velocidade/qualidade:
- x264/x265: `-preset ultrafast` .. `veryslow`
- VP9: `-deadline good -cpu-used N` (0=melhor, 5=pior)
- AV1 (libaom): `-cpu-used N` (0=melhor, 8=pior)
- SVT-AV1: `-preset N` (0=melhor, 13=pior — ordem INVERTIDA!)

`PresetSpeed` tem métodos `vp9_cpu_used()`, `av1_cpu_used()`,
`svtav1_preset()` que mapeiam os nomes x264 para os valores numéricos
correspondentes de cada codec.

#### probe_duration()

```rust
pub fn probe_duration(input: &Path, ffmpeg_path: Option<&Path>) -> Option<u64>
```

Usa `ffprobe` (bundled junto com ffmpeg) para extrair a duração do vídeo
em microssegundos ANTES de começar o encoding. Isso permite calcular a
porcentagem real de progresso (`out_time_us / total_duration * 100`).

**Localização do ffprobe:** Procura no mesmo diretório que o ffmpeg
(substituindo `ffmpeg` por `ffprobe` no nome do arquivo). No Windows,
`ffprobe.exe`.

**Comando:** `ffprobe -v quiet -show_entries format=duration -of csv=p=0 input.mp4`

Saída: `120.500000` (segundos como float). Convertido para `120_500_000`
microssegundos.

#### run_with_progress_and_ffmpeg_cancellable()

```rust
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
```

Esta é a função principal de execução. Recebe um callback `on_progress`
que é chamado a cada atualização de progresso.

**Arquitetura do loop de leitura (refatorado no code review):**

```
┌──────────────┐     channel      ┌──────────────┐
│ Reader Thread │ ──── mpsc ────► │ Main Thread   │
│ (stderr loop) │      lines      │ (recv_timeout)│
└──────────────┘                  └──────┬───────┘
                                        │
                          ┌─────────────▼──────────┐
                          │ parse_progress_line()   │
                          │ → out_time_us: emit     │
                          │ → progress=end: emit    │
                          │ → outros: skip          │
                          └─────────────┬──────────┘
                                        │
                          ┌─────────────▼──────────┐
                          │ on_progress(&info)      │
                          │ → emit job:progress     │
                          └────────────────────────┘
```

**Por que thread separada + channel?** Para implementar timeout de 30
segundos. Se o ffmpeg travar (disco cheio, pipe quebrado), o
`recv_timeout` detecta e mata o processo:

```rust
Err(RecvTimeoutError::Timeout) => {
    let _ = child.kill();
    let _ = child.wait();
    drop(line_rx);
    let _ = reader_handle.join();
    return Err(MediaForgeError::FfmpegTimeout(30));
}
```

**Filtragem de progresso:** Apenas `out_time_us` e `progress=end` disparam
callbacks. Linhas como `frame=100`, `speed=2.5x`, `fps=30.0` são ignoradas
porque carregam `progress_pct=0.0` que sobrescreveria o valor real no
frontend.

#### parse_progress_line()

```rust
pub fn parse_progress_line(line: &str) -> Option<(String, String)>
```

Faz split em `=` e retorna `(key, value)`. Ignora linhas sem `=`. O
formato é específico do `-progress pipe:2` do ffmpeg.

#### command_to_string()

```rust
pub fn command_to_string(params: &EncodeParams, input: &Path, output: &Path) -> String
```

Usado pelo Command Preview no frontend para mostrar o comando ffmpeg que
será executado. Constrói o comando com `build_command()` e extrai os
argumentos com `cmd.get_args()`.

#### detect_ffmpeg()

```rust
pub fn detect_ffmpeg() -> Option<PathBuf>
```

Cadeia de prioridade:
1. Variável de ambiente `MEDIAFORGE_FFMPEG_PATH`
2. Bundled ao lado do executável (`./ffmpeg` ou `./ffmpeg/ffmpeg`)
3. Sistema PATH (`which ffmpeg`)

**Por que bundled ao lado do executável?** Para distribuição portable.
O usuário baixa o `.tar.gz`, extrai, e tem ffmpeg + app na mesma pasta.
Não precisa instalar ffmpeg separadamente.

---

## 4. Crate: mediaforge-tauri

### 4.1 lib.rs

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_default_output_dir,
            check_output_overwrite,
            get_presets,
            get_builtin_preset_ids,
            create_preset,
            delete_preset,
            get_config,
            save_config,
            detect_ffmpeg,
            build_command_preview,
            start_encoding,
            cancel_encoding,
            get_history,
            clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**`tauri::Builder::default()`:** Construtor padrão do Tauri. Configura a
janela, plugins, e comandos.

**Plugins:**
- `tauri_plugin_shell`: Permite abrir URLs no browser externo
- `tauri_plugin_dialog`: Diálogos nativos de abrir arquivo/pasta (não o
  `<input type="file">` do browser)

**`generate_handler![]`:** Macro que registra funções Rust como comandos
invocáveis do frontend via `invoke('nome_do_comando', { args })`. Cada
função se torna um endpoint IPC.

### 4.2 commands.rs

**Arquivo:** `crates/mediaforge-tauri/src/commands.rs` (520 linhas)

Cada função anotada com `#[tauri::command]` é um endpoint IPC.

**`start_encoding`** — o comando mais complexo:

```rust
#[tauri::command]
pub async fn start_encoding(
    job_data: Vec<JobData>,
    ffmpeg_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String>
```

Fluxo:
1. Converte `JobData` (leve, serializável) em `Job` (com PathBuf, Uuid)
2. Valida: output == input → erro (protege source file)
3. Dragon Trap: CRF e bitrate ambos None → erro
4. Cria diretórios de saída (`create_dir_all`)
5. Probe durações com ffprobe → emite `job:diag`
6. Cria `Arc<AtomicBool>` para cancelamento
7. Spawna thread que:
   - Roda ffmpeg com callback de progresso → emite `job:progress`
   - No final → emite `job:done`
   - Append ao history (lock-protected, max 200 entradas)
   - Sempre limpa o cancel flag (mesmo em pânico)

**Cancelamento:**
```rust
static CANCEL_FLAG: OnceLock<Mutex<Option<Arc<AtomicBool>>>> = OnceLock::new();
```

`Arc<AtomicBool>` compartilhado entre o comando `cancel_encoding` e a
thread de encoding. A thread verifica `flag.load(Ordering::Relaxed)` a
cada linha do stderr. Se `true`, mata o processo com `child.kill()`.

**Presets persistence:**
```rust
static PRESET_LOCK: Mutex<()> = Mutex::new(());
```

Mutex que protege leitura/escrita do arquivo `presets.json`. Padrão
idêntico ao `HISTORY_LOCK`. Evita corrupção em acesso concorrente
(improávável mas possível com múltiplos comandos Tauri simultâneos).

### 4.3 main.rs

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mediaforge_tauri_lib::run()
}
```

`windows_subsystem = "windows"` faz o Windows não abrir uma janela de
console junto com a GUI. Só afeta builds release (`not(debug_assertions)`).

### 4.4 build.rs

```rust
// crates/mediaforge-tauri/build.rs
fn main() {
    tauri_build::build()
}
```

O `tauri_build::build()` gera código em tempo de compilação que:
- Embute o ícone e metadados no binário
- Configura o Windows resource file (.rc)
- Gera o schema de capabilities

---

## 5. Frontend: HTML + CSS

### 5.1 index.html

**Arquivo:** `crates/mediaforge-tauri/ui/index.html` (393 linhas)

Single Page Application (SPA) vanilla. Estrutura:

```
body
├── #splash (overlay inicial, some após 2s)
└── #app (flex column, h-full)
    ├── header (h-12, modo + about + theme + settings)
    ├── div.flex.flex-1 (body principal)
    │   ├── #sidebar (files + history)
    │   └── main (conteúdo)
    │       ├── #mode-simple (preset + video + output)
    │       └── #mode-advanced (tabs + painéis)
    ├── footer (queue + botões de ação)
    └── .status-bar (Ready / progresso)
```

**Modais:** `#settings-modal` e `#about-modal` ficam fora do fluxo normal,
com `z-40` e `bg-black/50` (overlay semi-transparente).

### 5.2 input.css / bundle.css

**Tailwind v4** (https://tailwindcss.com) — utility-first CSS framework.

Diferente do Tailwind v3 (que usava `tailwind.config.js`), o v4 usa CSS
nativo com `@theme` e `@import "tailwindcss"`:

```css
@import "tailwindcss";

@theme {
  --color-rose: #be4266;
  --color-rose-light: #d46484;
  --color-rose-dark: #9e2e50;
}
```

Isso torna `bg-rose`, `text-rose`, `border-rose`, etc. disponíveis como
classes utilitárias.

**Paleta:** Rose (`#be4266`) como cor de destaque. Todo o resto usa tons
de cinza-azulado escuro (`#0b0b12`, `#151520`, `#2a2a3a`) para o tema
dark.

**CSS custom properties para light mode:**

```css
:root {
  --bg-deep: #0b0b12;
  --bg-card: #151520;
  --text-primary: #f4f4f6;
  /* ... */
}
.light {
  --bg-deep: #f8f9fa;
  --bg-card: #ffffff;
  --text-primary: #212529;
  /* ... */
}
```

Os elementos estruturais (body, header, sidebar, footer, status bar)
usam `style="background:var(--bg-deep)"` em vez de classes Tailwind
hardcoded. Os elementos internos (cards, inputs, textos) usam classes
Tailwind e são sobregravados via:

```css
.light .bg-\[\#151520\] { background: var(--bg-card) !important; }
```

O `!important` é necessário para vencer a especificidade das classes
utilitárias do Tailwind (que usam seletor de classe simples).

**Build do CSS:**
```bash
npx @tailwindcss/cli -i ui/src/styles/input.css -o ui/bundle.css
```

O Tailwind CLI varre `index.html` e `main.js` em busca de classes usadas,
gera apenas o CSS necessário (tree-shaking). O `bundle.css` final tem
~900 linhas.

---

## 6. Frontend: JavaScript

**Arquivo:** `crates/mediaforge-tauri/ui/src/main.js` (~252 linhas após
minificação, ~45KB bundle)

**Build:** `esbuild` (https://esbuild.github.io) — bundler extremamente
rápido (Go, não JS). Minifica, faz tree-shaking, resolve imports.

```bash
esbuild ui/src/main.js --bundle --minify --outfile=ui/bundle.js
```

### 6.1 Estado Global (S)

```js
const S = {
  mode: 'simple',           // 'simple' | 'advanced'
  presets: [],              // carregado do backend
  builtinIds: [],           // IDs de presets built-in (não deletáveis)
  selectedPreset: 'default',
  _syncingPreset: false,    // flag interno: evita falso "Custom"
  params: { /* EncodeParams completo */ },
  files: [],                // paths dos arquivos selecionados
  outputDir: '',            // diretório de saída
  jobs: [],                 // fila de encoding
  isEncoding: false,        // flag de estado
  config: null,             // Config carregado do backend
  history: [],              // histórico de jobs
};
```

**Por que um objeto mutável e não Redux/Immer?** O app tem ~250 linhas de
JS. Um gerenciador de estado adicionaria complexidade desproporcional ao
tamanho do código. O event loop single-threaded do browser garante que
não há race conditions reais (Tauri serializa comandos invoke).

**Helpers:**
```js
const $ = s => document.querySelector(s);
const $$ = s => document.querySelectorAll(s);
const on = (el, ev, fn) => el.addEventListener(ev, fn);
```

Mini jQuery em 3 linhas. Suficiente para este escopo.

### 6.2 Inicialização e Splash

```js
async function init() {
  // Diagnostic bar
  const d = [];
  d.push('TI:' + (!!window.__TAURI_INTERNALS__));

  // Carrega dados do backend
  S.presets = await invoke('get_presets');
  S.builtinIds = await invoke('get_builtin_preset_ids');
  S.config = await invoke('get_config');

  // Restaura tema
  if (S.config?.theme === 'Light') {
    document.documentElement.classList.add('light');
    $('#btn-theme').textContent = '☾';
  }

  // Configura output dir
  S.outputDir = S.config?.output_dir || await invoke('get_default_output_dir');

  // Setup
  updateOutputDisplay();
  await setupEvents();    // listeners de job:progress, job:done
  await setupDragDrop();  // Tauri native drag-drop
  await loadHistory();    // histórico da sidebar

  // UI
  populatePresetDropdowns('default');
  updateProfileOptions();
  renderFilters();
  renderMetadata();
  renderFiles();
  updateCrfWarning();
  hookCmdPreview();       // listeners de change/input
  updateCmdPreview();     // preview inicial
}
```

A **diagnostic bar** (`diag()`) é um `<div>` fixo no rodapé que mostra o
estado de inicialização do Tauri. Essencial para debugging porque o
WebView não tem DevTools abertos por padrão.

### 6.3 Modos Simple/Advanced

```js
function setMode(m) {
  S.mode = m;
  $('#btn-simple').className = m === 'simple' ? '...bg-rose...' : '...';
  $('#btn-advanced').className = m === 'advanced' ? '...bg-rose...' : '...';
  $('#mode-simple').classList.toggle('hidden', m !== 'simple');
  $('#mode-advanced').classList.toggle('hidden', m !== 'advanced');
  if (m === 'advanced') syncSimpleToAdvanced();
}
```

**`syncSimpleToAdvanced()`:** Copia width, height, CRF do modo simple para
os campos do advanced. Garante que o usuário não perca ajustes ao trocar
de modo.

### 6.4 Sliders e Inputs Vinculados

```js
function bindSN(rid, nid, vid, suf) {
  const r = $(rid), n = $(nid), v = vid ? $(vid) : null;
  if (!r || !n) return;
  on(r, 'input', () => {
    n.value = r.value;
    if (v) v.textContent = suf ? r.value + suf : r.value;
  });
  on(n, 'change', () => {
    let x = parseInt(n.value);
    if (isNaN(x)) return;
    x = Math.max(parseInt(r.min), Math.min(parseInt(r.max), x));
    r.value = x;
    n.value = x;
    if (v) v.textContent = suf ? x + suf : x;
    if (rid === '#a-crf') updateCrfWarning();
  });
}
```

Vincula bidirecionalmente um `<input type="range">` com um
`<input type="number">`. Quando o usuário arrasta o slider, o número
atualiza. Quando digita no número e dá Enter/ blur, o slider move.
Ambos disparam `updateCmdPreview()`.

### 6.5 Presets: CRUD e Sync

**Carregamento:**
```js
on($('#preset-select'), 'change', () => {
  const id = $('#preset-select').value;
  const p = S.presets.find(x => x.id === id);
  if (p && p.params) {
    S._syncingPreset = true;
    S.selectedPreset = id;
    S.params = { ...p.params };
    syncPresetToUI();
    S._syncingPreset = false;
  }
});
```

**`syncPresetToUI()`:** Restaura TODOS os campos do DOM a partir de
`S.params`. Usa um helper `sv(id, v)` que seta `element.value = v` se o
elemento existir.

**`_syncingPreset`:** Flag que evita que `syncPresetToUI()` dispare
`markPresetCustom()`. Sem essa flag, ao carregar um preset built-in, os
handlers de `change`/`input` imediatamente marcariam como "Custom".

**Save/Delete unificados:**
```js
async function saveCurrentPreset(selId) { /* ... */ }
async function deleteCurrentPreset(selId) {
  if (isBuiltinPreset(id)) { diag('Cannot delete built-in preset'); return; }
  /* ... */
}
```

Ambos os modos (simple e advanced) chamam as mesmas funções, passando o
seletor CSS do `<select>` como argumento.

### 6.6 Filtros e Metadata

**Filtros de vídeo:**
```js
function parseVideoFilter(s) {
  if (s === 'HFlip') return { HFlip: null };
  // ...
  const m = s.match(/^Rotate\((\d+)\)$/);
  if (m) return { Rotate: parseInt(m[1]) };
  // ...
}
```

Converte strings do `<select>` (`"Rotate(90)"`) para objetos serializáveis
(`{ Rotate: 90 }`) que o Rust desserializa para `VideoFilter::Rotate(90)`.

**Labels:**
```js
function vfLabel(f) {
  for (const k of ['HFlip', 'VFlip', ...]) {
    if (k in f) {
      switch (k) {
        case 'HFlip': return 'Flip H';
        case 'Rotate': return 'Rotate ' + f.Rotate + '°';
      }
    }
  }
}
```

Usa `for...of` + `switch` em vez de ternários encadeados porque o esbuild
não parseia `'key' in obj` dentro de ternários encadeados.

### 6.7 Command Preview

```js
let cmdPreviewTimer = null;
function debouncedCmdPreview() {
  clearTimeout(cmdPreviewTimer);
  cmdPreviewTimer = setTimeout(updateCmdPreview, 300);
}

async function updateCmdPreview() {
  const p = S.mode === 'simple' ? collectSimpleParams() : collectAdvParams();
  const cmd = await invoke('build_command_preview', { params: p });
  $('#cmd-preview').value = cmd.replace(/output\.mp4/g, 'output.' + ext);
}
```

Debounce de 300ms: ao digitar no campo extra args, não dispara 25 chamadas
IPC — espera o usuário parar de digitar.

### 6.8 Arquivos: Add, Drag-Drop, Render

**Add via diálogo nativo:**
```js
on($('#btn-add-files'), 'click', async () => {
  const sel = await dialogOpen({
    multiple: true,
    filters: [{ name: 'Media', extensions: ['mp4', 'mkv', ...] }]
  });
  if (sel) addFiles(Array.isArray(sel) ? sel : [sel]);
});
```

Usa `@tauri-apps/plugin-dialog` para abrir o seletor de arquivos nativo
do SO. **Não usa `<input type="file">`** porque no Tauri isso abriria o
diálogo do WebView, não o nativo.

**Drag-drop via API nativa do Tauri:**
```js
const wv = getCurrentWebview();
await wv.onDragDropEvent(ev => {
  if (ev.payload.type === 'drop') {
    if (ev.payload.paths?.length) addFiles(ev.payload.paths);
  }
});
```

O Tauri injeta `ev.payload.paths` com os paths reais do sistema de
arquivos (não apenas nomes de arquivo como o HTML5 DragEvent).

**Path input (fallback para WSL):**
```js
on($('#path-input'), 'keydown', e => {
  if (e.key === 'Enter') {
    addFiles([e.target.value.trim()]);
    e.target.value = '';
  }
});
```

No WSL, `rfd` (usado pelo Tauri dialog) pode falhar porque não há
`xdg-desktop-portal`. O input manual é o fallback.

### 6.9 Fila e Encoding

**Add to Queue:**
```js
on($('#btn-add-queue'), 'click', () => {
  const bp = S.mode === 'simple' ? collectSimpleParams() : collectAdvParams();
  S.files.forEach(f => {
    let out = `${S.outputDir}/${stem}.${ext}`;
    let renamed = false;
    if (out === f) {
      out = `${S.outputDir}/${stem}_converted.${ext}`;
      renamed = true;
    }
    S.jobs.push({ input: f, output: out, params: structuredClone(bp),
                   status: 'pending', progress: 0, renamed });
  });
});
```

**`structuredClone(bp)`:** Cópia profunda dos parâmetros. Essencial porque
`{ ...bp }` é shallow — arrays como `video_filters` seriam compartilhados
entre jobs.

**Proteção de source file:** Se `output === input`, renomeia automaticamente
para `_converted`. Dupla proteção: JS aqui + Rust em `start_encoding`.

**Start Encoding:**
```js
on($('#btn-start'), 'click', async () => {
  // Check overwrite
  const ex = await invoke('check_output_overwrite', { paths: outs });
  if (ex.length) {
    if (!await confirm(`Overwrite?`)) return;
  }
  // Envia para o backend
  await invoke('start_encoding', {
    jobData: jd,
    ffmpegPath: S.config?.ffmpeg_path || null
  });
});
```

### 6.10 Eventos

**job:diag:** Emitido antes do encoding começar. Mostra se o ffprobe
conseguiu detectar a duração (necessário para % real na barra).

**job:progress:** Emitido a cada ~1s durante encoding.

```js
await listen('job:progress', e => {
  const j = S.jobs.find(x => x.status === 'running' || x.status === 'pending');
  if (j) {
    const wasPending = j.status === 'pending';
    j.status = 'running';
    j.progress = d.progress_pct ?? 0;

    if (wasPending || !$('#job-running')) {
      renderQueue();  // primeiro evento: cria o DOM
    } else {
      // updates diretos no DOM (sem innerHTML!)
      const fill = $('#job-running .progress-bar-fill');
      if (fill) fill.style.width = Math.max(1, j.progress) + '%';
      const pct = $('#job-running .progress-pct');
      if (pct) pct.textContent = Math.round(j.progress) + '%';
    }
  }
});
```

**Por que update direto no DOM e não `renderQueue()` a cada evento?**
`renderQueue()` usa `innerHTML` que DESTRÓI e RECRIA os elementos. Com
eventos a cada ~100ms, o CSS `transition: width 0.3s` nunca completaria
porque o elemento seria recriado antes. O update direto preserva o elemento
e permite transições CSS suaves (se implementadas).

### 6.11 Settings, About, Tema, Atalhos

**Settings modal:** Preenche campos ao abrir, salva via `invoke('save_config')`.
Campos: ffmpeg path, output dir (com browse), default mode, default preset.

**Tema:** Toggle `class="light"` no `<html>`. Salva preferência no config.

**Atalhos de teclado:**
```js
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 'o') { $('#btn-add-files').click(); }
  if (e.ctrlKey && e.key === 'Enter') { $('#btn-start').click(); }
  if (e.key === 'Escape') { /* cancel ou fecha modais */ }
  if (e.ctrlKey && e.key === ',') { $('#btn-settings').click(); }
});
```

**Sanitização de erros:**
```js
case 'failed': {
  const safeErr = encodeURIComponent(j.error || 'unknown');
  return `<div class="err-msg" data-error="${safeErr}">
    ${decodeURIComponent(safeErr).substring(0, 500)}</div>`;
}
```

`encodeURIComponent` escapa `"`, `<`, `>`, `&` para uso seguro em atributos
HTML. Event delegation no `#queue-list` captura cliques em `.err-msg` e
copia o erro para clipboard.

---

## 7. Build Pipeline

### 7.1 npm scripts

```json
{
  "build": "V=$(node -p \"require('./package.json').version\") && sed ... && npx @tailwindcss/cli -i ui/src/styles/input.css -o ui/bundle.css && esbuild ui/src/main.js --bundle --minify --outfile=ui/bundle.js"
}
```

1. Extrai versão do `package.json`
2. `sed` substitui `v__VERSION__` no `index.html` pela versão real
3. Tailwind CLI gera `bundle.css`
4. esbuild gera `bundle.js` (minificado)

### 7.2 scripts/check.sh

```bash
cargo test -p mediaforge-core
cargo test -p mediaforge-tauri --lib
node --test crates/mediaforge-tauri/ui/test.js
cargo check -p mediaforge-core
```

Roda as 3 suites de teste + verificação de compilação. `set -euo pipefail`
faz o script abortar no primeiro erro.

### 7.3 scripts/release.sh

Fluxo completo:
1. Valida semver (`x.y.z`)
2. Verifica versão atual vs target (ascendente)
3. Bump: `sed` em Cargo.toml, tauri.conf.json, package.json
4. Commit + tag (interativo)
5. `npm run build` (frontend)
6. `cargo tauri build` (Linux)
7. `cargo tauri build --target x86_64-pc-windows-gnu` (Windows)
8. `osslsigncode` (assinar Windows)
9. Empacota: `tar.gz` (Linux), `.zip` (Windows), `.deb`

---

## 8. Cross-Compilação

**Toolchain:** `x86_64-pc-windows-gnu` (MinGW-w64)

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo tauri build --target x86_64-pc-windows-gnu
```

**Desafios:**
- `build.rs` com `windres` para compilar `resource.rc` (ícone .ico)
- `WebView2Loader.dll` deve ficar na mesma pasta que o .exe
- `CREATE_NO_WINDOW` (0x08000000) em todos os `Command::new()` para evitar
  janelas de CMD piscando

---

## 9. Estratégia de Testes

**179 testes (124 Rust core + 13 Rust tauri + 42 JS)**

**Rust core (mediaforge-core):** Testa `build_command()` para todos os
codecs (H264, H265, VP9, AV1, SVTAV1, Copy), todos os presets, todos os
formatos de pixel, filtros, movflags, trim, metadata, audio settings,
parse de progresso, filtragem de eventos de progresso (TDD).

**Rust tauri (mediaforge-tauri):** Testa comandos IPC: presets CRUD
(inclui rejeição de built-in IDs), check_output_overwrite,
build_command_preview, detect_ffmpeg, get_default_output_dir,
cancel_encoding (não panica sem encoding ativo), get_and_clear_history.

**JS:** Testa funções puras extraídas: `even()`, `normPath()`,
`parseVideoFilter()`, `parseAudioFilter()`, `clamp()`,
sanitização de erro (`encodeURIComponent` roundtrip), `vfLabel()`,
`afLabel()`.

---

## 10. Pitfalls e Lições Aprendidas

### Pitfalls de Frontend

1. **innerHTML + CSS transition = barra quebrada.** Recriar o elemento a
   cada 100ms impede transições CSS. Solução: update direto no DOM.

2. **`window.confirm()` não funciona no Tauri.** O WebView não implementa
   `confirm()` nativo. Usar `confirm()` do `@tauri-apps/plugin-dialog`.

3. **`encodeURIComponent` não escapa `'`.** Seguro para atributos
   delimitados por `"`, mas não por `'`.

4. **esbuild não parseia `'key' in obj` em ternários.** Solução: extrair
   para função com `switch`/`case`.

### Pitfalls de Backend

5. **`libopus` rejeita 44100 Hz.** Apenas 48000, 24000, 16000, 12000,
   8000. Solução: validação em 3 camadas.

6. **GIF não aceita `-c:v libx264`.** Precisa de filter chain
   `palettegen/paletteuse`. Early return em `build_command()`.

7. **VP9/AV1 não aceitam `-preset medium`.** Usam `-cpu-used N` ou
   `-deadline good`.

8. **`cargo build` != `cargo tauri build`.** Só o segundo embute
   `WebView2Loader.dll` no Windows.

### Pitfalls de Infra

9. **`bundle.css` é arquivo gerado.** Deve ser rebuildado com Tailwind
   CLI a cada mudança no `input.css`. O `npm run build` agora faz isso.

10. **Workflow de release com `read -rp` bloqueia em não-PTY.** Para
    automação, usar bump manual + `cargo tauri build` direto.
