# Tutorial 08 — Frontend: HTML, CSS e Tailwind v4

**Objetivo:** Construir a interface completa do app usando HTML vanilla,
Tailwind CSS v4 para estilização, e esbuild para bundle.

**Tempo estimado:** 1 hora

---

## 1. Estrutura de arquivos

```
crates/mediaforge-tauri/ui/
├── index.html          # SPA (Single Page Application)
├── bundle.js           # Gerado pelo esbuild
├── bundle.css          # Gerado pelo Tailwind CLI
├── src/
│   ├── main.js         # Código JS fonte
│   └── styles/
│       └── input.css   # CSS fonte (Tailwind + custom)
└── test.js             # Testes unitários JS
```

---

## 2. index.html — Estrutura

```html
<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Cratua Media Forge</title>
  <link rel="stylesheet" href="/bundle.css" />
</head>
<body class="font-sans" style="background:var(--bg-deep);color:var(--text-primary)"
      oncontextmenu="return false">
```

**`oncontextmenu="return false"`** — desabilita o menu de contexto do
browser (clique direito). App desktop não deve mostrar menu de browser.

### Splash Screen

```html
<div id="splash" class="fixed inset-0 z-50 flex flex-col items-center
     justify-center bg-[#0b0b12]">
  <img src="/assets/splash.png" alt="Cratua"
       class="max-w-[600px] max-h-[350px] object-contain mb-6" />
  <span class="text-sm font-medium text-white">Cratua Media Forge</span>
  <span class="text-xs text-[#9090a0]">v0.4.0</span>
</div>
```

A splash some após 2 segundos (JS: `setTimeout` + fade out).

### App Shell

```
#app (flex column, h-full)
├── header (h-12, modo + about + theme + settings)
├── div.flex.flex-1
│   ├── #sidebar (files + history)
│   └── main
│       ├── #mode-simple
│       └── #mode-advanced (tabs)
├── footer (queue + botões)
└── .status-bar
```

### Header

```html
<header class="h-12 flex items-center justify-between px-4 border-b shrink-0"
        style="background:var(--bg-card);border-color:var(--border)">
  <span class="text-base font-semibold text-white">Cratua Media Forge</span>
  <div class="flex items-center gap-1">
    <button id="btn-simple" class="...bg-rose text-white...">Simple</button>
    <button id="btn-advanced" class="...">Advanced</button>
    <button id="btn-about" title="About">?</button>
    <button id="btn-theme" title="Toggle theme">☀</button>
    <button id="btn-settings" title="Settings">⚙</button>
  </div>
</header>
```

### Sidebar

```html
<aside id="sidebar" class="border-r flex flex-col shrink-0"
       style="background:var(--bg-card);border-color:var(--border)">
  <!-- Files section -->
  <div class="p-3 border-b border-[#2a2a3a]">
    <h2 class="text-xs font-semibold text-[#9090a0] uppercase">Files</h2>
    <button id="btn-add-files" class="...">+ Add Files</button>
  </div>
  <input id="path-input" type="text" placeholder="Paste file path..." />
  <div id="file-list" class="flex-1 overflow-y-auto"></div>
  <button id="btn-clear-files">Clear All</button>

  <!-- History section -->
  <h2 class="text-xs font-semibold text-[#9090a0] uppercase">History</h2>
  <div id="history-list"></div>
</aside>
```

### Modos Simple/Advanced

**Simple mode:** preset selector + resolution + CRF slider + audio bitrate
+ deinterlace checkbox + output dir.

**Advanced mode:** tabs (Video, Audio, Filters, Output, Metadata) com
painéis expansíveis.

### Queue Footer

```html
<footer class="border-t shrink-0"
        style="background:var(--bg-card);border-color:var(--border)">
  <button id="btn-add-queue" class="bg-rose text-white">Add to Queue</button>
  <button id="btn-start" class="bg-[#22c55e] text-white">Start Encoding</button>
  <button id="btn-cancel" class="bg-[#ef4444] text-white hidden">Cancel</button>
  <div id="queue-list" class="max-h-36 overflow-y-auto"></div>
</footer>
```

### Status Bar

```html
<div class="h-7 flex items-center justify-between px-4 border-t text-xs shrink-0"
     style="background:var(--bg-deep);border-color:var(--border);color:var(--text-dim)">
  <span id="status-text">Ready</span>
  <span id="status-count">0 files | 0 jobs</span>
</div>
```

---

## 3. input.css — Tailwind v4 + Custom Styles

```css
@import "tailwindcss";

/* Tema de cores */
@theme {
  --color-rose: #be4266;
  --color-rose-light: #d46484;
  --color-rose-dark: #9e2e50;
}

/* CSS Custom Properties para light/dark mode */
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

/* Overrides para classes Tailwind hardcoded */
.light .bg-\[\#151520\],
.light .bg-\[\#0b0b12\] { background: var(--bg-card) !important; }
.light .bg-\[\#1f1f2e\] { background: var(--bg-input) !important; }
.light .border-\[\#2a2a3a\] { border-color: var(--border) !important; }
.light .text-\[\#9090a0\] { color: var(--text-muted) !important; }
.light .text-\[\#606070\] { color: var(--text-dim) !important; }
.light .text-\[\#c0c0d0\] { color: var(--text-secondary) !important; }
.light .text-\[\#f4f4f6\] { color: var(--text-primary) !important; }
.light .text-white { color: #212529 !important; }
.light .hover\:text-white:hover { color: #212529 !important; }

/* Progress bar */
.progress-bar { height: 6px; background: #2a2a3a; border-radius: 3px; overflow: hidden; }
.progress-bar-fill { height: 100%; width: 0%; background: linear-gradient(90deg, #be4266, #d46484); border-radius: 3px; }
.progress-pct { font-variant-numeric: tabular-nums; }

/* Layout, tabs, forms, collapsible (movidos do <style>) */
.tab-btn { padding: 6px 16px; border-radius: 6px 6px 0 0; font-size: 12px; font-weight: 500; cursor: pointer; border: none; background: transparent; color: #9090a0; }
.tab-btn.active { background: #151520; color: #be4266; }
.form-grid { display: grid; grid-template-columns: 130px 1fr; gap: 8px 16px; align-items: center; font-size: 13px; }
.form-label { color: #9090a0; text-align: right; }
.form-input { background: #1f1f2e; border: 1px solid #2a2a3a; border-radius: 6px; padding: 5px 10px; color: #f4f4f6; font-size: 13px; outline: none; width: 100%; box-sizing: border-box; }
.form-input:focus { border-color: #be4266; }
.collapsible-header { display: flex; align-items: center; gap: 6px; padding: 8px 0; cursor: pointer; font-size: 13px; font-weight: 600; color: #c0c0d0; user-select: none; }
.collapsible-header:hover { color: #f4f4f6; }
.collapsible-header .arrow { font-size: 10px; }
```

**Build do CSS:**
```bash
npx @tailwindcss/cli -i ui/src/styles/input.css -o ui/bundle.css
```

---

## 4. package.json

```json
{
  "name": "mediaforge-tauri",
  "private": true,
  "version": "0.1.0",
  "scripts": {
    "build": "V=$(node -p \"require('./package.json').version\") && sed -i \"s/v__VERSION__/v$V/\" ui/index.html && npx @tailwindcss/cli -i ui/src/styles/input.css -o ui/bundle.css && esbuild ui/src/main.js --bundle --minify --outfile=ui/bundle.js"
  },
  "devDependencies": {
    "@tailwindcss/cli": "^4.0.0",
    "esbuild": "^0.25.0",
    "tailwindcss": "^4.0.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.11.0",
    "@tauri-apps/plugin-dialog": "^2.7.1"
  }
}
```

**Explicação do script `build`:**
1. Extrai versão do `package.json`
2. `sed` substitui `v__VERSION__` no HTML pela versão real
3. `@tailwindcss/cli` gera `bundle.css` (tree-shaking)
4. `esbuild` gera `bundle.js` (bundle + minify)

---

## 5. Verificação

```bash
cd crates/mediaforge-tauri && npm run build
# Deve gerar ui/bundle.css (~900 linhas) e ui/bundle.js (~45KB)
```

---

## Próximo passo

Tutorial 09 — JavaScript: estado global, inicialização, modos e sliders.
