# Tutorial 01 — Ambiente de Desenvolvimento

**Objetivo:** Configurar todas as ferramentas necessárias para desenvolver
o Cratua Media Forge do zero, sem depender de IA.

**Tempo estimado:** 30-45 minutos

---

## 1. Pré-requisitos

### 1.1 Rust

O backend é escrito em Rust. Instale via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Após instalar, verifique:

```bash
rustc --version   # deve mostrar >= 1.85.0 (edition 2024)
cargo --version   # gerenciador de pacotes do Rust
```

**Referência:** https://www.rust-lang.org/tools/install

### 1.2 Node.js + npm

O frontend usa Node.js para o build (esbuild, Tailwind CSS). Instale via
`nvm` (Node Version Manager):

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
source ~/.bashrc
nvm install 22
nvm use 22
node --version  # deve mostrar >= 22
npm --version
```

**Referência:** https://github.com/nvm-sh/nvm

### 1.3 Dependências do sistema (Linux)

O Tauri v2 precisa de bibliotecas do sistema para compilar o WebView:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libgtk-3-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev
```

**Referência:** https://tauri.app/v2/guides/getting-started/setup/

### 1.4 ffmpeg + ffprobe

O app empacota ffmpeg no binário, mas para desenvolvimento local:

```bash
sudo apt install ffmpeg
ffmpeg -version   # verifica instalação
ffprobe -version  # ferramenta de análise de mídia
```

**Referência:** https://ffmpeg.org/download.html

### 1.5 (Opcional) MinGW-w64 para cross-compilação Windows

```bash
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

### 1.6 (Opcional) osslsigncode para assinar binários Windows

```bash
sudo apt install osslsigncode
```

---

## 2. Criando o projeto

```bash
mkdir mediaforge
cd mediaforge
git init
```

Crie o arquivo `.gitignore`:

```gitignore
target/
dist/
node_modules/
vendor/ffmpeg/
*.pfx
```

Crie o workspace Cargo (`Cargo.toml`):

```toml
[workspace]
members = ["crates/mediaforge-core", "crates/mediaforge-tauri"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["seu-nome"]
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
log = "0.4"
env_logger = "0.11"
```

**Explicação linha a linha:**
- `members`: lista os crates do workspace. Começamos com dois.
- `resolver = "2"`: ativa o resolver de features da edição 2021,
  necessário para compilação condicional e features como `serde/derive`.
- `[workspace.package]`: metadados compartilhados por todos os crates.
  Herdam com `version.workspace = true`.
- `[workspace.dependencies]`: dependências comuns. Cada crate referencia
  com `serde.workspace = true`.

Crie a estrutura de diretórios:

```bash
mkdir -p crates/mediaforge-core/src
mkdir -p crates/mediaforge-tauri/src
mkdir -p crates/mediaforge-tauri/ui/src/styles
mkdir -p scripts
mkdir -p vendor/ffmpeg
mkdir -p assets
mkdir -p certs
```

---

## 3. Primeiro crate: mediaforge-core

`crates/mediaforge-core/Cargo.toml`:

```toml
[package]
name = "mediaforge-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
thiserror.workspace = true
uuid.workspace = true
log.workspace = true
```

`crates/mediaforge-core/src/lib.rs`:

```rust
pub mod enums;
pub mod error;
pub mod config;
pub mod i18n;
pub mod job;
pub mod preset;
pub mod ffmpeg;
```

Crie um arquivo vazio para cada módulo e um teste inicial:

```bash
touch crates/mediaforge-core/src/{enums,error,config,i18n,job,preset,ffmpeg}.rs
```

Verifique que compila:

```bash
cargo check -p mediaforge-core
```

---

## 4. Segundo crate: mediaforge-tauri

Inicialize o projeto Tauri:

```bash
cd crates/mediaforge-tauri
npm init -y
npm install --save-dev @tailwindcss/cli tailwindcss esbuild
npm install @tauri-apps/api @tauri-apps/plugin-dialog
```

Crie `crates/mediaforge-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mediaforge_tauri_lib::run()
}
```

**`windows_subsystem = "windows"`** faz o Windows não abrir uma janela de
console junto com a GUI. Só afeta builds release.

Crie `crates/mediaforge-tauri/Cargo.toml`:

```toml
[package]
name = "mediaforge-tauri"
version.workspace = true
edition.workspace = true

[lib]
name = "mediaforge_tauri_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
mediaforge-core = { path = "../mediaforge-core" }
```

**`crate-type`:** `lib` para testes unitários, `cdylib` e `staticlib`
para o Tauri (que compila como biblioteca dinâmica linkada ao WebView).

---

## 5. Primeiro build

```bash
cd ../..  # volta para raiz do projeto
cargo check --workspace
```

Deve compilar sem erros (os módulos estão vazios, mas a estrutura existe).

---

## 6. Verificação

```bash
cargo test --workspace
```

Deve mostrar `running 0 tests` — sem testes ainda, mas a estrutura compila.

---

## Próximo passo

Tutorial 02 — `enums.rs`: definir todos os tipos de domínio do app.
