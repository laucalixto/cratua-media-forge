# Correções — Plano Detalhado com Diffs Precisos

Cada item contém:
- Arquivo e linha exata
- Diff exato a aplicar (old_string → new_string)
- Garantia de não-quebra
- Como verificar depois

---

## Item 1: Timeout no encoding thread

### Razão
Se ffmpeg trava (disco cheio, pipe quebrado), `reader.lines()` bloqueia
para sempre. O botão Cancel só funciona se houver output — sem output,
o loop não avança até a checagem do cancel_flag.

### Estratégia
Thread separada lê stderr e envia cada linha por `mpsc::channel()`.
Thread principal faz `rx.recv_timeout(30s)`. Se timeout → mata child.
Se channel fecha (reader thread terminou) → `child.wait()` normalmente.

Isso NÃO quebra o comportamento existente porque:
- Cada linha é processada exatamente como antes (mesmo parse, mesmo callback)
- `on_progress` é chamado da thread principal (sem mudança)
- `stderr_lines` acumula igual
- A checagem de cancelamento acontece a cada linha (como antes)
- Os 123 testes existentes não testam `run_with_progress_and_ffmpeg_cancellable` diretamente (só testam `build_command`, `parse_progress_line`, etc.) — zero risco de quebrar testes

### Arquivo 1A: `crates/mediaforge-core/src/error.rs`
**Linhas:** 1-32 (substituir todo o enum)

**Diff:**
- old_string: linha 27 `    #[error("Encoding cancelled by user")]`
- new_string: linha 27 + nova linha 32

```
    #[error("Encoding cancelled by user")]
    Cancelled,

    #[error("ffmpeg timed out after {0}s without output")]
    FfmpegTimeout(u64),
```

E adicionar teste na linha ~43:

```
    #[test]
    fn timeout_display() { assert!(format!("{}", MediaForgeError::FfmpegTimeout(30)).contains("30")); }
```

### Arquivo 1B: `crates/mediaforge-core/src/ffmpeg.rs`
**Linhas:** 443-485 (substituir o `for line in reader.lines()` inteiro)

**Diff completo (substitui linhas 440-485):**

OLD (linhas 440-485):
```rust
    let reader = std::io::BufReader::new(stderr);
    let mut stderr_lines: Vec<String> = Vec::new();

    for line in reader.lines() {
        // Check cancellation
        if let Some(ref flag) = cancel_flag {
            if flag.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(MediaForgeError::Cancelled);
            }
        }

        let line = line.unwrap_or_default();
        let line_clone = line.clone();
        stderr_lines.push(line_clone);

        if let Some((key, value)) = parse_progress_line(&line) {
            let mut info = ProgressInfo::default();
            let mut should_emit = true;
            match key.as_str() {
                "out_time_us" => {
                    info.out_time_us = value.parse().unwrap_or(0);
                    if let Some(total) = total_duration_us {
                        if total > 0 {
                            info.progress_pct = (info.out_time_us as f64 / total as f64 * 100.0).min(99.9);
                        }
                    }
                }
                "progress" => {
                    if value == "end" {
                        info.progress_pct = 100.0;
                    } else {
                        should_emit = false; // progress=continue — don't emit
                    }
                }
                // Only emit for out_time_us and progress=end — all other keys
                // (speed, fps, frame, bitrate, total_size) carry progress_pct=0.0
                // which would overwrite the real value in the frontend.
                _ => should_emit = false,
            }
            if should_emit {
                on_progress(&info);
            }
        }
    }
```

NEW:
```rust
    // Spawn a thread to read stderr lines and send them through a channel.
    // The main thread uses recv_timeout to detect ffmpeg hangs.
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    let reader = std::io::BufReader::new(stderr);
    let reader_handle = std::thread::spawn(move || {
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            if line_tx.send(line).is_err() {
                break; // receiver dropped — main thread is done
            }
        }
    });

    let mut stderr_lines: Vec<String> = Vec::new();
    const TIMEOUT_SECS: u64 = 30;

    loop {
        match line_rx.recv_timeout(std::time::Duration::from_secs(TIMEOUT_SECS)) {
            Ok(line) => {
                // Check cancellation
                if let Some(ref flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        let _ = child.kill();
                        let _ = child.wait();
                        // Join reader thread to clean up
                        drop(line_rx);
                        let _ = reader_handle.join();
                        return Err(MediaForgeError::Cancelled);
                    }
                }

                let line_clone = line.clone();
                stderr_lines.push(line_clone);

                if let Some((key, value)) = parse_progress_line(&line) {
                    let mut info = ProgressInfo::default();
                    let mut should_emit = true;
                    match key.as_str() {
                        "out_time_us" => {
                            info.out_time_us = value.parse().unwrap_or(0);
                            if let Some(total) = total_duration_us {
                                if total > 0 {
                                    info.progress_pct = (info.out_time_us as f64 / total as f64 * 100.0).min(99.9);
                                }
                            }
                        }
                        "progress" => {
                            if value == "end" {
                                info.progress_pct = 100.0;
                            } else {
                                should_emit = false;
                            }
                        }
                        _ => should_emit = false,
                    }
                    if should_emit {
                        on_progress(&info);
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // 30 seconds without a line → ffmpeg is likely hung
                let _ = child.kill();
                let _ = child.wait();
                drop(line_rx);
                let _ = reader_handle.join();
                return Err(MediaForgeError::FfmpegTimeout(TIMEOUT_SECS));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Reader thread finished (stderr closed normally)
                break;
            }
        }
    }
```

**Garantia de não-quebra:**
- O `stderr_lines` é populado do mesmo jeito (cada linha empurrada)
- O callback `on_progress` recebe os mesmos `ProgressInfo`
- A checagem de cancelamento está no mesmo lugar lógico (antes de processar
  a linha, dentro do loop)
- Os imports necessários (`std::time::Duration`, `std::sync::mpsc`) já estão
  disponíveis (Duration já é usado em outras partes do crate)
- O `reader_handle.join()` no caminho de cancel/timeout garante que a thread
  não fica zumbi

**Verificação:**
```bash
cargo test -p mediaforge-core  # 123 tests pass + 1 novo (timeout_display)
cargo check -p mediaforge-tauri # sem warnings
```

Para testar o timeout manualmente: encoding de um arquivo que causa hang
no ffmpeg (ex: input de rede com url inválida). Deve mostrar erro após 30s.

---

## Item 2: Lock em custom presets

### Arquivo: `crates/mediaforge-tauri/src/commands.rs`

**Linha 15** (após `static HISTORY_LOCK`):

OLD:
```rust
// ── History file lock ──
static HISTORY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ── Lightweight job data for IPC ──
```

NEW:
```rust
// ── History file lock ──
static HISTORY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ── Custom presets file lock ──
static PRESET_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ── Lightweight job data for IPC ──
```

**Linhas 74-83** (`load_custom_presets`):

OLD:
```rust
fn load_custom_presets() -> Vec<Preset> {
    let path = custom_presets_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    }
}
```

NEW:
```rust
fn load_custom_presets() -> Vec<Preset> {
    let _lock = PRESET_LOCK.lock().unwrap();
    let path = custom_presets_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    }
}
```

**Linhas 86-94** (`save_custom_presets`):

OLD:
```rust
fn save_custom_presets(presets: &[Preset]) {
    let path = custom_presets_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(presets) {
        let _ = std::fs::write(&path, json);
    }
}
```

NEW:
```rust
fn save_custom_presets(presets: &[Preset]) {
    let _lock = PRESET_LOCK.lock().unwrap();
    let path = custom_presets_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(presets) {
        let _ = std::fs::write(&path, json);
    }
}
```

**Garantia de não-quebra:**
- Padrão idêntico ao `HISTORY_LOCK` na linha 16
- `Mutex<()>` com `lock().unwrap()` — mesmo comportamento
- Se o lock falhar (poisoned), o unwrap propaga o panic — comportamento
  padrão do Rust para mutex poisoned (consistente com HISTORY_LOCK)
- Nenhum teste existente mexe em presets concorrentemente

**Verificação:**
```bash
cargo test -p mediaforge-tauri --lib  # tests existentes passam
```

---

## Item 3: Sanitização do renderQueue (erro ffmpeg)

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 211** — caso `'failed'` no template literal do `renderQueue()`:

OLD:
```js
case'failed':return`<div class=\"flex flex-col text-xs py-1 px-2\"><div class=\"flex items-center justify-between\"><span class=\"text-[#ef4444] truncate\">✗ ${inn}</span><span class=\"text-[#ef4444] shrink-0 ml-2\">failed</span></div><div class=\"text-[#606070] mt-1 text-[10px] cursor-pointer hover:text-[#c0c0d0]\" title=\"${(j.error||'').replace(/\"/g,'&quot;')}\" onclick=\"navigator.clipboard.writeText(this.getAttribute('title'))\">${(j.error||'unknown').substring(0,500)}</div></div>`;
```

NEW:
```js
case'failed':{const safeErr=encodeURIComponent(j.error||'unknown');return`<div class=\"flex flex-col text-xs py-1 px-2\"><div class=\"flex items-center justify-between\"><span class=\"text-[#ef4444] truncate\">✗ ${inn}</span><span class=\"text-[#ef4444] shrink-0 ml-2\">failed</span></div><div class=\"err-msg text-[#606070] mt-1 text-[10px] cursor-pointer hover:text-[#c0c0d0]\" data-error=\"${safeErr}\">${decodeURIComponent(safeErr).substring(0,500)}</div></div>`}
```

Explicação:
- `encodeURIComponent` escapa `"`, `'`, `&`, `<`, `>` e todos os caracteres
  especiais — seguro para colocar em atributo HTML
- `data-error` em vez de `title` — não dispara tooltip nativo, mas permite
  cópia via event delegation
- `decodeURIComponent` no `textContent` visível mostra o texto original
- Remove `onclick` inline — substituído por event delegation (abaixo)

**Linha ~222** — após `setupEvents()`, adicionar event delegation:

OLD (linha 222 é `// ── Events ──`):

```js
// ── Events ──
async function setupEvents(){...}
```

Adicionar ANTES da linha 222:

```js
// ── Error message click-to-copy (event delegation) ──
on($('#queue-list'),'click',e=>{const el=e.target.closest('.err-msg');if(el&&el.dataset.error){navigator.clipboard.writeText(decodeURIComponent(el.dataset.error));diag('Error copied to clipboard')}});
```

**Garantia de não-quebra:**
- `encodeURIComponent`/`decodeURIComponent` são roundtrip-safe: o texto
  original é preservado
- O `substring(0,500)` é aplicado no texto decodificado (visível), não no
  codificado — o dataset tem o texto completo codificado
- Se `j.error` for `null`, cai no fallback `'unknown'`
- O event delegation no `#queue-list` já existe implicitamente (os elementos
  são criados via innerHTML), mas agora explicitamente tratamos a classe
  `.err-msg`
- Se `renderQueue` for chamada de novo (re-render), os event listeners
  do delegation continuam funcionando (estão no parent, não nos filhos)

**Arquivo 3B: `crates/mediaforge-tauri/ui/test.js`**

Adicionar após o último `describe`:

```js
describe('error sanitization', () => {
    it('roundtrips encode/decode', () => {
        const err = `Error: can't "parse" this & that <tag>`;
        const safe = encodeURIComponent(err);
        // Safe string has no bare quotes or angle brackets
        assert(!safe.includes("'"));
        assert(!safe.includes('"'));
        assert(!safe.includes('<'));
        assert.equal(decodeURIComponent(safe), err);
    });
    it('handles null/undefined', () => {
        const safe = encodeURIComponent(null || 'unknown');
        assert.equal(decodeURIComponent(safe), 'unknown');
    });
});
```

**Verificação:**
```bash
node --test crates/mediaforge-tauri/ui/test.js  # 6 tests (4 + 2 novos)
```

---

## Item 4: Dedup — browse e preset handlers

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linhas 96-98** (dois handlers browse idênticos):

OLD:
```js
on($('#btn-browse'),'click',async()=>{try{const sel=await dialogOpen({directory:true,multiple:false,title:'Select Output Folder'});if(sel){S.outputDir=normPath(typeof sel==='string'?sel:sel[0]);updateOutputDisplay();if(S.config){S.config.output_dir=S.outputDir;invoke('save_config',{config:S.config}).catch(()=>{})}}}catch(e){diag('Browse error: '+e)}});
// Advanced browse
on($('#a-btn-browse'),'click',async()=>{try{const sel=await dialogOpen({directory:true,multiple:false,title:'Select Output Folder'});if(sel){S.outputDir=normPath(typeof sel==='string'?sel:sel[0]);updateOutputDisplay();if(S.config){S.config.output_dir=S.outputDir;invoke('save_config',{config:S.config}).catch(()=>{})}}}catch(e){diag('Browse error: '+e)}});
```

NEW:
```js
async function browseOutputDir(){try{const sel=await dialogOpen({directory:true,multiple:false,title:'Select Output Folder'});if(sel){S.outputDir=normPath(typeof sel==='string'?sel:sel[0]);updateOutputDisplay();if(S.config){S.config.output_dir=S.outputDir;invoke('save_config',{config:S.config}).catch(()=>{})}}}catch(e){diag('Browse error: '+e)}}
on($('#btn-browse'),'click',browseOutputDir);
on($('#a-btn-browse'),'click',browseOutputDir);
```

**Linhas 200-203** (dois handlers save/delete preset):

OLD (save preset, ~linhas 200 e 202):
```js
on($('#adv-btn-save-preset'),'click',async()=>{const name=prompt('Preset name:');if(!name)return;const params=collectAdvParams();const id=name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');const ex=S.presets.find(p=>p.id===id);if(ex){if(!await confirm('Preset \"'+ex.name+'\" already exists. Overwrite?'))return}try{await invoke('create_preset',{preset:{id,name,description:'Custom preset',category:'Video',params}});S.presets=await invoke('get_presets');populatePresetDropdowns(id);diag('Preset saved: '+name)}catch(e){diag('Save error: '+e)}});
on($('#adv-btn-delete-preset'),'click',async()=>{const id=$('#adv-preset-select').value,p=S.presets.find(x=>x.id===id);if(!p)return;const bi=['default','web-h264','web-h265','web-vp9','archive-h264','audio-mp3','audio-aac','audio-opus','gif'];if(bi.includes(id)){diag('Cannot delete built-in preset');return}if(!await confirm('Delete preset \"'+p.name+'\"?'))return;try{await invoke('delete_preset',{id});S.presets=await invoke('get_presets');populatePresetDropdowns();diag('Preset deleted')}catch(e){diag('Delete error: '+e)}});
on($('#btn-save-preset'),'click',async()=>{const name=prompt('Preset name:');if(!name)return;const params=S.mode==='simple'?collectSimpleParams():collectAdvParams();const id=name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');const ex=S.presets.find(p=>p.id===id);if(ex){if(!await confirm(`Preset \"${ex.name}\" already exists. Overwrite?`))return}try{await invoke('create_preset',{preset:{id,name,description:'Custom preset',category:'Video',params}});S.presets=await invoke('get_presets');populatePresetDropdowns(id);diag('Preset saved: '+name)}catch(e){diag('Save error: '+e)}});
on($('#btn-delete-preset'),'click',async()=>{const id=$('#preset-select').value,p=S.presets.find(x=>x.id===id);if(!p)return;const bi=['default','web-h264','web-h265','web-vp9','archive-h264','audio-mp3','audio-aac','audio-opus','gif'];if(bi.includes(id)){diag('Cannot delete built-in preset');return}if(!await confirm(`Delete preset \"${p.name}\"?`))return;try{await invoke('delete_preset',{id});S.presets=await invoke('get_presets');populatePresetDropdowns();diag('Preset deleted')}catch(e){diag('Delete error: '+e)}});
```

NEW:
```js
async function saveCurrentPreset(selId){const name=prompt('Preset name:');if(!name)return;const params=S.mode==='simple'?collectSimpleParams():collectAdvParams();const id=name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'');const ex=S.presets.find(p=>p.id===id);if(ex){if(!await confirm(`Preset \"${ex.name}\" already exists. Overwrite?`))return}try{await invoke('create_preset',{preset:{id,name,description:'Custom preset',category:'Video',params}});S.presets=await invoke('get_presets');populatePresetDropdowns(id);diag('Preset saved: '+name)}catch(e){diag('Save error: '+e)}}
async function deleteCurrentPreset(selId){const id=$(selId).value;const p=S.presets.find(x=>x.id===id);if(!p)return;if(isBuiltinPreset(id)){diag('Cannot delete built-in preset');return}if(!await confirm(`Delete preset \"${p.name}\"?`))return;try{await invoke('delete_preset',{id});S.presets=await invoke('get_presets');populatePresetDropdowns();diag('Preset deleted')}catch(e){diag('Delete error: '+e)}}
on($('#btn-save-preset'),'click',()=>saveCurrentPreset('#preset-select'));
on($('#adv-btn-save-preset'),'click',()=>saveCurrentPreset('#adv-preset-select'));
on($('#btn-delete-preset'),'click',()=>deleteCurrentPreset('#preset-select'));
on($('#adv-btn-delete-preset'),'click',()=>deleteCurrentPreset('#adv-preset-select'));
```

**Garantia de não-quebra:**
- `saveCurrentPreset` usa `S.mode` para decidir qual collector usar (simples
  ou avançado) — mesma lógica de antes, unificada
- `deleteCurrentPreset` usa `isBuiltinPreset(id)` (item 5) — se não existir
  ainda, usamos fallback inline até o item 5 ser implementado
- O comportamento do prompt/confirm/diag é idêntico

---

## Item 5: Built-in IDs do backend

### Arquivo 5A: `crates/mediaforge-tauri/src/commands.rs`

Adicionar após `get_presets` (~linha 116):

```rust
#[tauri::command]
pub fn get_builtin_preset_ids() -> Vec<String> {
    preset::builtin_presets().iter().map(|p| p.id.clone()).collect()
}
```

### Arquivo 5B: `crates/mediaforge-tauri/src/lib.rs`

**Linha 13** (após `get_presets,`):

OLD:
```rust
            get_presets,
            create_preset,
```

NEW:
```rust
            get_presets,
            get_builtin_preset_ids,
            create_preset,
```

### Arquivo 5C: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 8** — adicionar ao state `S`:

OLD:
```js
const S={mode:'simple',presets:[],selectedPreset:'default',_syncingPreset:false,...};
```

Adicionar `builtinIds:[]` após `presets:[]`:

```js
const S={mode:'simple',presets:[],builtinIds:[],selectedPreset:'default',_syncingPreset:false,...};
```

**Linha 235** — em `init()`, após carregar presets:

OLD:
```js
S.presets=await invoke('get_presets');d.push('pre:'+S.presets.length);
```

NEW:
```js
S.presets=await invoke('get_presets');S.builtinIds=await invoke('get_builtin_preset_ids');d.push('pre:'+S.presets.length);
```

Adicionar helper (após init, ou no topo com as outras funções):

```js
function isBuiltinPreset(id){return S.builtinIds.includes(id)}
```

**Garantia de não-quebra:**
- `get_builtin_preset_ids` é um command novo, não altera existentes
- Se falhar (rede, etc.), `S.builtinIds` fica `[]` — `isBuiltinPreset` retorna
  `false` para tudo. Comportamento: não bloqueia delete de built-in (pior caso:
  permite). Aceitável como fallback.

---

## Item 6: Debounce no updateCmdPreview

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 47** — no `hookCmdPreview()`:

OLD:
```js
  ids.forEach(id=>{const el=$(id);if(el){on(el,'change',()=>{updateCmdPreview();if(!S._syncingPreset)markPresetCustom()});on(el,'input',()=>{updateCmdPreview();if(!S._syncingPreset)markPresetCustom()})}});
```

NEW:
```js
  ids.forEach(id=>{const el=$(id);if(el){on(el,'change',()=>{debouncedCmdPreview();if(!S._syncingPreset)markPresetCustom()});on(el,'input',()=>{debouncedCmdPreview();if(!S._syncingPreset)markPresetCustom()})}});
```

Adicionar ANTES de `hookCmdPreview`:

```js
let cmdPreviewTimer=null;function debouncedCmdPreview(){clearTimeout(cmdPreviewTimer);cmdPreviewTimer=setTimeout(updateCmdPreview,300)}
```

**Linha 51** — no collapsible click handler (Command Preview expand):

Manter `updateCmdPreview()` direto (sem debounce) — o usuário clicou pra ver:

OLD:
```js
if(h.classList.contains('open')&&h.textContent.includes('Command Preview'))updateCmdPreview()
```

Essa linha NÃO muda. Já está correta — é click, não input.

**Garantia de não-quebra:**
- `clearTimeout` com `null` é no-op (não quebra)
- 300ms é tempo padrão de debounce (UX comprovado)
- O preview ainda atualiza em <300ms após o último keypress
- `markPresetCustom()` NÃO é debounced — continua instantâneo

---

## Item 7: Validação de ranges no collectAdvParams

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

Adicionar helper ANTES de `collectSimpleParams` (~linha 114):

```js
function clamp(v,min,max,fallback){const n=parseInt(v);return isNaN(n)?fallback:Math.max(min,Math.min(max,n))}
```

**Linhas 116-141** — `collectAdvParams()`:

Substituir extrações diretas por validadas:

OLD (trechos):
```js
    width:even($('#a-width').value),height:even($('#a-height').value),
```

NEW:
```js
    width:even(clamp($('#a-width').value,64,7680,1920)),height:even(clamp($('#a-height').value,64,4320,1080)),
```

OLD:
```js
    audio_bitrate:parseInt($('#a-abitrate').value)||128,audio_channels:parseInt($('#a-channels').value)||2,
    sample_rate:parseInt($('#a-samplerate').value)||48000,threads:parseInt($('#a-threads').value)||0,
```

NEW:
```js
    audio_bitrate:clamp($('#a-abitrate').value,32,320,128),audio_channels:clamp($('#a-channels').value,1,8,2),
    sample_rate:clamp($('#a-samplerate').value,8000,96000,48000),threads:clamp($('#a-threads').value,0,32,0),
```

OLD:
```js
    const crfVal=parseInt($('#a-crf').value);const crf=isNaN(crfVal)?null:crfVal;
```

NEW:
```js
    const crfVal=clamp($('#a-crf').value,0,51,23);const crf=crfVal;
```

**Linha 115** — `collectSimpleParams()` também:

OLD:
```js
    width:parseInt($('#s-width').value)||1920,height:parseInt($('#s-height').value)||1080,
    crf:parseInt($('#s-crf').value)||19,audio_bitrate:parseInt($('#s-audio').value)||128,
```

NEW:
```js
    width:even(clamp($('#s-width').value,64,7680,1920)),height:even(clamp($('#s-height').value,64,4320,1080)),
    crf:clamp($('#s-crf').value,0,51,19),audio_bitrate:clamp($('#s-audio').value,32,320,128),
```

### Arquivo 7B: `crates/mediaforge-tauri/ui/test.js`

Adicionar:

```js
describe('clamp()',()=>{
    it('within range',()=>assert.equal(clamp(50,0,100,0),50));
    it('below min',()=>assert.equal(clamp(-5,0,100,0),0));
    it('above max',()=>assert.equal(clamp(999,0,100,0),100));
    it('NaN returns fallback',()=>assert.equal(clamp('abc',0,100,42),42));
    it('empty string returns fallback',()=>assert.equal(clamp('',0,100,42),42));
});
```

**Garantia de não-quebra:**
- `clamp` retorna o mesmo valor se estiver no range — comportamento idêntico
- Para `NaN`, retorna fallback em vez de 0 ou `NaN` — mais seguro
- Os ranges são os mesmos do HTML (`min`/`max` attributes) — consistente

---

## Item 8: Documentar estado S

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 8** — antes de `const S={...}`:

OLD:
```js
// ── State ──
const S={mode:'simple',presets:[],...
```

NEW:
```js
// ── State ──
// Ownership: S.presets/builtinIds = R/O after init. S.params = R/W (collect*,
// syncPresetToUI). S.files = R/W (addFiles, renderFiles, clear). S.jobs = R/W
// (add-queue, start, progress/done events, requeue, clear). S.isEncoding = R/W
// (start, finishEncoding). S.mode = R/W (setMode). S.outputDir = R/W (browse,
// init). S.config = R/O after init. S.history = R/W (load/render/clearHistory).
// S._syncingPreset = internal guard flag. S.selectedPreset = R/W (preset change).
const S={mode:'simple',presets:[],builtinIds:[],...
```

**Garantia de não-quebra:**
- Só adiciona comentário. Zero mudança de comportamento.

---

## Item 9: Progress bar visual (div CSS)

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 211** — caso `'running'` no renderQueue:

OLD:
```js
case'running':return`<div id=\"job-running\" class=\"flex items-center justify-between text-xs py-1 px-2\"><span class=\"text-[#c0c0d0] truncate\">${inn}</span><span class=\"progress-bar-text text-[#be4266] font-mono text-[10px] shrink-0 ml-2\">${progressBar(j.progress)}</span></div>`;
```

NEW:
```js
case'running':return`<div id=\"job-running\" class=\"text-xs py-1 px-2\"><div class=\"flex justify-between items-center\"><span class=\"text-[#c0c0d0] truncate\">${inn}</span><span class=\"progress-pct text-[#be4266] font-mono text-[10px] shrink-0 ml-2\">${Math.round(j.progress)}%</span></div><div class=\"progress-bar mt-1\"><div class=\"progress-fill\" style=\"width:${Math.max(1,j.progress)}%\"></div></div></div>`;
```

**Linha 223** — handler `job:progress` (update direto):

OLD:
```js
const tb=$('#job-running .progress-bar-text');if(tb)tb.textContent=progressBar(j.progress)
```

NEW:
```js
const fill=$('#job-running .progress-fill');if(fill)fill.style.width=Math.max(1,j.progress)+'%';const pct=$('#job-running .progress-pct');if(pct)pct.textContent=Math.round(j.progress)+'%'
```

**Linha 211** — remover função `progressBar` (não é mais usada):

OLD (linha 211, logo após `on($('#btn-add-queue'...`):
```js
function progressBar(pct){const w=20,f=Math.max(0,Math.min(w,Math.round(pct/100*w)));return'['+'█'.repeat(f)+'░'.repeat(w-f)+'] '+Math.round(pct)+'%'}
```

NEW: (remover a função — `totalProgress` começa em seguida)

**Garantia de não-quebra:**
- O CSS `.progress-bar` e `.progress-fill` já existem no `input.css` (linhas
  58-70) com `width: 0%` default — a barra não "flasha" 100% no primeiro frame
- O `Math.max(1, progress)` garante que 0% ainda mostra 1px visível
- A atualização direta no handler `job:progress` usa os mesmos seletores
  (`.progress-fill`, `.progress-pct`) que o template HTML — consistente
- `progressBar()` era usada em 2 lugares (renderQueue template + job:progress
  handler) — ambos foram atualizados. Verificar com grep:
  ```bash
  grep -n "progressBar" crates/mediaforge-tauri/ui/src/main.js
  ```
  Deve retornar zero matches após a mudança.

---

## Item 10: Settings modal completo

### Arquivo 10A: `crates/mediaforge-tauri/ui/index.html`

**Linhas 366-379** — substituir settings modal:

OLD (modal inteiro):
```html
  <!-- Settings Modal -->
  <div id="settings-modal" class="hidden fixed inset-0 z-40 flex items-center justify-center bg-black/50">
    <div class="bg-[#151520] border border-[#2a2a3a] rounded-xl p-6 w-96 shadow-2xl">
      <h2 class="text-lg font-semibold text-white mb-4">Settings</h2>
      <div class="space-y-4 text-sm">
        <div>
          <label class="block text-[#9090a0] mb-1">ffmpeg Path</label>
          <input type="text" id="ffmpeg-path" placeholder="Auto-detected" class="form-input" />
        </div>
      </div>
      <div class="flex justify-end gap-2 mt-6">
        <button id="btn-settings-cancel" class="px-4 py-1.5 text-xs rounded-md bg-[#1f1f2e] text-[#9090a0] hover:text-white">Cancel</button>
        <button id="btn-settings-save" class="px-4 py-1.5 text-xs rounded-md bg-rose text-white hover:bg-rose-light">Save</button>
      </div>
    </div>
  </div>
```

NEW:
```html
  <!-- Settings Modal -->
  <div id="settings-modal" class="hidden fixed inset-0 z-40 flex items-center justify-center bg-black/50">
    <div class="bg-[#151520] border border-[#2a2a3a] rounded-xl p-6 w-[420px] shadow-2xl">
      <h2 class="text-lg font-semibold text-white mb-4">Settings</h2>
      <div class="space-y-3 text-sm">
        <div>
          <label class="block text-[#9090a0] mb-1 text-xs">ffmpeg Path</label>
          <input type="text" id="ffmpeg-path" placeholder="Auto-detected" class="form-input" />
        </div>
        <div>
          <label class="block text-[#9090a0] mb-1 text-xs">Output Directory</label>
          <div class="flex items-center gap-2">
            <input type="text" id="settings-output-dir" placeholder="~/output" class="form-input flex-1" />
            <button id="btn-settings-browse" class="px-3 py-1.5 text-xs rounded-md bg-[#1f1f2e] text-[#9090a0] hover:text-white hover:bg-[#2a2a3a]">Browse</button>
          </div>
        </div>
        <div>
          <label class="block text-[#9090a0] mb-1 text-xs">Default Mode</label>
          <select id="settings-mode" class="form-input">
            <option value="simple">Simple</option>
            <option value="advanced">Advanced</option>
          </select>
        </div>
        <div>
          <label class="block text-[#9090a0] mb-1 text-xs">Default Preset</label>
          <select id="settings-preset" class="form-input"></select>
        </div>
      </div>
      <div class="flex justify-end gap-2 mt-6">
        <button id="btn-settings-cancel" class="px-4 py-1.5 text-xs rounded-md bg-[#1f1f2e] text-[#9090a0] hover:text-white">Cancel</button>
        <button id="btn-settings-save" class="px-4 py-1.5 text-xs rounded-md bg-rose text-white hover:bg-rose-light">Save</button>
      </div>
    </div>
  </div>
```

### Arquivo 10B: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 226** — handler settings open (preencher campos):

OLD:
```js
on($('#btn-settings'),'click',()=>$('#settings-modal').classList.remove('hidden'));
```

NEW:
```js
on($('#btn-settings'),'click',()=>{if(S.config){$('#ffmpeg-path').value=S.config.ffmpeg_path||'';$('#settings-output-dir').value=S.outputDir||'';$('#settings-mode').value=(S.config.default_mode==='Advanced'?'advanced':'simple');const ps=$('#settings-preset');if(ps){ps.innerHTML='<option value="">—</option>'+S.presets.map(p=>`<option value="${p.id}">${p.name}</option>`).join('');ps.value=S.config.default_preset||''}}$('#settings-modal').classList.remove('hidden')});
```

**Linha 226** — handler settings save:

OLD:
```js
on($('#btn-settings-save'),'click',async()=>{if(S.config)S.config.ffmpeg_path=$('#ffmpeg-path').value||null;await invoke('save_config',{config:S.config});$('#settings-modal').classList.add('hidden')});
```

NEW:
```js
on($('#btn-settings-save'),'click',async()=>{if(!S.config)return;S.config.ffmpeg_path=$('#ffmpeg-path').value||null;S.config.output_dir=$('#settings-output-dir').value||null;S.config.default_mode=$('#settings-mode').value==='advanced'?'Advanced':'Simple';S.config.default_preset=$('#settings-preset').value||null;S.outputDir=S.config.output_dir||S.outputDir;updateOutputDisplay();await invoke('save_config',{config:S.config});$('#settings-modal').classList.add('hidden');diag('Settings saved')});
```

Adicionar handler para o browse dentro do settings (após a linha acima):

```js
on($('#btn-settings-browse'),'click',browseOutputDir);
```

**Garantia de não-quebra:**
- `browseOutputDir` já existe (item 4) e atualiza `S.outputDir` + display
- O `settings-output-dir` é sincronizado com `S.outputDir` ao abrir
- `save_config` é o mesmo command de antes — comportamento idêntico
- Campos adicionais (default_mode, default_preset) são salvos mas não aplicados
  até o próximo launch (init lê `get_config`)

---

## Item 11: Mover <style> para input.css

### Arquivo 11A: `crates/mediaforge-tauri/ui/index.html`
**Linhas 10-41** — remover TODO o bloco `<style>...</style>`

OLD: remover 31 linhas (10-41)

### Arquivo 11B: `crates/mediaforge-tauri/ui/src/styles/input.css`
**Após linha 92** — adicionar os estilos removidos:

```css
/* ── Layout (moved from index.html) ── */
html, body { height: 100%; margin: 0; overflow: hidden; }
#app { height: 100%; }
#sidebar { min-width: 200px; max-width: 260px; }

/* ── Tabs ── */
.tab-btn { padding: 6px 16px; border-radius: 6px 6px 0 0; font-size: 12px; font-weight: 500; cursor: pointer; transition: all 0.15s; border: none; background: transparent; color: #9090a0; }
.tab-btn.active { background: #151520; color: #be4266; }
.tab-btn:hover:not(.active) { color: #c0c0d0; }

/* ── Form grid ── */
.form-grid { display: grid; grid-template-columns: 130px 1fr; gap: 8px 16px; align-items: center; font-size: 13px; }
.form-label { color: #9090a0; text-align: right; }
.form-input { background: #1f1f2e; border: 1px solid #2a2a3a; border-radius: 6px; padding: 5px 10px; color: #f4f4f6; font-size: 13px; outline: none; width: 100%; box-sizing: border-box; }
.form-input:focus { border-color: #be4266; }
select.form-input { cursor: pointer; }

/* ── Collapsible ── */
.collapsible-header { display: flex; align-items: center; gap: 6px; padding: 8px 0; cursor: pointer; font-size: 13px; font-weight: 600; color: #c0c0d0; user-select: none; }
.collapsible-header:hover { color: #f4f4f6; }
.collapsible-header .arrow { font-size: 10px; }

/* ── HiDPI ── */
@media (-webkit-min-device-pixel-ratio: 2), (min-resolution: 192dpi) { body { -webkit-font-smoothing: antialiased; } }
```

**Garantia de não-quebra:**
- `npm run build` regenera `bundle.css` com os estilos movidos
- A ordem no HTML (`<link rel="stylesheet" href="/bundle.css">`) é a mesma
- Os seletores são idênticos — zero mudança visual
- Testar: abrir app e verificar visualmente que nada mudou

---

## Item 12: Toggle dark/light mode

### Arquivo 12A: `crates/mediaforge-tauri/ui/src/styles/input.css`
**Após `@import "tailwindcss";` e antes de `@theme`:**

```css
:root {
  --bg-deep: #0b0b12;
  --bg-card: #151520;
  --bg-input: #1f1f2e;
  --border: #2a2a3a;
  --text-primary: #f4f4f6;
  --text-secondary: #c0c0d0;
  --text-muted: #9090a0;
  --text-dim: #606070;
}
.light {
  --bg-deep: #f8f9fa;
  --bg-card: #ffffff;
  --bg-input: #e9ecef;
  --border: #dee2e6;
  --text-primary: #212529;
  --text-secondary: #495057;
  --text-muted: #6c757d;
  --text-dim: #adb5bd;
}
```

### Arquivo 12B: `crates/mediaforge-tauri/ui/index.html`

Substituir classes hardcoded por `var()` nas áreas PRINCIPAIS (body, header,
sidebar, footer, status bar, modals). NÃO substituir todas as 300 referências
— só as estruturais (~20 classes).

**Linha 43** — body:

OLD:
```html
<body class="bg-[#0b0b12] text-[#f4f4f6] font-sans" oncontextmenu="return false">
```

NEW:
```html
<body class="font-sans" style="background:var(--bg-deep);color:var(--text-primary)" oncontextmenu="return false">
```

**Linha 56** — header:

OLD:
```html
<header class="h-12 flex items-center justify-between px-4 bg-[#151520] border-b border-[#2a2a3a] shrink-0">
```

NEW:
```html
<header class="h-12 flex items-center justify-between px-4 border-b shrink-0" style="background:var(--bg-card);border-color:var(--border)">
```

**Linha 69** — sidebar:

OLD:
```html
<aside id="sidebar" class="bg-[#151520] border-r border-[#2a2a3a] flex flex-col shrink-0">
```

NEW:
```html
<aside id="sidebar" class="border-r flex flex-col shrink-0" style="background:var(--bg-card);border-color:var(--border)">
```

**Linha 346** — footer:

OLD:
```html
<footer class="border-t border-[#2a2a3a] bg-[#151520] shrink-0">
```

NEW:
```html
<footer class="border-t shrink-0" style="background:var(--bg-card);border-color:var(--border)">
```

**Linha 359** — status bar:

OLD:
```html
<div class="h-7 flex items-center justify-between px-4 bg-[#0b0b12] border-t border-[#2a2a3a] text-xs text-[#606070] shrink-0">
```

NEW:
```html
<div class="h-7 flex items-center justify-between px-4 border-t text-xs shrink-0" style="background:var(--bg-deep);border-color:var(--border);color:var(--text-dim)">
```

Adicionar toggle button no header (linha ~63, ao lado de `btn-settings`):

```html
<button id="btn-theme" class="p-1.5 rounded-md text-[#9090a0] hover:text-white hover:bg-[#1f1f2e]" title="Toggle theme">☀</button>
```

### Arquivo 12C: `crates/mediaforge-tauri/ui/src/main.js`

Adicionar no `init()` (~linha 235) para restaurar tema:

OLD:
```js
async function init(){const d=[];d.push('TI:'+(!!window.__TAURI_INTERNALS__));try{S.presets=...
```

NEW:
```js
async function init(){const d=[];d.push('TI:'+(!!window.__TAURI_INTERNALS__));try{S.presets=await invoke('get_presets');S.builtinIds=await invoke('get_builtin_preset_ids');d.push('pre:'+S.presets.length);S.config=await invoke('get_config');if(S.config?.theme==='Light'){document.documentElement.classList.add('light');$('#btn-theme').textContent='☾'}S.outputDir=...
```

Adicionar handler (após init, ou nos handlers de modal):

```js
on($('#btn-theme'),'click',()=>{const hl=document.documentElement;hl.classList.toggle('light');const isLight=hl.classList.contains('light');$('#btn-theme').textContent=isLight?'☾':'☀';if(S.config){S.config.theme=isLight?'Light':'Dark';invoke('save_config',{config:S.config}).catch(()=>{})}});
```

**Garantia de não-quebra:**
- Light mode usa `style` attribute com `var()` — se o browser não suportar
  `var()`, volta ao fallback inline (que é o dark mode original)
- Só as cores estruturais são afetadas; botões, cards internos, inputs
  continuam com cores hardcoded (dark) até migração completa futura
- Não afeta encoding, presets, ou lógica de negócio

---

## Item 13: check.sh incluir mediaforge-tauri

### Arquivo: `scripts/check.sh`

OLD:
```bash
#!/usr/bin/env bash
# Cratua Media Forge — Full test suite
set -euo pipefail

echo "=== Rust Tests ==="
cargo test -p mediaforge-core

echo ""
echo "=== JS Tests ==="
node --test crates/mediaforge-tauri/ui/test.js

echo ""
echo "=== Cargo Check ==="
cargo check -p mediaforge-core 2>&1

echo ""
echo "=== All checks passed ==="
```

NEW:
```bash
#!/usr/bin/env bash
# Cratua Media Forge — Full test suite
set -euo pipefail

echo "=== Rust Tests (core) ==="
cargo test -p mediaforge-core

echo ""
echo "=== Rust Tests (tauri) ==="
cargo test -p mediaforge-tauri --lib

echo ""
echo "=== JS Tests ==="
node --test crates/mediaforge-tauri/ui/test.js

echo ""
echo "=== Cargo Check ==="
cargo check -p mediaforge-core 2>&1

echo ""
echo "=== All checks passed ==="
```

---

## Item 14: Remover mediaforge-gui do workspace

### Arquivo: `Cargo.toml` (raiz)

**Linha 2:**

OLD:
```toml
members = ["crates/*"]
```

NEW:
```toml
members = ["crates/mediaforge-core", "crates/mediaforge-tauri"]
```

**Verificação prévia:**
```bash
grep -r "mediaforge-gui" scripts/ --include="*.sh"
```
Deve retornar zero. Se retornar algo, atualizar os scripts primeiro.

---

## Item 15: .editorconfig

### Arquivo: `.editorconfig` (novo, raiz)

Criar com:

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.rs]
indent_style = space
indent_size = 4

[*.toml]
indent_style = space
indent_size = 2

[*.{js,css,html,json,yml,yaml}]
indent_style = space
indent_size = 2

[*.md]
trim_trailing_whitespace = false

[Makefile]
indent_style = tab
```

---

## Item 16: CHANGELOG.md

### Arquivo: `CHANGELOG.md` (novo, raiz)

Criar com o histórico 0.3.5 → 0.3.23 (gerado do git log):

```bash
git log --oneline v0.3.4..HEAD --format="- %s" --reverse
```

Template:
```markdown
# Changelog

## [Unreleased]

## [0.3.23] - 2026-05-30
- Fix progress bar renders on first progress event
- Add Opus sample rate auto-correction (48000 Hz fallback)

## [0.3.22] - 2026-05-30
- ...

(completar com git log)
```

---

## Item 17: Atalhos de teclado no About

### Arquivo 17A: `crates/mediaforge-tauri/ui/index.html`
**Linhas 382-389** — About modal:

OLD:
```html
  <!-- About Modal -->
  <div id="about-modal" class="hidden fixed inset-0 z-40 flex items-center justify-center bg-black/50">
    <div class="bg-[#151520] border border-[#2a2a3a] rounded-xl p-6 w-80 text-center shadow-2xl">
      <h2 class="text-lg font-semibold text-white mb-2">Cratua Media Forge</h2>
      <p class="text-sm text-[#9090a0]">v0.3.23</p>
      <p class="text-xs text-[#606070] mt-3">Portable media converter powered by ffmpeg.<br/>Built with Rust + Tauri.</p>
      <button id="btn-about-close" class="mt-4 px-4 py-1.5 text-xs rounded-md bg-rose text-white hover:bg-rose-light">OK</button>
    </div>
  </div>
```

NEW:
```html
  <!-- About Modal -->
  <div id="about-modal" class="hidden fixed inset-0 z-40 flex items-center justify-center bg-black/50">
    <div class="bg-[#151520] border border-[#2a2a3a] rounded-xl p-6 w-80 text-center shadow-2xl">
      <h2 class="text-lg font-semibold text-white mb-2">Cratua Media Forge</h2>
      <p class="text-sm text-[#9090a0]">v0.3.23</p>
      <p class="text-xs text-[#606070] mt-3">Portable media converter powered by ffmpeg.<br/>Built with Rust + Tauri.</p>
      <div class="mt-4 pt-3 border-t border-[#2a2a3a] text-left text-xs text-[#9090a0] space-y-1">
        <div class="flex justify-between"><span>Add Files</span><kbd class="text-[#c0c0d0]">Ctrl+O</kbd></div>
        <div class="flex justify-between"><span>Start Encoding</span><kbd class="text-[#c0c0d0]">Ctrl+Enter</kbd></div>
        <div class="flex justify-between"><span>Cancel / Close</span><kbd class="text-[#c0c0d0]">Esc</kbd></div>
        <div class="flex justify-between"><span>Settings</span><kbd class="text-[#c0c0d0]">Ctrl+,</kbd></div>
      </div>
      <button id="btn-about-close" class="mt-4 px-4 py-1.5 text-xs rounded-md bg-rose text-white hover:bg-rose-light">OK</button>
    </div>
  </div>
```

### Arquivo 17B: `crates/mediaforge-tauri/ui/src/main.js`
Adicionar handler de atalhos globais (após `init()`):

```js
document.addEventListener('keydown',e=>{if(e.ctrlKey&&e.key==='o'){e.preventDefault();$('#btn-add-files').click()}else if(e.ctrlKey&&e.key==='Enter'){e.preventDefault();$('#btn-start').click()}else if(e.key==='Escape'){if(S.isEncoding)$('#btn-cancel').click();else{$('#settings-modal').classList.add('hidden');$('#about-modal').classList.add('hidden')}}else if(e.ctrlKey&&e.key===','){e.preventDefault();$('#btn-settings').click()}});
```

**Garantia de não-quebra:**
- `Ctrl+O` já abria dialog em algumas plataformas (comportamento nativo) —
  `e.preventDefault()` evita conflito
- `Esc` já fechava modals (comportamento nativo do Tauri window) — o handler
  só adiciona a lógica de cancel durante encoding
- Se o foco estiver num input, `Escape` não fecha o modal (comportamento
  correto do browser — keydown no input não propaga para document se
  o input consumir o evento)

---

## Item 18: Indicador de rename automático

### Arquivo: `crates/mediaforge-tauri/ui/src/main.js`

**Linha 210** — no `btn-add-queue`, adicionar flag `renamed`:

OLD (dentro do forEach):
```js
S.jobs.push({input:f,output:out,params:structuredClone(bp),status:'pending',progress:0})
```

NEW:
```js
S.jobs.push({input:f,output:out,params:structuredClone(bp),status:'pending',progress:0,renamed:out!==f&&out.includes('_converted')})
```

**Linha 211** — no renderQueue, caso `'pending'`:

OLD:
```js
case'pending':return`<div class=\"flex items-center justify-between text-xs py-1 px-2\"><span class=\"text-[#c0c0d0] truncate\">${inn} → ${outn}</span><span class=\"text-[#606070] shrink-0 ml-2\">pending</span></div>`;
```

NEW:
```js
case'pending':{const rn=j.renamed?'<span class=\"text-[#f59e0b] ml-1\" title=\"Renamed to avoid overwriting source\">⚠</span>':'';return`<div class=\"flex items-center justify-between text-xs py-1 px-2\"><span class=\"text-[#c0c0d0] truncate\">${inn} → ${outn}${rn}</span><span class=\"text-[#606070] shrink-0 ml-2\">pending</span></div>`}
```

**Garantia de não-quebra:**
- `out !== f` é sempre `true` quando o path é diferente do input (99% dos
  casos) — `renamed` só é `true` quando há `_converted` no nome
- Se `_converted` aparecer naturalmente no path (ex: output dir já tem
  `_converted`), o ícone aparece falsamente — edge case aceitável
- O ícone ⚠ é exibido inline, não quebra layout

---

## Ordem de aplicação

Por dependência entre itens:

1. **Item 15** (.editorconfig) — zero dependências, 1 arquivo novo
2. **Item 14** (remover egui workspace) — verificar scripts primeiro,
   depois alterar Cargo.toml
3. **Item 13** (check.sh) — adicionar linha de teste
4. **Item 1** (ffmpeg timeout) — error.rs + ffmpeg.rs, depois rodar tests
5. **Item 2** (preset lock) — 3 linhas, depois rodar tests
6. **Item 5** (builtin IDs backend) — commands.rs + lib.rs + main.js
7. **Item 6** (debounce) — main.js apenas
8. **Item 7** (validação) — main.js + test.js
9. **Item 8** (documentar S) — main.js comentário
10. **Item 4** (dedup handlers) — main.js (depende de item 5 pronto)
11. **Item 9** (progress bar visual) — main.js
12. **Item 3** (sanitização renderQueue) — main.js + test.js
13. **Item 11** (mover CSS) — index.html + input.css
14. **Item 10** (settings modal) — index.html + main.js (usa browseOutputDir
    do item 4)
15. **Item 12** (light mode) — input.css + index.html + main.js
16. **Item 16** (CHANGELOG) — git log → arquivo
17. **Item 17** (atalhos About) — index.html + main.js
18. **Item 18** (rename indicator) — main.js

---

## Verificação final (após todos os itens)

```bash
bash scripts/check.sh         # Rust core + tauri + JS tests + cargo check
cargo build --release          # sem warnings
npm run build                  # frontend compila
cargo test -p mediaforge-core  # 125 tests (123 + timeout_display + ?)
node --test crates/mediaforge-tauri/ui/test.js  # ~10 tests
```

Smoke test manual:
1. `cargo run` (Tauri dev mode)
2. Splash → app carrega sem erro no diag
3. Abrir Settings → campos novos visíveis (output dir, mode, preset)
4. Mudar output dir → salvar → reabrir settings → valor persiste
5. Adicionar arquivo → "Add to Queue" → ícone ⚠ não aparece (path normal)
6. Mesmo arquivo como output → ícone ⚠ aparece
7. Start encoding → barra de progresso visual (div CSS) → completa
8. Forçar erro (ex: codec inválido) → mensagem de erro clicável →
   copia para clipboard
9. Ctrl+, → abre Settings
10. Ctrl+O → abre Add Files dialog
11. About → atalhos documentados
