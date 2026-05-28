# Plano Completo: Advanced Tab — Cratua Media Forge

**Data:** 2026-05-28
**Objetivo:** Corrigir todos os bugs e validações da aba Advanced sem quebrar a Simple.

---

## Bugs Confirmados

### B1. Profile inválido para codec
- **Sintoma:** `-profile:v baseline` com `libx265` → exit code -22
- **Causa:** Dropdown fixo com profiles H.264 (baseline/main/high)
- **Correção:** Tornar `#a-profile` dinâmico — `onchange` no `#a-vcodec` popula opções corretas

### B2. CRF + bitrate simultâneos
- **Sintoma:** `-crf 30 -b:v 5000k` conflita no ffmpeg
- **Causa:** `collectAdvParams()` envia ambos se preenchidos
- **Correção:** Se CRF > 0, ignorar bitrate (CRF prioritário). Aviso visual.

### B3. Seta do Pixel Format invertida
- **Sintoma:** ▶ abre painel, ▼ fecha (invertido)
- **Causa:** CSS `rotate(90deg)` no ▶ aponta pra cima em algumas fontes
- **Correção:** Trocar CSS por toggle explícito `▶` ↔ `▼` no JS (para TODAS as collapsible sections)

---

## Problemas Adicionais Encontrados na Revisão

### P1. Codec + Container incompatíveis
- VP9 + MP4 → erro. VP9 exige WebM.
- MP3 (audio) + MP4 → funciona mas é confuso
- **Correção:** `onchange` no codec ajusta container automaticamente (VP9→WebM, MP3→MP3)

### P2. Pixel Format + Profile conflitam
- `yuv444p` exige profile high (H.264) ou main (H.265)
- `baseline` + `yuv444p` → erro
- **Correção:** Aviso visual se combinação for inválida, ou auto-ajuste

### P3. CRF=0 sem aviso (lossless)
- CRF 0 = lossless, arquivo enorme
- **Correção:** Label muda para "lossless ⚠" quando CRF=0

### P4. Botão "Auto" do CRF enganoso
- Mostra "auto" mas seta 23 (valor fixo)
- **Correção:** Mostrar "23" em vez de "auto"

### P5. Command Preview nunca preenchido
- Comando `build_command_preview` existe no Rust mas nunca chamado
- **Correção:** Chamar `invoke('build_command_preview', {params})` ao abrir tab Output ou ao mudar params

### P6. Filtros faltando no dropdown
- Video: falta Brightness, Contrast, Saturation, Crop
- Audio: falta Highpass, Lowpass
- **Correção:** Adicionar opções ao dropdown

### P7. Width/Height ímpares quebram alguns codecs
- Codecs exigem resolução par (even)
- **Correção:** Auto-arredondar para par na coleta

### P8. Sincronização Simple↔Advanced
- Mudar sliders no Simple e alternar para Advanced: campos não refletem
- **Correção:** Ao clicar na tab Advanced, sincronizar campos com `S.params` atual

### P9. Trim sem validação de formato
- Aceita qualquer string, ffmpeg pode rejeitar
- **Correção:** Aceitar como está (ffmpeg faz validação), mas documentar placeholder

---

## Plano de Execução (ordem)

| # | O quê | Arquivo |
|---|---|---|
| 1 | Corrigir seta ▶/▼ — toggle explícito em vez de CSS | `index.html` + `main.js` |
| 2 | Profile dinâmico por codec | `main.js` |
| 3 | CRF prioritário + aviso bitrate ignorado | `main.js` |
| 4 | Codec→Container auto-ajuste | `main.js` |
| 5 | CRF=0 warning lossless + botão Auto mostra 23 | `main.js` |
| 6 | Command Preview funcional | `main.js` |
| 7 | Filtros adicionais no dropdown | `index.html` |
| 8 | Width/Height auto-arredondar para par | `main.js` |
| 9 | Sincronizar Simple→Advanced ao trocar modo | `main.js` |

**Arquivos:** `ui/index.html` + `ui/src/main.js`  
**Rust:** 0 mudanças  
**Tempo:** ~1h30

---

## Validação pós-implementação

1. Selecionar H.265 → profile deve mostrar main/main10/main12
2. Setar CRF=30 + bitrate=5000 → comando ffmpeg deve ter `-crf 30` sem `-b:v`
3. Selecionar VP9 → container deve mudar para WebM
4. Clicar ▶ no Pixel Format → deve abrir painel
5. Abrir tab Output → Command Preview deve mostrar comando
6. Voltar para Simple, mudar slider, alternar para Advanced → campos sincronizados
