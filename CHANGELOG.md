# Changelog

## [Unreleased]

### Fixed
- Progress bar: CSS class mismatch (`progress-fill` → `progress-bar-fill`)
- Progress bar: added `.progress-pct` CSS class for percentage text
- Light mode: progress bar background override
- Sponsor button: use `@tauri-apps/plugin-shell` `open()` instead of blocked `window.open()`

### Added
- Encoding timeout (30s) — ffmpeg hang detection
- Settings: output directory, default mode, default preset
- Keyboard shortcuts (Ctrl+O, Ctrl+Enter, Esc, Ctrl+,)
- Theme toggle (dark/light) with CSS custom properties
- Built-in preset IDs from backend (no more hardcoded arrays)
- Rename indicator (⚠) when output path auto-renamed to `_converted`
- `.editorconfig` for consistent formatting

### Changed
- Debounce command preview (300ms) — reduces IPC calls 20x during typing
- Progress bar: CSS-based (div + gradient) instead of Unicode characters
- Error messages: click-to-copy via event delegation, HTML-safe encoding
- Browse handler deduplicated into `browseOutputDir()`
- Preset save/delete handlers deduplicated into `saveCurrentPreset()`/`deleteCurrentPreset()`
- `collectAdvParams`/`collectSimpleParams`: validated ranges with `clamp()`
- State object documented with ownership comments
- CSS moved from `<style>` in index.html to `input.css`
- `check.sh` now runs `cargo test -p mediaforge-tauri --lib`

### Removed
- `mediaforge-gui` (egui) from workspace members — Tauri is the only UI
- `progressBar()` function — replaced by CSS progress bar

### Fixed
- Custom presets: added `PRESET_LOCK` mutex, identical to `HISTORY_LOCK`
- `renderQueue`: error messages sanitized with `encodeURIComponent`

## [0.3.23] - 2026-05-30
- Fix progress bar textual + total queue diag, icon update, opus sample rate guards

## [0.3.22] - 2026-05-30
- Release

## [0.3.21] - 2026-05-30
- Release

## [0.3.20] - 2026-05-30
- Fix: progress bar now renders on first progress event

## [0.3.19] - 2026-05-30
- Release

## [0.3.18] - 2026-05-30
- Release

## [0.3.17] - 2026-05-30
- Release

## [0.3.16] - 2026-05-30
- Release

## [0.3.15] - 2026-05-30
- Fix: sample_rate 48000 everywhere (frontend state + fallback + backend default)

## [0.3.14] - 2026-05-30
- Fix: default sample_rate 44100→48000 for Opus compatibility, increase error display to 500 chars

## [0.3.12] - 2026-05-29
- Release

## [0.3.11] - 2026-05-29
- Release

## [0.3.10] - 2026-05-29
- Release

## [0.3.9] - 2026-05-29
- Release

## [0.3.8] - 2026-05-29
- Fix: suppress CMD window for GIF, fix progress bar CSS, add probe diagnostics

## [0.3.5] - 2026-05-29
- Initial Tauri v2 migration from egui
