# Tutorial 10 — Fila, Encoding, Eventos e UI Final

**Objetivo:** Implementar a fila de encoding, comunicação em tempo real
com o backend via eventos, e os toques finais da UI (settings, tema,
atalhos, doação).

---

## 1. Presets: Load, Sync, Save, Delete

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

**`_syncingPreset`:** Flag que evita que `syncPresetToUI()` dispare
`markPresetCustom()`. Sem isso, carregar um preset built-in marcaria
imediatamente como "Custom".

**Save/Delete unificados:**
```js
async function saveCurrentPreset(selId) {
  const name = prompt('Preset name:');
  if (!name) return;
  const params = S.mode === 'simple' ? collectSimpleParams() : collectAdvParams();
  const id = name.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  await invoke('create_preset', { preset: { id, name, params } });
  S.presets = await invoke('get_presets');
  populatePresetDropdowns(id);
}

async function deleteCurrentPreset(selId) {
  const id = $(selId).value;
  if (isBuiltinPreset(id)) { diag('Cannot delete built-in preset'); return; }
  if (!await confirm(`Delete?`)) return;
  await invoke('delete_preset', { id });
  S.presets = await invoke('get_presets');
  populatePresetDropdowns();
}
```

---

## 2. Command Preview com Debounce

```js
let cmdPreviewTimer = null;
function debouncedCmdPreview() {
  clearTimeout(cmdPreviewTimer);
  cmdPreviewTimer = setTimeout(updateCmdPreview, 300);
}

async function updateCmdPreview() {
  const p = S.mode === 'simple' ? collectSimpleParams() : collectAdvParams();
  const ext = p.container ? p.container.toLowerCase() : 'mp4';
  const cmd = await invoke('build_command_preview', { params: p });
  if ($('#cmd-preview'))
    $('#cmd-preview').value = cmd.replace(/output\.mp4/g, 'output.' + ext);
}
```

**Por que debounce?** Cada keypress no campo extra_args dispara uma chamada
IPC. 300ms de espera reduz 25 chamadas para 1.

---

## 3. Arquivos: Add, Drag-Drop, Path Input

```js
// Diálogo nativo
on($('#btn-add-files'), 'click', async () => {
  const sel = await dialogOpen({
    multiple: true,
    filters: [{ name: 'Media', extensions: ['mp4','mkv','mov','avi','webm','mp3','wav','flac','m4a','ogg'] }]
  });
  if (sel) addFiles(Array.isArray(sel) ? sel : [sel]);
});

// Drag-drop nativo do Tauri
const wv = getCurrentWebview();
await wv.onDragDropEvent(ev => {
  if (ev.payload.type === 'drop' && ev.payload.paths?.length)
    addFiles(ev.payload.paths);
});

// Path input manual (fallback WSL)
on($('#path-input'), 'keydown', e => {
  if (e.key === 'Enter') { addFiles([e.target.value.trim()]); e.target.value = ''; }
});

function addFiles(paths) {
  const np = paths.map(normPath).filter(p => !S.files.includes(p));
  if (np.length) S.files.push(...np);
  renderFiles(); updateStatus();
}
```

---

## 4. Fila de Encoding

```js
on($('#btn-add-queue'), 'click', () => {
  if (!S.files.length) { diag('No files'); return; }

  const bp = S.mode === 'simple' ? collectSimpleParams() : collectAdvParams();
  const ext = bp.container.toLowerCase();
  const ow = [];

  S.files.forEach(f => {
    const nm = f.split('/').pop();
    const st = nm.includes('.') ? nm.substring(0, nm.lastIndexOf('.')) : nm;
    let out = `${S.outputDir}/${st}.${ext}`;

    // Protege source file: renomeia se output == input
    if (out === f) { out = `${S.outputDir}/${st}_converted.${ext}`; ow.push(nm); }

    S.jobs.push({
      input: f, output: out,
      params: structuredClone(bp),  // cópia profunda!
      status: 'pending', progress: 0,
      renamed: out !== f && out.includes('_converted')
    });
  });

  S.files = []; renderFiles(); renderQueue();
});
```

**`structuredClone(bp)`** — essencial! `{ ...bp }` é shallow e
compartilharia arrays (`video_filters`) entre jobs.

---

## 5. Progress Bar (CSS, não texto)

**Template no renderQueue (caso 'running'):**
```js
case 'running': return `<div id="job-running" class="text-xs py-1 px-2">
  <div class="flex justify-between items-center">
    <span class="text-[#c0c0d0] truncate">${inn}</span>
    <span class="progress-pct ...">${Math.round(j.progress)}%</span>
  </div>
  <div class="progress-bar mt-1">
    <div class="progress-bar-fill" style="width:${Math.max(1,j.progress)}%"></div>
  </div>
</div>`;
```

**Handler de progresso (update DIRETO no DOM):**
```js
await listen('job:progress', e => {
  const j = S.jobs.find(x => x.status === 'running' || x.status === 'pending');
  if (j) {
    const wasPending = j.status === 'pending';
    j.status = 'running'; j.progress = e.payload.progress_pct ?? 0;

    if (wasPending || !$('#job-running')) {
      renderQueue();  // primeiro evento: cria o DOM
    } else {
      // Update direto — sem innerHTML!
      const fill = $('#job-running .progress-bar-fill');
      if (fill) fill.style.width = Math.max(1, j.progress) + '%';
      const pct = $('#job-running .progress-pct');
      if (pct) pct.textContent = Math.round(j.progress) + '%';
    }
  }
});
```

**Por que update direto no DOM?** `renderQueue()` usa `innerHTML` que
destrói e recria elementos. Com eventos a cada ~100ms, qualquer transição
CSS seria resetada. Update direto preserva o elemento.

---

## 6. Sanitização de Erros

```js
case 'failed': {
  const safeErr = encodeURIComponent(j.error || 'unknown');
  return `<div class="err-msg ..." data-error="${safeErr}">
    ${decodeURIComponent(safeErr).substring(0, 500)}</div>`;
}
```

Event delegation no `#queue-list`:
```js
on($('#queue-list'), 'click', e => {
  const el = e.target.closest('.err-msg');
  if (el && el.dataset.error) {
    navigator.clipboard.writeText(decodeURIComponent(el.dataset.error));
    diag('Error copied to clipboard');
  }
});
```

---

## 7. Settings, Tema, Atalhos

**Settings:** Preenche campos ao abrir, salva via `invoke('save_config')`.

**Tema:** Toggle `class="light"` no `<html>`. Salva em `S.config.theme`.

**Atalhos de teclado:**
```js
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 'o') { e.preventDefault(); $('#btn-add-files').click(); }
  if (e.ctrlKey && e.key === 'Enter') { e.preventDefault(); $('#btn-start').click(); }
  if (e.key === 'Escape') { /* cancela ou fecha modais */ }
  if (e.ctrlKey && e.key === ',') { e.preventDefault(); $('#btn-settings').click(); }
});
```

**Doação:** Botão no About abre GitHub Sponsors:
```js
on($('#btn-donate'), 'click', () => {
  window.open('https://github.com/sponsors/laucalixto', '_blank');
});
```

---

## 8. Verificação

```bash
npm run build          # bundle.js 45KB, bundle.css ~900 linhas
node --test ui/test.js # 42 testes passando
cargo test --workspace # 137 testes passando
bash scripts/check.sh  # verificação completa
```

---

## Próximo passo

Tutorial 11 — Build pipeline, cross-compilação e release.
