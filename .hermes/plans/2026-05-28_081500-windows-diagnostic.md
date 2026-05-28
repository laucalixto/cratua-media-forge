# Diagnóstico Profundo: Windows .exe — Cratua Media Forge

**Data:** 2026-05-28
**Contexto:** O `.exe` cross-compilado para Windows não abre e o ícone não aparece. Múltiplas tentativas de correção sem confirmação de sucesso.

---

## 1. Hipóteses de falha

### H1: WebView2 Runtime ausente ou DLL não carregada
- `WebView2Loader.dll` está ao lado do `.exe`, mas a versão do WebView2 Runtime pode não estar instalada no Windows alvo
- O `webviewInstallMode: embedBootstrapper` pode não funcionar corretamente no cross-compile

### H2: DLLs MinGW faltantes
- Cross-compile `x86_64-pc-windows-gnu` pode gerar dependências em `libgcc_s_seh-1.dll`, `libwinpthread-1.dll`
- Rust recente linka estaticamente, mas o Tauri pode reintroduzir dependências dinâmicas

### H3: Ícone não exibido por formato incorreto no resource
- `tauri_build` pode estar usando o PNG em vez do ICO como RT_ICON
- O resource `.rc` gerado pode ter o ícone com formato inválido para o Windows Explorer

### H4: Crash silencioso na inicialização
- CSP bloqueia recursos essenciais
- Erro no JavaScript que impede a UI de carregar
- Plugin Tauri não inicializa (dialog, shell)
- ffmpeg detect falha e trava o app

### H5: Problema de permissão ou antivírus
- Windows Defender bloqueia binário não assinado
- Falta de permissão para acessar diretório de trabalho

---

## 2. Plano de diagnóstico (em ordem)

### Fase A: Logging e visibilidade (1-2 horas)

#### A1. Adicionar logging de inicialização no Rust
**Arquivo:** `crates/mediaforge-tauri/src/main.rs`
- Criar `main.rs` com `setup` hook que escreve em `%TEMP%/cratua-media-forge.log`
- Logar: versão do Tauri, WebView2 detectado, plugins carregados, ffmpeg encontrado
- Usar `std::fs::write` com append mode, timestamp em cada entrada
- Fazer setup hook ANTES de `tauri::Builder::default().run()`

#### A2. Adicionar error boundary no frontend
**Arquivo:** `crates/mediaforge-tauri/ui/index.html`
- Adicionar `<div id="crash-screen" class="hidden">` com mensagem de erro visível
- No JS: `window.onerror` + `window.onunhandledrejection` que mostra esse div
- Logar erros no console e no status text

#### A3. Criar script de diagnóstico PowerShell
**Arquivo:** `scripts/diagnose-windows.ps1`
- Verificar WebView2 Runtime instalado (`Get-ItemProperty`)
- Listar DLLs na pasta do app
- Verificar dependências com `dumpbin /dependents`
- Verificar se o ícone está embedado (extrair com Resource Hacker)
- Executar o .exe e capturar exit code + stderr

### Fase B: Correções direcionadas (2-4 horas)

#### B1. Forçar uso de ICO no resource do Windows
**Arquivo:** `crates/mediaforge-tauri/build.rs`
- Hook customizado no `build.rs`: após `tauri_build::build()`, verificar se `.rc` gerado usa `.ico`
- Se não, gerar `.rc` manualmente com `windres` apontando para `icons/icon.ico`
- Alternativa: remover `icon.png` do `bundle.icon` para Windows (usar só `.ico`)

#### B2. Testar build nativo Windows vs cross-compile
- Compilar em uma VM Windows ou GitHub Actions (windows-latest)
- Comparar binários: tamanho, DLLs importadas, resource section
- Se build nativo funciona, documentar workflow com GitHub Actions

#### B3. Substituir cross-compile por GitHub Actions
**Arquivo:** `.github/workflows/release.yml`
- Workflow com `windows-latest` runner
- Build: `cargo tauri build`
- Upload artifacts: `.exe` + `.msi` + `.zip`
- Linux: adicionar `ubuntu-latest` runner também

#### B4. WebView2: embed completo em vez de bootstrapper
**Arquivo:** `crates/mediaforge-tauri/tauri.conf.json`
- Mudar `webviewInstallMode.type` de `embedBootstrapper` para `offline`
- Testar em Windows que não tem WebView2 (VM limpa)
- Se não funcionar, usar `fixedVersion` com WebView2 Fixed Version Runtime

### Fase C: Empacotamento robusto (2-3 horas)

#### C1. Gerar .msi com WiX via GitHub Actions
**Arquivo:** `crates/mediaforge-tauri/tauri.conf.json`
- Remover `"wix": null` e configurar WiX mínimo
- Adicionar `.wxs` template se necessário
- Testar no CI Windows

#### C2. Code signing com certificado real (futuro)
- Documentar processo para quando houver certificado EV
- Manter suporte a `osslsigncode` para self-signed
- Adicionar timestamp server fallback (DigiCert + Sectigo)

---

## 3. Arquivos a modificar

| Arquivo | Ação | Fase |
|---|---|---|
| `crates/mediaforge-tauri/src/main.rs` | **CRIAR** — logging de startup | A1 |
| `crates/mediaforge-tauri/ui/index.html` | **EDITAR** — crash screen + error handler | A2 |
| `scripts/diagnose-windows.ps1` | **CRIAR** — script de diagnóstico | A3 |
| `crates/mediaforge-tauri/build.rs` | **EDITAR** — forçar .ico no resource | B1 |
| `.github/workflows/release.yml` | **CRIAR** — CI/CD Windows + Linux | B3 |
| `crates/mediaforge-tauri/tauri.conf.json` | **EDITAR** — webview offline mode | B4 |
| `scripts/release.sh` | **EDITAR** — apontar para CI artifacts | B3 |

---

## 4. Validação

- **Fase A:** Log aparece em `%TEMP%` após executar o `.exe`
- **Fase B1:** `dumpbin /headers` mostra RT_ICON com dados ICO, não PNG
- **Fase B3:** GitHub Actions gera `.zip` funcional no Windows limpo
- **Fase B4:** App abre em Windows sem WebView2 Runtime
- **Fase C:** `.msi` instala e app aparece no menu Iniciar com ícone correto

---

## 5. Riscos e tradeoffs

| Risco | Mitigação |
|---|---|
| Cross-compile nunca funcionará 100% | Migrar para GitHub Actions Windows runner |
| GitHub Actions custa créditos | Usar runners gratuitos (2000 min/mês para repo público) |
| WebView2 offline aumenta o bundle em ~150 MB | Oferecer duas versões: lite (offline) e full (embedded) |
| Ícone ainda não aparece após correções | Pode ser cache do Explorer — instruir `ie4uinit -show` |

---

## 6. Ordem recomendada de execução

1. **Fase A1 + A2** — logging (1h) — para ter diagnóstico nas próximas execuções
2. **Fase A3** — script PS (30min) — para o usuário rodar e reportar
3. **Fase B3** — GitHub Actions (1.5h) — build nativo Windows, resolve cross-compile
4. **Fase B1** — ícone (30min) — corrge resource se ainda necessário após B3
5. **Fase B4** — WebView2 (1h) — garante funcionamento offline
6. **Fase C** — empacotamento (2h) — MSI + signing, apenas se tudo acima OK
