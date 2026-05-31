# Tutorial 11 — Build, Cross-Compilação e Release

**Objetivo:** Entender o pipeline completo de build, cross-compilação
Linux → Windows, e o processo de release com assinatura de binários.

---

## 1. Pipeline de Build

```
npm run build          (1) Frontend: Tailwind CSS + esbuild JS
       │
       ▼
cargo tauri build      (2) Backend: compila Rust + embute ui/
       │
       ├── target/release/mediaforge-tauri    (binário Linux)
       ├── target/release/bundle/deb/*.deb    (pacote Debian)
       └── target/release/bundle/rpm/*.rpm    (pacote RPM)
```

**`cargo tauri build`** faz duas coisas:
1. Roda `beforeBuildCommand` (`npm run build`) para gerar `bundle.js` e `bundle.css`
2. Compila o código Rust em modo release com LTO (Link-Time Optimization)
3. Embute a pasta `ui/` no binário
4. Cria pacotes específicos da plataforma

---

## 2. scripts/check.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== Rust Tests (core) ==="
cargo test -p mediaforge-core

echo "=== Rust Tests (tauri) ==="
cargo test -p mediaforge-tauri --lib

echo "=== JS Tests ==="
node --test crates/mediaforge-tauri/ui/test.js

echo "=== Cargo Check ==="
cargo check -p mediaforge-core

echo "=== All checks passed ==="
```

**`set -euo pipefail`:**
- `-e`: aborta no primeiro erro
- `-u`: erro ao usar variável não definida
- `-o pipefail`: o exit code do pipe é o do último comando que falhou

---

## 3. scripts/release.sh

Fluxo completo (215 linhas):

### Passo 1: Bump de versão

```bash
VERSION="0.4.0"
CURRENT=$(grep -oP '^version\s*=\s*"\K[^"]+' Cargo.toml | head -1)

# Valida ascending
if [ "$CURRENT" = "$VERSION" ]; then
  echo "Version is already $VERSION"
fi

# Bump
sed -i "s/^version = \"$CURRENT\"/version = \"$VERSION\"/" Cargo.toml
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/" tauri.conf.json
sed -i "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/" package.json

# Commit + tag
git add Cargo.toml crates/mediaforge-tauri/tauri.conf.json crates/mediaforge-tauri/package.json
git commit -m "chore: release v$VERSION"
git tag "v$VERSION"
```

### Passo 2: Build frontend

```bash
cd crates/mediaforge-tauri
npm run build --silent
```

### Passo 3: Build Linux

```bash
cargo tauri build
strip target/release/mediaforge-tauri

# Empacota
mkdir -p dist/cratua-media-forge-linux/ffmpeg
cp target/release/mediaforge-tauri dist/cratua-media-forge-linux/cratua-media-forge
cp vendor/ffmpeg/ffmpeg dist/cratua-media-forge-linux/ffmpeg/
cp LICENSE dist/cratua-media-forge-linux/
tar czf dist/cratua-media-forge-v0.4.0-linux-x86_64.tar.gz cratua-media-forge-linux
```

### Passo 4: Cross-compile Windows

```bash
cargo tauri build --target x86_64-pc-windows-gnu
x86_64-w64-mingw32-strip target/x86_64-pc-windows-gnu/release/mediaforge-tauri.exe

mkdir -p dist/cratua-media-forge-windows/ffmpeg
cp target/x86_64-pc-windows-gnu/release/mediaforge-tauri.exe dist/cratua-media-forge-windows/cratua-media-forge.exe
cp vendor/ffmpeg/ffmpeg.exe dist/cratua-media-forge-windows/ffmpeg/
cp vendor/ffmpeg/ffprobe.exe dist/cratua-media-forge-windows/ffmpeg/
cp target/x86_64-pc-windows-gnu/release/WebView2Loader.dll dist/cratua-media-forge-windows/
cp LICENSE dist/cratua-media-forge-windows/
zip -r dist/cratua-media-forge-v0.4.0-windows-x86_64.zip cratua-media-forge-windows
```

**⚠ DLL location:** `WebView2Loader.dll` DEVE ficar na mesma pasta que o
`.exe`. Windows NÃO busca DLLs em subdiretórios.

### Passo 5: Assinar Windows

```bash
osslsigncode sign \
  -h sha256 \
  -pkcs12 certs/cratua-cert-dev.pfx -pass 'senha' \
  -n "Cratua Media Forge" \
  -t "http://timestamp.digicert.com" \
  -in dist/cratua-media-forge-windows/cratua-media-forge.exe \
  -out dist/cratua-media-forge-windows/cratua-media-forge.exe.signed
mv dist/cratua-media-forge-windows/cratua-media-forge.exe.signed \
   dist/cratua-media-forge-windows/cratua-media-forge.exe
```

**`-h sha256`** é obrigatório para Windows 10/11.

---

## 4. Cross-Compilação: Detalhes Técnicos

### Toolchain

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
```

### build.rs — Dual purpose

```rust
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let host = std::env::var("HOST").unwrap_or_default();

    // Compilar resource.rc → resource.o (ícone do .exe)
    if target.contains("windows") {
        let status = Command::new("x86_64-w64-mingw32-windres")
            .arg("resource.rc").arg("resource.o")
            .status();
        if let Ok(s) = s {
            if s.success() {
                println!("cargo:rustc-link-arg=resource.o");
            }
        }
    }
}
```

### CREATE_NO_WINDOW

Todo `Command::new("ffmpeg")` e `Command::new("ffprobe")` deve ter:

```rust
#[cfg(target_os = "windows")]
{
    cmd.creation_flags(0x08000000);  // CREATE_NO_WINDOW
}
```

Sem isso, cada processo ffmpeg abre uma janela CMD visível.

---

## 5. Verificação

```bash
bash scripts/check.sh     # 179 testes
bash scripts/release.sh 0.4.0  # gera dist/
ls -lh dist/
# cratua-media-forge-v0.4.0-linux-x86_64.tar.gz
# cratua-media-forge-v0.4.0-windows-x86_64.zip
# Cratua Media Forge_0.4.0_amd64.deb
```

---

## Próximo passo

Tutorial 12 — Estratégia de Testes e Pitfalls.
