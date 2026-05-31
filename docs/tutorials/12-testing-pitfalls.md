# Tutorial 12 — Testes, TDD e Pitfalls

**Objetivo:** Compreender a estratégia de testes do projeto, a abordagem
TDD (RED → GREEN → REFACTOR), e os 10 principais pitfalls encontrados
durante o desenvolvimento.

---

## 1. Estrutura de Testes

| Suite | Framework | Arquivo | Testes |
|-------|-----------|---------|--------|
| Rust core | `cargo test` | `ffmpeg.rs`, `job.rs`, etc. | 124 |
| Rust tauri | `cargo test --lib` | `commands.rs` | 13 |
| JS unit | `node --test` | `test.js` | 42 |
| **Total** | | | **179** |

---

## 2. Rust: Testes de build_command()

O padrão para testar comandos ffmpeg:

```rust
fn cmd_str(p: &EncodeParams) -> String {
    let cmd = build_command(p, Path::new("in.mp4"), Path::new("out.mp4"));
    let args: Vec<String> = cmd.get_args()
        .map(|a| a.to_string_lossy().to_string()).collect();
    args.join(" ")
}

#[test]
fn codec_h264() {
    let p = EncodeParams { video_codec: VideoCodec::H264, ..Default::default() };
    assert!(cmd_str(&p).contains("libx264"));
}
```

**Por que `cmd_str()` e não executar ffmpeg real?** Executar ffmpeg real
requer binário instalado, é lento, e testa o ffmpeg, não nosso código.
Testamos a SAÍDA do comando, não a execução.

### TDD: progress event filtering

Este foi o teste mais importante do projeto. Documenta o comportamento
de que apenas `out_time_us` e `progress=end` disparam callbacks:

```rust
fn simulate_progress_events(lines: &[&str], total_us: Option<u64>)
    -> (usize, Vec<f64>)
{
    let mut call_count = 0;
    let mut pcts = vec![];
    let mut on_progress = |info: &ProgressInfo| {
        call_count += 1;
        pcts.push(info.progress_pct);
    };
    // Simula o loop de stderr
    for &line in lines {
        if let Some((key, value)) = parse_progress_line(line) {
            let mut info = ProgressInfo::default();
            let mut emit = true;
            match key.as_str() {
                "out_time_us" => { /* calcula progress_pct */ }
                "progress" => {
                    if value == "end" { info.progress_pct = 100.0; }
                    else { emit = false; }
                }
                _ => emit = false,
            }
            if emit { on_progress(&info); }
        }
    }
    (call_count, pcts)
}

#[test]
fn progress_filter_only_emits_out_time_and_end() {
    let lines = [
        "frame=0", "out_time_us=0", "frame=100",
        "out_time_us=2500000", "speed=3.5x",
        "out_time_us=5000000", "progress=continue",
        "out_time_us=9990000", "progress=end",
    ];
    let (count, pcts) = simulate_progress_events(&lines, Some(10_000_000));
    assert_eq!(count, 5);  // 4 out_time_us + 1 end
    assert_eq!(pcts, vec![0.0, 25.0, 50.0, 99.9, 100.0]);
}
```

Este teste pegou o bug em que `progress=continue` disparava callback com
`progress_pct=0.0`, sobrescrevendo o valor real na UI.

---

## 3. Rust: Testes de commands.rs

Testam os comandos Tauri como funções puras (sem janela):

```rust
#[test]
fn create_preset_rejects_builtin_id() {
    let p = Preset { id: "default".into(), ... };
    assert!(create_preset(p).is_err());
}

#[test]
fn create_and_delete_custom_preset() {
    create_preset(Preset { id: "__test__".into(), ... }).unwrap();
    assert!(get_presets().iter().any(|p| p.id == "__test__"));
    delete_preset("__test__".into()).unwrap();
}

#[test]
fn cancel_encoding_no_active() {
    cancel_encoding();  // não deve panicar
}
```

---

## 4. JS: Testes de funções puras

Extraímos funções do `main.js` para teste:

```js
function clamp(v, min, max, fallback) {
    const n = parseInt(v);
    return isNaN(n) ? fallback : Math.max(min, Math.min(max, n));
}

describe('clamp()', () => {
    it('within range', () => assert.equal(clamp(50, 0, 100, 0), 50));
    it('below min', () => assert.equal(clamp(-5, 0, 100, 0), 0));
    it('NaN returns fallback', () => assert.equal(clamp('abc', 0, 100, 42), 42));
});

describe('error sanitization', () => {
    it('roundtrips encode/decode', () => {
        const err = 'Error: can\'t "parse" this & that <tag>';
        const safe = encodeURIComponent(err);
        assert(!safe.includes('"'));
        assert(!safe.includes('<'));
        assert.equal(decodeURIComponent(safe), err);
    });
});
```

---

## 5. Pitfalls — Lições Aprendidas

### 1. `libopus` rejeita 44100 Hz
Só aceita 48000, 24000, 16000, 12000, 8000. Solução: validação em 3
camadas (HTML default, JS `fixOpusSampleRate()`, Rust fallback).

### 2. `innerHTML` + CSS transition = barra quebrada
Recriar elemento a cada 100ms impede transições CSS. Solução: update
direto no DOM (`fill.style.width = ...`).

### 3. `window.confirm()` não funciona no Tauri
WebView não implementa `confirm()`. Solução: `confirm()` do
`@tauri-apps/plugin-dialog`.

### 4. `encodeURIComponent` não escapa `'`
Seguro para atributos `"` mas não `'`. Solução: usar aspas duplas nos
atributos HTML.

### 5. esbuild e ternários com `in`
Esbuild não parseia `'key' in obj` dentro de ternários encadeados.
Solução: extrair para função com `switch`/`case`.

### 6. `cargo build` ≠ `cargo tauri build`
Só o segundo embute `WebView2Loader.dll` no Windows.

### 7. `bundle.css` é arquivo gerado
Deve ser rebuildado com Tailwind CLI a cada mudança no `input.css`.

### 8. GIF não aceita `-c:v libx264`
Precisa de filter chain `palettegen/paletteuse`. Solução: early return
em `build_command()`.

### 9. VP9/AV1 não aceitam `-preset`
Usam `-cpu-used` (VP9/AV1) ou `-preset` numérico (SVT-AV1).

### 10. CREATE_NO_WINDOW antes de qualquer `return`
Se um `return cmd` executar antes do `creation_flags`, o ffmpeg abre
janela CMD no Windows.

---

## 6. Verificação Final

```bash
bash scripts/check.sh
```

Deve mostrar:
```
=== Rust Tests (core) ===
test result: ok. 124 passed

=== Rust Tests (tauri) ===
test result: ok. 13 passed

=== JS Tests ===
ℹ pass 42

=== Cargo Check ===
Finished

=== All checks passed ===
```

---

## Parabéns!

Você completou todos os 12 tutoriais. Seu app está pronto para produção.

**Referências adicionais:**
- Documentação técnica completa: `docs/TECHNICAL.md`
- Tauri v2: https://tauri.app/v2/guides/
- ffmpeg: https://ffmpeg.org/documentation.html
- Tailwind CSS v4: https://tailwindcss.com/docs
- Rust: https://doc.rust-lang.org/book/
