# Cratua Media Forge

**Portable media converter powered by ffmpeg. Built with Rust + Tauri v2.**

Converta vídeos e áudios entre formatos com uma interface moderna,
presets inteligentes e barra de progresso em tempo real.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![Tauri v2](https://img.shields.io/badge/tauri-v2-ffc131.svg)](https://v2.tauri.app)

---

## ✨ Features

- 🎬 **Codecs:** H.264, H.265/HEVC, VP9, AV1, SVT-AV1
- 🎵 **Áudio:** AAC, MP3, Opus, Vorbis, FLAC
- 📦 **Containers:** MP4, MKV, WebM, MOV, AVI, GIF
- ⚡ **Presets:** built-in + custom (save/load)
- 📊 **Progresso em tempo real** com barra CSS
- 🎨 **Tema dark/light** com toggle
- ⌨️ **Atalhos de teclado** (Ctrl+O, Ctrl+Enter, Esc)
- 🖥️ **Cross-platform:** Linux + Windows (portable, sem instalação)
- 🔒 **Stream copy** (remux sem re-encode)

---

## 📥 Download

| Plataforma | Download |
|-----------|----------|
| Linux (x86_64) | [cratua-media-forge-v0.4.0-linux-x86_64.tar.gz](https://github.com/laucalixto/mediaforge/releases/latest) |
| Windows (x86_64) | [cratua-media-forge-v0.4.0-windows-x86_64.zip](https://github.com/laucalixto/mediaforge/releases/latest) |
| Debian/Ubuntu | [.deb package](https://github.com/laucalixto/mediaforge/releases/latest) |

ffmpeg incluso — **nada para instalar**.

### Linux portable

```bash
tar xzf cratua-media-forge-v0.4.0-linux-x86_64.tar.gz
cd cratua-media-forge-linux
./cratua-media-forge
# HiDPI: GDK_DPI_SCALE=2 ./cratua-media-forge
```

### Windows portable

Extraia o `.zip` e execute `cratua-media-forge.exe`.

Requer Windows 10+ ou [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

---

## 🛠️ Tech Stack

| Camada | Tecnologia |
|--------|-----------|
| Backend | Rust (edition 2024) |
| Frontend | Vanilla JS + Tailwind CSS v4 |
| Framework | Tauri v2 |
| Media engine | ffmpeg + ffprobe |
| Bundle JS | esbuild |
| Bundle CSS | Tailwind CLI |
| IPC | Tauri commands + events |

**Por que vanilla JS?** O app tem ~250 linhas de JS. React/Vue adicionariam
complexidade desproporcional. Tailwind v4 resolve 90% do CSS.

**Por que Tauri e não Electron?** Binário ~5MB vs ~150MB. Memória ~50MB vs
~300MB+. Sem `node_modules` de 500MB.

---

## 🚀 Dev Quick Start

```bash
# Pré-requisitos
# Rust: https://rustup.rs
# Node: https://nodejs.org (v22+)
# Linux: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev ...

git clone https://github.com/laucalixto/mediaforge.git
cd mediaforge

# Instalar deps
cd crates/mediaforge-tauri && npm install && cd ../..

# Build + test
bash scripts/check.sh

# Dev mode (hot reload)
cd crates/mediaforge-tauri && npm run dev && cargo tauri dev

# Release
bash scripts/release.sh 0.4.0
```

---

## 📁 Estrutura do Projeto

```
mediaforge/
├── Cargo.toml                  # workspace: core + tauri
├── crates/
│   ├── mediaforge-core/        # Lógica pura (zero deps de UI)
│   │   └── src/
│   │       ├── enums.rs        # 40+ tipos de domínio
│   │       ├── error.rs        # Erros tipados (thiserror)
│   │       ├── config.rs       # Persistência TOML
│   │       ├── job.rs          # Job, EncodeParams, ProgressInfo
│   │       ├── preset.rs       # Presets built-in + custom
│   │       └── ffmpeg.rs       # build_command, execução, timeout
│   └── mediaforge-tauri/       # Ponte Tauri (IPC + UI)
│       ├── src/
│       │   ├── lib.rs          # Entry point Tauri
│       │   ├── commands.rs     # 14 comandos IPC
│       │   └── main.rs         # Binary entry
│       └── ui/
│           ├── index.html      # SPA vanilla
│           ├── src/
│           │   ├── main.js     # ~250 linhas de JS
│           │   └── styles/
│           │       └── input.css  # Tailwind v4 + custom
│           └── test.js         # 42 testes unitários JS
├── scripts/
│   ├── check.sh                # Test suite completa
│   ├── release.sh              # Bump + build + package + sign
│   └── download-ffmpeg.sh      # Download ffmpeg static builds
├── docs/
│   ├── TECHNICAL.md            # Documentação técnica completa (44KB)
│   └── tutorials/              # 12 tutoriais passo a passo
│       ├── 01-setup.md
│       ├── 02-core-enums.md
│       ├── ...
│       └── 12-testing-pitfalls.md
├── vendor/ffmpeg/              # Binários ffmpeg/ffprobe (gitignored)
├── CHANGELOG.md
├── LICENSE                     # MIT
└── .editorconfig
```

---

## 📚 Documentação

| Documento | Conteúdo |
|-----------|----------|
| [`docs/TECHNICAL.md`](docs/TECHNICAL.md) | Documentação técnica completa — cada módulo, função e decisão explicados |
| [`docs/tutorials/`](docs/tutorials/) | 12 tutoriais do zero ao deploy |
| [`CHANGELOG.md`](CHANGELOG.md) | Histórico de versões |

---

## 🧪 Testes

```bash
bash scripts/check.sh
```

| Suite | Framework | Testes |
|-------|-----------|--------|
| `mediaforge-core` | `cargo test` | 124 |
| `mediaforge-tauri` | `cargo test --lib` | 13 |
| JS | `node --test` | 42 |
| **Total** | | **179** |

---

## 🤝 Contribuindo

Bug reports e PRs são bem-vindos. Veja [`docs/TECHNICAL.md`](docs/TECHNICAL.md)
para entender a arquitetura antes de contribuir.

---

## ❤ Apoie

Se este projeto te ajudou, considere [sponsor no GitHub](https://github.com/sponsors/laucalixto).

---

## 📄 Licença

MIT — veja [LICENSE](LICENSE).

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.
