# Tutorial 06 — ffmpeg: Execução, Progresso e Timeout

**Objetivo:** Implementar a execução do ffmpeg com barra de progresso em
tempo real, cancelamento e detecção de hang (timeout).

**Tempo estimado:** 1 hora

---

## 1. detect_ffmpeg()

Antes de executar, precisamos encontrar o ffmpeg:

```rust
pub fn detect_ffmpeg() -> Option<PathBuf> {
    // 1. Variável de ambiente (override manual)
    if let Ok(path) = std::env::var("MEDIAFORGE_FFMPEG_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() { return Some(p); }
    }
    // 2. Bundled com o executável
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            #[cfg(windows)] let name = "ffmpeg.exe";
            #[cfg(not(windows))] let name = "ffmpeg";
            let bundled = dir.join(name);
            if bundled.exists() { return Some(bundled); }
            let bundled_sub = dir.join("ffmpeg").join(name);
            if bundled_sub.exists() { return Some(bundled_sub); }
        }
    }
    // 3. Sistema PATH
    which::which("ffmpeg").ok()
}
```

---

## 2. probe_duration()

Usa `ffprobe` para descobrir a duração do vídeo ANTES de começar:

```rust
pub fn probe_duration(input: &Path, ffmpeg_path: Option<&Path>) -> Option<u64> {
    // Encontra ffprobe ao lado do ffmpeg
    let ffprobe = find_ffprobe(ffmpeg_path)?;

    let output = Command::new(&ffprobe)
        .args(["-v", "quiet", "-show_entries", "format=duration",
               "-of", "csv=p=0"])
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output().ok()?;

    let secs: f64 = String::from_utf8_lossy(&output.stdout)
        .trim().parse().ok()?;
    Some((secs * 1_000_000.0) as u64)  // segundos → microssegundos
}
```

**Comando equivalente:**
```
ffprobe -v quiet -show_entries format=duration -of csv=p=0 video.mp4
→ 120.500000
```

---

## 3. parse_progress_line()

O ffmpeg com `-progress pipe:2` escreve no stderr:

```
frame=100
fps=25.0
out_time_us=4000000
speed=2.5x
progress=continue
```

Parse:

```rust
pub fn parse_progress_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, '=');
    let key = parts.next()?.trim().to_string();
    let value = parts.next()?.trim().to_string();
    Some((key, value))
}
```

---

## 4. run_with_progress_and_ffmpeg_cancellable()

**Esta é a função mais complexa do app.** Arquitetura:

```
┌──────────────┐  mpsc::channel  ┌──────────────┐
│ Reader Thread │ ──── lines ───► │ Main Thread   │
│ (stderr loop) │                 │ (recv_timeout)│
└──────────────┘                 └──────┬───────┘
                                       │
                         ┌─────────────▼──────────┐
                         │ parse → progress_pct   │
                         │ emit → on_progress()   │
                         └────────────────────────┘
```

```rust
pub fn run_with_progress_and_ffmpeg_cancellable<F>(
    params: &EncodeParams,
    input: &Path, output: &Path,
    ffmpeg_path: Option<&Path>,
    cancel_flag: Option<Arc<AtomicBool>>,
    total_duration_us: Option<u64>,
    mut on_progress: F,
) -> Result<String, MediaForgeError>
where F: FnMut(&ProgressInfo),
{
    let resolved = ffmpeg_path.or_else(|| detect_ffmpeg().as_deref());
    let mut cmd = build_command_with_ffmpeg(params, input, output, resolved);
    cmd.stderr(Stdio::piped()).stdin(Stdio::null());

    #[cfg(windows)] {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()
        .map_err(|e| MediaForgeError::FfmpegProcess(format!("spawn: {e}")))?;
    let stderr = child.stderr.take().unwrap();

    // Thread separada para ler stderr → channel
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let reader = std::io::BufReader::new(stderr);
    let reader_handle = std::thread::spawn(move || {
        for line in reader.lines() {
            if tx.send(line.unwrap_or_default()).is_err() { break; }
        }
    });

    let mut stderr_lines = Vec::new();
    const TIMEOUT: u64 = 30;

    loop {
        match rx.recv_timeout(Duration::from_secs(TIMEOUT)) {
            Ok(line) => {
                // Cancelamento
                if let Some(ref flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        let _ = child.kill(); let _ = child.wait();
                        drop(rx); let _ = reader_handle.join();
                        return Err(MediaForgeError::Cancelled);
                    }
                }

                stderr_lines.push(line.clone());

                // Parse e callback de progresso
                if let Some((key, value)) = parse_progress_line(&line) {
                    let mut info = ProgressInfo::default();
                    let mut emit = true;
                    match key.as_str() {
                        "out_time_us" => {
                            info.out_time_us = value.parse().unwrap_or(0);
                            if let Some(total) = total_duration_us {
                                if total > 0 {
                                    info.progress_pct =
                                        (info.out_time_us as f64 / total as f64 * 100.0)
                                        .min(99.9);
                                }
                            }
                        }
                        "progress" => {
                            if value == "end" { info.progress_pct = 100.0; }
                            else { emit = false; }
                        }
                        _ => emit = false,  // frame, fps, speed → skip
                    }
                    if emit { on_progress(&info); }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // 30s sem output → ffmpeg travou
                let _ = child.kill(); let _ = child.wait();
                drop(rx); let _ = reader_handle.join();
                return Err(MediaForgeError::FfmpegTimeout(TIMEOUT));
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = child.wait()
        .map_err(|e| MediaForgeError::FfmpegProcess(e.to_string()))?;

    if !status.success() {
        let tail: String = stderr_lines.iter().rev().take(20)
            .collect::<Vec<_>>().into_iter().rev()
            .cloned().collect::<Vec<_>>().join("\n");
        return Err(MediaForgeError::FfmpegProcess(format!(
            "exit code {}\nStderr:\n{}",
            status.code().unwrap_or(-1), tail
        )));
    }

    Ok(cmd_str)
}
```

**Por que thread separada + channel?** `reader.lines()` bloqueia até a
próxima linha. Se o ffmpeg travar, `lines()` bloqueia para sempre.
Com channel, a thread principal faz `recv_timeout(30s)` e detecta o hang.

**Filtragem de progresso:** Apenas `out_time_us` e `progress=end` disparam
callbacks. Linhas como `frame=100`, `speed=2.5x` são ignoradas porque
carregam `progress_pct=0.0` e sobrescreveriam o valor real na UI.

---

## 5. Testes de progresso

```rust
#[test]
fn progress_filter_only_emits_out_time_and_end() {
    let lines = [
        "frame=0", "fps=0.0",
        "out_time_us=0",
        "frame=100",
        "out_time_us=2500000",   // 25%
        "speed=3.5x",
        "out_time_us=5000000",   // 50%
        "progress=continue",     // NÃO emitido
        "out_time_us=9990000",   // 99.9%
        "progress=end",          // 100%
    ];
    let (count, pcts) = simulate_progress_events(&lines, Some(10_000_000));
    assert_eq!(count, 5);  // 4 out_time_us + 1 progress=end
    assert_eq!(pcts, vec![0.0, 25.0, 50.0, 99.9, 100.0]);
}
```

---

## 6. Verificação

```bash
cargo test -p mediaforge-core
```

~90 testes passando (ffmpeg build + progress + probe).

---

## Próximo passo

Tutorial 07 — Tauri v2: integração do backend Rust com o frontend web.
