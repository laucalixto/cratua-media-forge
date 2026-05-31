# Tutorial 09 — JavaScript: Estado, Inicialização, Modos e Sliders

**Objetivo:** Implementar o estado global da aplicação, a inicialização,
a troca de modos Simple/Advanced e os sliders vinculados.

**Tempo estimado:** 1 hora

---

## 1. Imports e Helpers

```js
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, confirm } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
```

| Import | Propósito |
|--------|-----------|
| `invoke` | Chamar comandos Rust do frontend |
| `dialogOpen` | Diálogo nativo de abrir arquivo/pasta |
| `confirm` | Diálogo nativo de confirmação |
| `listen` | Escutar eventos do backend |
| `getCurrentWebview` | Acessar API de drag-drop nativa |

**Mini jQuery:**
```js
const $ = s => document.querySelector(s);
const $$ = s => document.querySelectorAll(s);
const on = (el, ev, fn) => el.addEventListener(ev, fn);
```

---

## 2. Estado Global (S)

```js
const S = {
  mode: 'simple',
  presets: [],
  builtinIds: [],
  selectedPreset: 'default',
  _syncingPreset: false,   // flag interno
  params: { /* EncodeParams completo */ },
  files: [],
  outputDir: '',
  jobs: [],
  isEncoding: false,
  config: null,
  history: [],
};
```

**Por que um objeto mutável e não Redux?** O app tem ~250 linhas de JS.
Adicionar um gerenciador de estado seria desproporcional. O event loop
single-threaded do browser + Tauri (que serializa comandos) garante que
não há race conditions.

---

## 3. Diagnostic Bar

```js
const diagEl = document.createElement('div');
diagEl.style.cssText = 'position:fixed;bottom:0;left:0;right:0;background:#111;' +
  'color:#0f0;font:10px monospace;padding:4px 10px;z-index:999;opacity:0.9';
document.body.appendChild(diagEl);
function diag(m) { diagEl.textContent = m; console.log(m); }
```

Fixada no rodapé, sempre visível. Essencial para debugging em produção
(sem DevTools abertos).

---

## 4. Inicialização

```js
async function init() {
  const d = [];
  d.push('TI:' + (!!window.__TAURI_INTERNALS__));

  try {
    S.presets = await invoke('get_presets');
    S.builtinIds = await invoke('get_builtin_preset_ids');
    d.push('pre:' + S.presets.length);

    S.config = await invoke('get_config');

    // Restaura tema
    if (S.config?.theme === 'Light') {
      document.documentElement.classList.add('light');
      $('#btn-theme').textContent = '☾';
    }

    S.outputDir = S.config?.output_dir
      || await invoke('get_default_output_dir');
    updateOutputDisplay();

    await setupEvents();
    await setupDragDrop();
    await loadHistory();
    d.push('READY');
  } catch (e) {
    d.push('ERR:' + String(e).substring(0, 60));
    S.outputDir = 'output';
  }

  diag(d.join(' | '));
  populatePresetDropdowns('default');
  updateProfileOptions();
  renderFilters(); renderMetadata();
  renderFiles(); updateCrfWarning();
  hookCmdPreview(); updateCmdPreview();
}
init();
```

**Fluxo de init:**
1. Carrega presets + builtin IDs do backend
2. Carrega config (tema, output dir)
3. Restaura tema light se salvo
4. Configura output dir
5. Registra listeners de eventos e drag-drop
6. Carrega histórico
7. Renderiza UI inicial

---

## 5. Splash Screen

```js
setTimeout(() => {
  const s = $('#splash');
  s.style.opacity = '0';
  s.style.transition = 'opacity 0.5s';
  setTimeout(() => {
    s.classList.add('hidden');
    $('#app').classList.remove('hidden');
  }, 500);
}, 2000);
```

Mostra a splash por 2 segundos, fade out em 0.5s.

---

## 6. Modos Simple/Advanced

```js
function setMode(m) {
  S.mode = m;
  $('#btn-simple').className = m === 'simple'
    ? 'px-3 py-1.5 text-xs font-medium rounded-md bg-rose text-white'
    : 'px-3 py-1.5 text-xs font-medium rounded-md text-[#9090a0] hover:text-white hover:bg-[#1f1f2e]';
  $('#btn-advanced').className = m === 'advanced'
    ? 'px-3 py-1.5 text-xs font-medium rounded-md bg-rose text-white'
    : 'px-3 py-1.5 text-xs font-medium rounded-md text-[#9090a0] hover:text-white hover:bg-[#1f1f2e]';
  $('#mode-simple').classList.toggle('hidden', m !== 'simple');
  $('#mode-advanced').classList.toggle('hidden', m !== 'advanced');
  if (m === 'advanced') syncSimpleToAdvanced();
}
```

**`syncSimpleToAdvanced()`:** Copia width, height, CRF do simple para o
advanced. Garante que o usuário não perca ajustes ao trocar de modo.

---

## 7. Sliders Vinculados (range ↔ number)

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
    r.value = x; n.value = x;
    if (v) v.textContent = suf ? x + suf : x;
    if (rid === '#a-crf') updateCrfWarning();
  });
}

// Liga os 4 pares de slider+number
bindSN('#s-crf', '#s-crf-num', '#s-crf-val', '');
bindSN('#s-audio', '#s-audio-num', '#s-audio-val', ' kbps');
bindSN('#a-crf', '#a-crf-num', '#a-crf-val', '');
bindSN('#a-abitrate', '#a-abitrate-num', '#a-abitrate-val', ' kbps');
```

**Por que dois handlers?**
- `input` no range: atualiza o number em tempo real (drag)
- `change` no number: atualiza o range quando usuário digita e dá Enter/blur

---

## 8. Tabs (Video, Audio, Filters, Output, Metadata)

```js
$$('.tab-btn').forEach(b => on(b, 'click', () => {
  $$('.tab-btn').forEach(x => x.classList.remove('active'));
  b.classList.add('active');
  $$('.tab-panel').forEach(x => x.classList.add('hidden'));
  $(`#tab-${b.dataset.tab}`).classList.remove('hidden');
  if (b.dataset.tab === 'output') updateCmdPreview();
}));
```

---

## 9. Collapsible (▶/▼)

```js
function setCollapsible(h) {
  const arrow = h.querySelector('.arrow');
  const body = h.nextElementSibling;
  const isOpen = h.classList.contains('open');
  body.style.display = isOpen ? '' : 'none';
  if (arrow) arrow.textContent = isOpen ? '▼' : '▶';
}

$$('.collapsible-header').forEach(h => {
  setCollapsible(h);
  on(h, 'click', () => {
    h.classList.toggle('open');
    setCollapsible(h);
    if (h.classList.contains('open') &&
        h.textContent.includes('Command Preview'))
      updateCmdPreview();
  });
});
```

**`classList.contains('open')` como source of truth** — nunca usar
`body.style.display !== 'none'` porque `style.display` só reflete estilos
inline, não CSS.

---

## 10. Validação: clamp()

```js
function clamp(v, min, max, fallback) {
  const n = parseInt(v);
  return isNaN(n) ? fallback : Math.max(min, Math.min(max, n));
}
```

Usado em `collectSimpleParams()` e `collectAdvParams()` para garantir
que valores dos inputs estejam em ranges válidos.

---

## 11. Verificação

```bash
npm run build          # bundle OK
node --test ui/test.js # 42 testes passando
```

---

## Próximo passo

Tutorial 10 — Presets, filtros, metadata, command preview e fila de encoding.
