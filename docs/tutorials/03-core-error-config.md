# Tutorial 03 — Erros e Configuração

**Objetivo:** Implementar tratamento de erros tipado com `thiserror` e
persistência de configuração com `serde` + TOML.

**Tempo estimado:** 30 minutos

---

## 1. error.rs — Erros como tipos, não strings

### Por que `thiserror`?

Rust tem `std::error::Error` como trait base para erros. Implementá-lo
manualmente exige escrever `impl Display` e `impl Error` para cada
variante — boilerplate puro. `thiserror` é uma macro que gera esse
código automaticamente.

**Referência:** https://docs.rs/thiserror

### Implementação

```rust
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum MediaForgeError {
    #[error("ffmpeg not found at `{0}`. Install ffmpeg or set the path in Settings.")]
    FfmpegNotFound(PathBuf),

    #[error("ffmpeg process failed: {0}")]
    FfmpegProcess(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Encoding cancelled by user")]
    Cancelled,

    #[error("ffmpeg timed out after {0}s without output")]
    FfmpegTimeout(u64),
}
```

**Explicação de cada variante:**

| Variante | Quando ocorre |
|----------|--------------|
| `FfmpegNotFound` | ffmpeg não encontrado no PATH nem bundled |
| `FfmpegProcess` | ffmpeg rodou mas retornou exit code != 0 |
| `Io` | Erro de sistema de arquivos (permissão, disco cheio) |
| `Config` | Erro ao ler/escrever arquivo de configuração |
| `Cancelled` | Usuário clicou Cancel durante encoding |
| `FfmpegTimeout` | ffmpeg ficou 30s sem output (hang detectado) |

**`#[from]` em `Io`:** O atributo `#[from]` implementa `From<std::io::Error>`
para `MediaForgeError`. Isso permite usar o operador `?` em funções que
retornam `Result<T, MediaForgeError>` com operações de I/O:

```rust
fn read_config() -> Result<Config, MediaForgeError> {
    let content = std::fs::read_to_string("config.toml")?;  // io::Error → MediaForgeError
    // ...
}
```

**`#[error("...")]`:** Define a mensagem de erro. `{0}` referencia o
primeiro campo da variante.

### Testes

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_display() {
        assert_eq!(
            format!("{}", MediaForgeError::Cancelled),
            "Encoding cancelled by user"
        );
    }

    #[test]
    fn ffmpeg_process_display() {
        let err = MediaForgeError::FfmpegProcess("boom".into());
        assert!(format!("{}", err).contains("boom"));
    }

    #[test]
    fn timeout_display() {
        let err = MediaForgeError::FfmpegTimeout(30);
        assert!(format!("{}", err).contains("30"));
    }
}
```

---

## 2. config.rs — Persistência de configuração

### Por que TOML?

TOML é mais legível para humanos que JSON (suporta comentários, sintaxe
mais limpa para configuração). O crate `toml` é mais leve que `serde_json`
para parsing de configuração.

**Referência TOML:** https://toml.io

### Estrutura

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::enums::{Theme, UiMode};
use crate::i18n::Language;

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

impl Default for Config {
    fn default() -> Self {
        Self {
            ffmpeg_path: None,
            language: Language::EnUs,
            theme: Theme::Dark,
            default_mode: UiMode::Simple,
            output_dir: None,
            default_preset: None,
            parallel_jobs: 1,
        }
    }
}
```

**Por que `Default` manual e não `#[derive(Default)]`?** Porque queremos
valores específicos (`Theme::Dark`, `Language::EnUs`) e não zero-values.

### Persistência

```rust
impl Config {
    /// Detecta o diretório de configuração do SO
    pub fn config_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok().map(PathBuf::from)
                .or_else(|| std::env::var("HOME").ok()
                    .map(|h| PathBuf::from(h).join(".config")))
                .unwrap_or_else(|| PathBuf::from("."))
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA")
                .ok().map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        }
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("mediaforge").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), crate::error::MediaForgeError> {
        let dir = Self::config_dir().join("mediaforge");
        std::fs::create_dir_all(&dir)?;  // '?' converte io::Error → MediaForgeError
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::MediaForgeError::Config(e.to_string()))?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}
```

**Fluxo de `load()`:**
1. Constrói o path: `~/.config/mediaforge/config.toml` (Linux) ou
   `%APPDATA%/mediaforge/config.toml` (Windows)
2. Se o arquivo existe → lê e parseia TOML
3. Se parse falhar ou arquivo não existe → retorna `Config::default()`
4. O app nunca quebra por falta de config

**Por que `dirs_next()` e não o crate `dirs`?** O crate `dirs` é pequeno
mas adiciona uma dependência desnecessária. `std::env::var` resolve com
3 linhas.

### Diretórios por plataforma:

| SO | Config dir |
|----|-----------|
| Linux | `$XDG_CONFIG_HOME` ou `~/.config` |
| Windows | `%APPDATA%` (ex: `C:\Users\Nome\AppData\Roaming`) |

### Testes

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let c = Config::default();
        assert!(c.ffmpeg_path.is_none());
        assert_eq!(c.parallel_jobs, 1);
    }

    #[test]
    fn config_path_ends_with_toml() {
        let path = Config::config_path();
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn serialize_roundtrip() {
        let c = Config::default();
        let json = serde_json::to_string(&c).unwrap();
        let c2: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(c2.parallel_jobs, c.parallel_jobs);
    }
}
```

O teste `serialize_roundtrip` usa JSON (não TOML) porque é o formato
usado na comunicação IPC com o frontend Tauri.

---

## 3. Verificação

```bash
cargo test -p mediaforge-core
```

Deve mostrar `3 tests passed` para o módulo `error` e `3 tests passed`
para o módulo `config`.

---

## Próximo passo

Tutorial 04 — `job.rs` e `preset.rs`: modelo de dados para jobs de
encoding e sistema de presets.
