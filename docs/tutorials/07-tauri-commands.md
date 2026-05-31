# Tutorial 07 — Tauri v2: Ponte IPC (lib.rs + commands.rs)

**Objetivo:** Conectar o backend Rust (`mediaforge-core`) ao frontend
JavaScript via Tauri v2 commands e events.

**Referência oficial:** https://tauri.app/v2/guides/inter-process-communication/

---

## 1. lib.rs — Entry Point do Tauri

```rust
mod commands;
use commands::*;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_presets,
            get_builtin_preset_ids,
            create_preset, delete_preset,
            get_config, save_config,
            detect_ffmpeg, build_command_preview,
            check_output_overwrite,
            get_default_output_dir,
            start_encoding, cancel_encoding,
            get_history, clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
```

**`tauri::Builder::default()`** — construtor padrão.

**`.plugin(...)`** — registra plugins oficiais:
- `tauri_plugin_shell`: abre URLs no browser (`window.open()`)
- `tauri_plugin_dialog`: diálogos nativos de arquivo

**`generate_handler![]`** — macro que registra funções Rust como comandos
invocáveis do frontend via `invoke('nome', { args })`.

**`generate_context!()`** — macro que lê `tauri.conf.json` em tempo de
compilação e embute as configurações no binário.

---

## 2. commands.rs — Comandos IPC

Cada função anotada com `#[tauri::command]` vira um endpoint:

### Presets

```rust
#[tauri::command]
pub fn get_presets() -> Vec<Preset> {
    let mut presets = preset::builtin_presets();
    presets.extend(load_custom_presets());
    presets
}

#[tauri::command]
pub fn get_builtin_preset_ids() -> Vec<String> {
    preset::builtin_presets().iter().map(|p| p.id.clone()).collect()
}

#[tauri::command]
pub fn create_preset(preset: Preset) -> Result<(), String> {
    let builtin_ids: Vec<&str> = preset::builtin_presets()
        .iter().map(|p| p.id.as_str()).collect();
    if builtin_ids.contains(&preset.id.as_str()) {
        return Err(format!("Cannot overwrite built-in preset"));
    }
    let mut custom = load_custom_presets();
    custom.retain(|p| p.id != preset.id);
    custom.push(preset);
    save_custom_presets(&custom);
    Ok(())
}
```

**Persistência de presets custom:** Arquivo JSON em
`~/.config/mediaforge/presets.json`. Protegido por `PRESET_LOCK: Mutex<()>`.

### Config

```rust
#[tauri::command]
pub fn get_config() -> Config { Config::load() }

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}
```

### ffmpeg

```rust
#[tauri::command]
pub fn detect_ffmpeg(custom_path: Option<String>) -> Option<String> {
    if let Some(ref p) = custom_path {
        if PathBuf::from(p).exists() { return Some(p.clone()); }
    }
    ffmpeg::detect_ffmpeg().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn build_command_preview(params: EncodeParams) -> String {
    ffmpeg::command_to_string(&params,
        Path::new("input.mp4"), Path::new("output.mp4"))
}
```

### Encoding (o comando mais complexo)

```rust
#[tauri::command]
pub async fn start_encoding(
    job_data: Vec<JobData>,
    ffmpeg_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String>
```

**Fluxo completo:**
1. Converte `JobData` (leve, serializável) → `Vec<Job>` (com PathBuf, Uuid)
2. Validações (output ≠ input, Dragon Trap)
3. Cria diretórios de saída
4. Probe durações com ffprobe → emite `job:diag`
5. Cria `Arc<AtomicBool>` para cancelamento
6. Spawna thread que executa ffmpeg com callback:
   - `on_progress` → emite `job:progress` com `progress_pct`
   - Ao final → emite `job:done` com status
   - Append ao history (max 200 entradas, lock-protected)
   - Sempre limpa cancel flag (mesmo em pânico)

**Cancelamento:**

```rust
static CANCEL_FLAG: OnceLock<Mutex<Option<Arc<AtomicBool>>>> = OnceLock::new();

#[tauri::command]
pub fn cancel_encoding() {
    if let Ok(guard) = CANCEL_FLAG.get_or_init(|| Mutex::new(None)).lock() {
        if let Some(ref flag) = *guard {
            flag.store(true, Ordering::SeqCst);
        }
    }
}
```

`Arc<AtomicBool>` é compartilhado via `OnceLock`. A thread de encoding
verifica `flag.load(Ordering::Relaxed)` a cada linha do stderr.

### History

```rust
#[tauri::command]
pub fn get_history() -> Vec<HistoryEntry> { load_history() }

#[tauri::command]
pub fn clear_history() {
    let _ = std::fs::remove_file(&history_path());
}
```

---

## 3. main.rs — Binary Entry

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mediaforge_tauri_lib::run()
}
```

---

## 4. Testes

```rust
#[test]
fn create_preset_rejects_builtin_id() {
    let p = Preset { id: "default".into(), ... };
    assert!(create_preset(p).is_err());
}

#[test]
fn create_and_delete_custom_preset() {
    let p = Preset { id: "__test__".into(), ... };
    create_preset(p).unwrap();
    assert!(get_presets().iter().any(|p| p.id == "__test__"));
    delete_preset("__test__".into()).unwrap();
}

#[test]
fn cancel_encoding_no_active() {
    cancel_encoding();  // não deve panicar
}
```

---

## 5. Verificação

```bash
cargo test -p mediaforge-tauri --lib   # 13 testes
```

---

## Próximo passo

Tutorial 08 — Frontend: HTML, CSS e Tailwind v4.
