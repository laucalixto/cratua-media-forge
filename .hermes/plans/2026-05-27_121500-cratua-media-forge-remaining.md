# Plano — Cratua Media Forge (pós-refactor)

**Data:** 2026-05-27
**Status atual:** App funcional, compila Linux + Windows cross, splash/ícone/assinatura OK, UI modularizada em views/.

---

## 1. Cancel real (matar processo ffmpeg)

**Problema:** `cancel_encoding()` em `views/queue.rs` só marca `JobStatus::Cancelled` mas o `std::thread` continua rodando e o `child process` do ffmpeg nunca é morto. Se o usuário cancelar e iniciar novo encoding, processos zumbis acumulam.

**Abordagem:** O `run_with_progress_and_ffmpeg` em `mediaforge-core/src/ffmpeg.rs` spawna um `std::process::Child`. Precisamos:
1. Guardar o `Child` handle em algum lugar acessível pela thread de cancel
2. No cancel, chamar `child.kill()` (SIGKILL/terminate)
3. A thread detecta que o child foi morto e reporta `Cancelled`

**Opção A:** `Arc<Mutex<Option<Child>>>` compartilhado entre a thread de encoding e o app state. A thread de encoding escreve o Child lá, o cancel lê e chama kill.

**Opção B:** Usar `mpsc` com uma variante `JobEvent::Cancel { job_id }` — a thread de encoding recebe o cancel pelo channel e mata o child. Mas o `mpsc::Sender` já está sendo usado pra enviar progresso, não pra receber comandos. Precisaria de um segundo channel (`cancel_tx`/`cancel_rx`).

Recomendo **Opção A**: `Arc<Mutex<Option<Child>>>` por thread de job. Menos invasivo.

**Arquivos:** `views/queue.rs`, `mediaforge-core/src/ffmpeg.rs`

**Validação:** Iniciar encoding, cancelar, verificar com `ps aux | grep ffmpeg` que o processo morreu.

---

## 2. Dragon Trap no Advanced Mode

**Problema:** No modo avançado, campos `Option<T>` como `crf`, `video_bitrate`, `max_bitrate`, `bufsize` usam `unwrap_or(0)` em variáveis locais e só escrevem de volta se `.changed()`. Se o usuário nunca interage, o campo fica `None` e o ffmpeg recebe comando incompleto.

O Simple mode já tem guard:
```rust
if self.params.crf.is_none() {
    self.params.crf = Some(crf);
}
```

O Advanced mode (`views/advanced.rs`) não tem. Precisamos adicionar para: `crf`, `video_bitrate`, `max_bitrate`, `bufsize`.

**Arquivo:** `views/advanced.rs`

**Validação:** Abrir modo avançado, trocar de aba sem mexer em CRF, verificar no Command Preview que `-crf` aparece.

---

## 3. Histórico de jobs

**Objetivo:** Salvar os últimos N jobs (ex: 20) em disco, exibir numa aba/lista, permitir reexecutar com 1 clique.

**Abordagem:**
1. Adicionar `JobHistory` struct em `mediaforge-core` que serializa/deserializa com serde + TOML
2. Após cada encoding concluído, adicionar ao histórico e salvar
3. Nova view `views/history.rs` com lista scrollável e botão "Re-run"
4. Integrar no menu ou como botão na UI principal

**Arquivos:** `mediaforge-core/src/job.rs` (ou novo `history.rs`), `views/history.rs`, `app.rs`

**Validação:** Converter 2 arquivos, fechar e reabrir o app, ver histórico listado, clicar re-run.

---

## 4. Presets customizados

**Objetivo:** Usuário configura parâmetros no Simple ou Advanced, clica "Save as Preset", dá um nome, e o preset aparece no dropdown.

**Abordagem:**
1. Adicionar `custom_presets: Vec<Preset>` ao `Config` (já serializado em TOML)
2. Botão "Save as Preset" no Simple mode (abaixo do dropdown de preset)
3. Carregar custom presets junto com built-in no `new()`
4. Permitir deletar preset customizado (botão ✕)

**Arquivos:** `mediaforge-core/src/config.rs`, `mediaforge-core/src/job.rs` (struct Preset), `views/simple.rs`, `app.rs`

**Validação:** Configurar params, salvar como "Meu Preset", ver no dropdown, selecionar e verificar que params foram aplicados.

---

## 5. Revisão visual

**Problema:** O tema Forge atual não agradou visualmente. A UI ficou funcional mas com identidade visual questionável.

**Abordagem:** Em vez de iterar às cegas no egui, mockar o visual desejado primeiro:
1. Criar um HTML estático com o layout ideal (sidebar + painel central + cards)
2. Validar com o usuário
3. Implementar o equivalente em egui

**Alternativas a considerar:**
- Voltar ao egui default com ajustes mínimos (só spacing, cantos arredondados, sem cores customizadas)
- Tema mais claro/sóbrio (cinzas neutros, sem âmbar)
- Implementar sidebar de verdade (painel esquerdo fixo com files/presets, direita com conteúdo)

**Arquivos:** `theme.rs`, `app.rs`, views (potencialmente todos)

**Validação:** Preview visual antes de codificar.

---

## 6. Testes unitários

**Objetivo:** Cobrir `mediaforge-core` com testes para evitar regressões.

**Cobertura proposta:**
- `ffmpeg.rs`: `build_command_with_ffmpeg` — verificar que parâmetros geram os args corretos
- `ffmpeg.rs`: `detect_ffmpeg` — mock de PATH
- `ffmpeg.rs`: `parse_progress_line` — parsing de linha de progresso
- `preset.rs`: builtin presets têm todos os campos obrigatórios
- `job.rs`: `JobQueue::add/remove/overall_progress`

**Arquivos:** `mediaforge-core/src/ffmpeg.rs`, `mediaforge-core/src/preset.rs`, `mediaforge-core/src/job.rs`

**Validação:** `cargo test -p mediaforge-core` com zero falhas.

---

## 7. README / landing

**Objetivo:** Página simples com screenshots, instruções de instalação e download.

**Abordagem:** Markdown no `README.md` da raiz com:
- Nome e descrição
- Screenshots (Linux + Windows)
- Instruções de download e execução
- Requisitos (ffmpeg bundled, não precisa instalar nada)
- Links para releases no GitHub

**Arquivo:** `README.md`

---

## Ordem recomendada

```
1. Cancel real        (bug — afeta experiência)
2. Dragon Trap Adv    (bug — parâmetros incorretos)
3. Revisão visual     (validação de direção antes de investir em features)
4. Histórico          (feature)
5. Presets custom     (feature)
6. Testes             (qualidade)
7. README             (distribuição)
```

**Riscos:** Nenhum breaking change previsto. Todos os itens são aditivos ou correções pontuais.
