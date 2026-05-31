# Plano: Sistema de Licenciamento — Cratua Media Forge

**Meta:** Transformar o app desktop (Tauri v2) em produto pago com licenças,
mantendo a base de código atual. Adicionar ativação por chave, trial, e
validação online com fallback offline.

**Data:** 2026-05-30 | **Versão atual:** 0.3.23 | **Alvo:** 0.4.0

---

## 1. Modelo de Negócio

**Proposta:** Trial de 14 dias (funcionalidade completa) → licença perpétua
paga (one-time purchase). Sem assinatura recorrente — simplifica backend e
evita churn.

- **Preço sugerido:** USD 19-29 (one-time, licença vitalícia para 1 máquina)
- **Canais de venda:** Gumroad ou Lemon Squeezy (zero infra própria, entregam
  chave por email, webhook para validação)
- **Trial:** 14 dias com todas as features, watermark no canto ou toast
  "Unlicensed" nas primeiras 2 semanas

**Alternativa futura (fora do escopo inicial):** Plano Pro com features extras
(processamento em lote ilimitado, presets de estúdio, suporte prioritário).

---

## 2. Arquitetura Técnica

### 2.1 Nova crate: `mediaforge-license`

Isolar toda lógica de licenciamento em uma crate separada, sem dependência
de Tauri/UI. Comunicação com o backend de validação via `reqwest` (HTTP).

```
crates/mediaforge-license/
├── Cargo.toml
├── src/
│   ├── lib.rs          # pub mod declarations
│   ├── key.rs          # Key parsing, validation (Ed25519 signature)
│   ├── machine.rs      # Machine fingerprint (hostname + OS + MAC)
│   ├── trial.rs        # Trial state (first run timestamp, days remaining)
│   ├── state.rs        # LicenseState enum + persistence (~/.config/mediaforge/license.json)
│   └── verify.rs       # Online activation (HTTP POST to license server)
```

**Dependências novas:**
- `ed25519-dalek` — assinatura de chaves (sem serde, leve)
- `reqwest` com `rustls-tls` — chamadas HTTP (bloqueante, sem async runtime)
- `base64` — encoding da chave
- `sha2` — fingerprint da máquina

**Por que Ed25519 e não RSA?** Mais leve (32 bytes de chave pública vs ~256+),
implementação pura em Rust sem OpenSSL, assinatura determinística.

### 2.2 Formato da Chave de Licença

```
CRATUA-XXXX-XXXX-XXXX-XXXX
```

Onde `XXXX-XXXX-XXXX-XXXX` é uma string base32 (Crockford) de:
- 1 byte: versão do formato (0x01)
- 8 bytes: timestamp de expiração (0 = vitalícia)
- 4 bytes: machine-id hash (0 = qualquer máquina)
- 64 bytes: assinatura Ed25519

Total: 77 bytes → ~124 caracteres base32 → 4 grupos de 4 = 16 caracteres
visuais + prefixo.

**Validação offline (sem internet):**
1. Decodificar base32 → bytes
2. Verificar versão do formato
3. Extrair assinatura
4. Verificar assinatura com chave pública embedded no binário
5. Se expiração > 0, verificar timestamp_atual < expiração
6. Se machine-id != 0, verificar fingerprint local

### 2.3 Fingerprint da Máquina

```rust
fn machine_fingerprint() -> [u8; 8] {
    let host = hostname();           // "DESKTOP-ABC"
    let os = std::env::consts::OS;   // "linux" / "windows"
    let hash = sha2::Sha256::digest(format!("{host}|{os}|mediaforge").as_bytes());
    hash[..8].try_into().unwrap()
}
```

Propositalmente NÃO usa MAC address (problemático com VPNs, VMs, Wi-Fi/Ethernet
switch). O fingerprint é informativo, não criptograficamente seguro — o objetivo
é dificultar compartilhamento casual, não impedir ataque determinado.

### 2.4 Estado da Licença

Arquivo: `~/.config/mediaforge/license.json`

```json
{
  "key": "CRATUA-XXXX-...",
  "activated_at": 1717000000,
  "machine_fingerprint": "a1b2c3d4e5f6g7h8",
  "last_verified_at": 1717100000,
  "trial_started_at": 1716900000
}
```

**Máquina de estados:**

```
                ┌──────────┐
    primeiro ──→│  TRIAL   │──→ expirou ──→ BLOQUEADO
    launch      │ (14 dias)│
                └────┬─────┘
                     │ ativou com chave
                     ▼
                ┌──────────┐
                │  ATIVO   │──→ re-verificação falha ──→ BLOQUEADO
                │ (full)   │
                └──────────┘
```

### 2.5 Validação Online

Servidor simples (ou serverless function) em **Cloudflare Workers** ou
**Deno Deploy** (ambos têm free tier generoso):

**Endpoint:** `POST https://license.cratua.com/verify`

**Request:**
```json
{
  "key": "CRATUA-XXXX-...",
  "machine_fingerprint": "a1b2c3d4e5f6g7h8",
  "version": "0.4.0"
}
```

**Response:**
```json
{
  "valid": true,
  "expires_at": null,
  "message": "Licença válida"
}
```

O servidor consulta um banco KV (Cloudflare KV ou Deno KV) com as chaves
emitidas. Cada chave pode ser ativada em até 2 máquinas (generosidade).

**Periodicidade de verificação:** A cada 7 dias + em cada launch. Se falhar
(timeout 5s), usa cache offline (última verificação bem-sucedida). Após 30
dias sem verificação, exige online.

### 2.6 Integração com Payment Provider

**Lemon Squeezy (recomendado):**
- Merchant of Record (lida com VAT/Tax globalmente)
- Webhook envia chave gerada para o servidor de licenças
- Checkout hosted (não precisa construir página de pagamento)
- ~$0.50 + 5% por venda

**Fluxo:**
1. Usuário clica "Buy License" no app → abre URL do Lemon Squeezy no browser
2. Lemon Squeezy processa pagamento → webhook `order_created` → nosso worker
   gera chave Ed25519 (assinada com chave privada do servidor)
3. Lemon Squeezy envia email com a chave para o usuário
4. Usuário cola a chave no campo "Activate License" do app
5. App valida a chave offline (assinatura) + online (não revogada)

### 2.7 Nova UI

#### Splash → Tela de Licença

Após o splash atual de 2s, se não houver licença:

```
┌─────────────────────────────────────────────┐
│         Cratua Media Forge                   │
│                                             │
│     ⚡ Trial — 14 dias restantes             │
│                                             │
│  [   CRATUA-____-____-____-____   ]         │
│           [ Activate License ]              │
│                                             │
│  [ Try Free ]   [ Buy License → ]          │
│                                             │
│  v0.4.0  ·  github.com/laucalixto/mediaforge│
└─────────────────────────────────────────────┘
```

Design: modal centralizado com blur background, mesma paleta `#0b0b12` +
`#be4266` rose.

**"Try Free"** → fecha modal, entra no app normal com indicador de trial na
status bar: `⏳ Trial · 12 days left | 0 files | 0 jobs`

**"Buy License"** → abre link do Lemon Squeezy no browser externo
(`tauri_plugin_shell::open::<url>`).

#### Status Bar (durante trial)

```
⏳ Trial — 12 days left | 3 files | 2 jobs
```

Sem watermark na interface principal — o trial é full-featured, só a status
bar indica.

#### About Modal (com licença)

```
Cratua Media Forge
v0.4.0
Licensed to: laucalixto
Machine: DESKTOP-ABC
Expires: Never
```

#### Settings (nova seção)

```
License
  Status: Active / Trial (X days left) / Expired
  Key: CRATUA-XXXX-XXXX-XXXX-XXXX
  [ Deactivate ]  [ Buy License ]
```

---

## 3. Plano de Implementação (fases)

### Fase 1: Core de Licenciamento (4-6h)

**Tarefas:**
1. Criar `crates/mediaforge-license/` com Cargo.toml e dependências
2. Implementar `machine.rs` — fingerprint da máquina
3. Implementar `key.rs` — parse + validação Ed25519
4. Implementar `trial.rs` — lógica de trial (14 dias)
5. Implementar `state.rs` — persistência do estado em JSON
6. Implementar `verify.rs` — HTTP POST para worker (com timeout + cache offline)
7. Implementar `lib.rs` — API pública limpa
8. Testes unitários para key parse, validação offline, transições de estado

**API pública esperada:**
```rust
pub fn license_state() -> LicenseState;          // Trial/Active/Blocked + days left
pub fn activate_license(key: &str) -> Result<ActivationResult>;
pub fn deactivate_license() -> Result<()>;
pub fn check_online() -> Result<OnlineStatus>;   // chamada periódica
```

### Fase 2: Worker de Validação (2-3h)

**Tarefas:**
1. Criar repositório separado `mediaforge-license-server`
2. Cloudflare Worker (JS/TS) com KV namespace
3. Endpoint `POST /verify` — valida chave contra KV
4. Endpoint `POST /activate` — associa máquina à chave (webhook do Lemon Squeezy)
5. Script de geração de chaves offline (CLI em Rust, usa chave privada Ed25519)
6. Deploy com `wrangler publish`

**KV schema:**
```
key: "key:CRATUA-XXXX..." → { created_at, expires_at, activations: [fingerprint, ...], revoked: false }
key: "fingerprint:a1b2c3..." → ["CRATUA-XXXX...", ...]
```

### Fase 3: Integração Tauri + UI (3-4h)

**Tarefas:**
1. Adicionar `mediaforge-license` como dependency em `mediaforge-tauri`
2. Adicionar commands: `get_license_state`, `activate_license`,
   `deactivate_license`, `buy_license` (abre browser)
3. Modificar `lib.rs` — chamar `license_state()` no startup, emitir evento
4. Criar tela de licença (HTML/CSS — modal no `index.html`)
5. Modificar init do JS — após splash, verificar estado da licença
6. Se trial/bloqueado → mostrar modal de licença
7. Adicionar indicador na status bar
8. Adicionar seção License no Settings modal
9. Atualizar About modal com info de licença

### Fase 4: Proteção e Edge Cases (2-3h)

**Tarefas:**
1. Ofuscação básica — `strip = "symbols"` no release profile, LTO enabled
2. Chave pública embedded como array de bytes (não string literal óbvia)
3. Verificação de relógio alterado — guardar `last_seen_timestamp` e detectar
   clock rollback (se timestamp < último visto, trial conta como expirado)
4. Teste de desinstalação/reinstalação — trial persiste via fingerprint
   (reinstalar não reseta)
5. Bloqueio de VMs — fingerprint inclui `std::env::consts::OS` + hostname,
   não impede mas dificulta
6. Log anti-tamper — se `license.json` for modificado manualmente e assinatura
   não bater, invalida

### Fase 5: Lemon Squeezy + Deploy (2h)

**Tarefas:**
1. Criar produto no Lemon Squeezy (one-time, USD 19-29)
2. Configurar webhook → Cloudflare Worker
3. Testar fluxo completo: compra → webhook → chave gerada → email → ativação
4. Configurar página de checkout customizada (domínio `license.cratua.com`)
5. Atualizar links no app (Buy License)
6. Testar em Windows (cross-compile)

---

## 4. Arquivos que Serão Modificados/Criados

### Novos:
- `crates/mediaforge-license/Cargo.toml`
- `crates/mediaforge-license/src/lib.rs`
- `crates/mediaforge-license/src/key.rs`
- `crates/mediaforge-license/src/machine.rs`
- `crates/mediaforge-license/src/trial.rs`
- `crates/mediaforge-license/src/state.rs`
- `crates/mediaforge-license/src/verify.rs`
- `crates/mediaforge-license/tests/integration.rs`
- `scripts/gen-license-key.rs` (CLI offline key generator)

### Modificados:
- `Cargo.toml` (workspace — adicionar `mediaforge-license` member)
- `crates/mediaforge-tauri/Cargo.toml` (adicionar dep `mediaforge-license` + `reqwest`)
- `crates/mediaforge-tauri/src/lib.rs` (novos commands de licença)
- `crates/mediaforge-tauri/src/commands.rs` (implementação dos commands)
- `crates/mediaforge-tauri/tauri.conf.json` (CSP — permitir license.cratua.com)
- `crates/mediaforge-tauri/ui/index.html` (modal de licença + seção settings)
- `crates/mediaforge-tauri/ui/src/main.js` (lógica de licença no frontend)
- `crates/mediaforge-tauri/package.json` (versão)

---

## 5. Plano de Testes

### Testes unitários (Rust):
- `key.rs`: parse de chave válida, chave inválida, assinatura corrompida,
  expiração passada, machine-id mismatch
- `trial.rs`: início do trial, dias restantes, expiração, reinstalação
- `state.rs`: roundtrip serialize/deserialize, transições de estado,
  clock rollback detection

### Testes de integração:
- Fluxo trial: primeiro launch → 14 dias → expira → bloqueio
- Ativação offline: chave válida sem internet → ativa
- Ativação online: chave válida com worker → ativa + fingerprint registrado
- Revogação: worker marca chave como revoked → próxima verificação bloqueia

### Smoke test manual:
- `cargo test -p mediaforge-license`
- `cargo build --release` — verificar sem regressão no app existente
- Rodar app, verificar modal de trial aparece
- Ativar com chave de teste, verificar status "Active"

---

## 6. Riscos e Tradeoffs

| Risco | Mitigação |
|-------|-----------|
| Usuário altera relógio do sistema para extender trial | Guardar `last_seen_timestamp`; se timestamp atual < último visto, considerar expirado |
| Chave compartilhada em fóruns | Limitar a 2 ativações por chave (servidor); fingerprint binding local |
| Servidor offline → app quebra | Validação offline (assinatura Ed25519) cobre o caso básico; online só para revogação e binding |
| Cracker remove a verificação do binário | Ofuscação básica; sem solução perfeita em app desktop. Aceitar ~5-10% de pirataria como custo do modelo |
| Lemon Squeezy fecha/rejeita | Usar formato de chave proprietário; migrar para outro provider (Gumroad, Paddle) é só trocar o webhook |

---

## 7. Perguntas em Aberto

1. **Preço:** USD 19 ou USD 29? (Lemon Squeezy cobra ~$0.50 + 5%, então $19
   líquido ≈ $17.55; $29 líquido ≈ $27.05)
2. **Domínio:** `license.cratua.com` — já possui o domínio `cratua.com`?
   Se não, usar subdomínio gratuito do Cloudflare Workers
   (`mediaforge-license.laucalixto.workers.dev`)
3. **Key generator offline:** Quem vai gerar as chaves iniciais para teste?
   Eu mesmo, ou você tem preferência por manter esse script?
4. **Telemetria:** Vale adicionar telemetria anônima de uso (opt-in) para
   saber quantos usuários convertem do trial?

---

## 8. Análise: Licenciamento Pago vs Doação vs Freemium

*Adicionado em 2026-05-30 após revisão sem viés de confirmação.*

### O mercado real

HandBrake domina esse nicho há 10+ anos. É grátis, open source, tem centenas
de colaboradores, presets para todo dispositivo do planeta. Fora ele existem
dezenas de alternativas gratuitas: Shutter Encoder, FFmpeg Batch, WinFF,
XMedia Recode, Permute (Mac). As poucas ferramentas pagas (VideoProc,
WonderShare) competem com orçamento de marketing, não com qualidade técnica.

O Cratua compete em UX (Tailwind bonito, sidebar, presets) e em portabilidade
(Tauri cross-platform leve). Mas o diferencial real vs HandBrake é pequeno.

### Contra o licenciamento pago

**1. Pirataria trivial.** Ed25519 em binário local é security theater.
Qualquer pessoa com Ghidra e 10 minutos remove a verificação.

**2. Fricção mata adoção.** Hoje: baixar → abrir → usar. Com licença: baixar
→ abrir → modal de trial → testar → gostar → pegar cartão → pagar → colar
chave. Cada etapa perde 30-50% dos usuários.

**3. Suporte pago é dívida.** Quem paga $19 exige que funcione. Bug vira
incidente. Feature request vira obrigação. O tempo gasto com suporte pode
consumir toda a receita.

**4. Receita projetada é baixa.** Mesmo otimista: 100 vendas no ano 1 = ~$1,745
líquido. Dá ~$145/mês. Provavelmente não cobre o esforço.

**5. Dano reputacional.** Projeto MIT que vira pago parece bait-and-switch.
Quem já usa vai se sentir traído. Fork é trivial (o código está público).

### Contra doação pura

**1. Doação não paga nada.** GitHub Sponsors: mediana é $0/mês. "Buy me a
coffee": mesma coisa. Sem base de usuários grande (10k+), doação é simbólica.

**2. Sem urgência.** App grátis para sempre = zero incentivo para doar.

### O meio-termo: Freemium (recomendado)

Manter core gratuito (conversão básica, presets built-in, simple mode) e
cobrar por features que power users realmente valorizam:

- **Batch processing ilimitado** (>3 arquivos simultâneos)
- **Watch folders** (monitorar pasta, converter automaticamente)
- **Output templates** (nome customizado: `{date}_{codec}_{res}`)
- **Cloud upload** (S3, FTP, Google Drive pós-conversão)
- **Presets custom ilimitados** (free = 3, pro = ilimitado)

**Por que isso funciona melhor:**
- App grátis ainda é genuinamente útil (nível HandBrake)
- Usuários gratuitos são marketing orgânico
- Power users que convertem 50+ arquivos/semana pagam felizes
- Fricção só aparece quando o usuário já extraiu valor do app
- Diferencial real vs HandBrake (batch + watch folders + cloud)

### Matriz de decisão

| Critério | Licença Paga | Doação | Freemium |
|----------|-------------|--------|----------|
| Tempo implementação | 13-18h | 30min | 20-25h |
| Manutenção contínua | Alta | Zero | Média |
| Receita ano 1 (realista) | $200-1000 | $0-50 | $300-1500 |
| Risco rejeição | Alto | Zero | Baixo |
| Crescimento usuários | Baixo | Alto | Médio |
| Suporte esperado | Exigente | Nenhum | Moderado |
| Diferencial vs HandBrake | Nenhum (pior: pago) | Nenhum | Sim (batch/watch) |

### Recomendação final

Começar com **doação** agora (30min: botão no About + GitHub Sponsors link).
Medir downloads e tração por 3-6 meses enquanto constrói as features Pro.
Quando houver base instalada, ativar **freemium** com batch/watch folders.
Pular a fase de licenciamento puro — é o pior dos dois mundos (fricção máxima,
receita mínima, proteção inexistente).
