# AGENTS.md — Model Backup

Guidance for AI agents working in this repository.

## Project summary

Cross-platform **Tauri 2** desktop app that discovers AI models from **LM Studio** and **Unsloth** (via Hugging Face Hub cache), backs them up to external drives, and supports sync, restore, delete, and offload operations.

## Architecture

```
src/                  React + TypeScript UI (pages, components, api/client.ts)
src-tauri/src/
  adapters/           Source scanners (lmstudio, huggingface_cache)
  core/
    operations/       backup, sync, restore, delete, offload, batch
    copy_engine.rs    File copy, symlinks, hashing
    scanner.rs        Scan + persist to SQLite
    drive_monitor.rs  Mount detection, backup path layout
  db/                 SQLite schema + queries
  lib.rs              Tauri commands + AppState
  types.rs            Shared serde types
```

## Key conventions

- **Rust backend owns all file I/O** — UI calls Tauri `invoke` commands only; never copy/delete files from the frontend.
- **Long operations run as background jobs** — spawn a thread, emit `job-progress` events, persist status in `jobs` table.
- **Adapter pattern** for new model sources — implement `SourceAdapter` in `adapters/`, register in `AdapterRegistry`.
- **Model identity** — logical models are directories (LM Studio) or HF cache repo snapshots (Unsloth); IDs like `lmstudio:author/model` or `hf:org/repo`.
- **Backup layout** on drives: `{drive_root}/lmstudio/...` or `{drive_root}/hf/...` plus `model.manifest.json` per model.
- **Offload** = copy to drive + remove source + symlink/junction at original path (Unix symlink, Windows junction).

## Common tasks

| Task | Where to change |
|------|-----------------|
| New model source (v2) | `src-tauri/src/adapters/` + `AdapterRegistry` |
| New operation | `src-tauri/src/core/operations/` + Tauri command in `lib.rs` + `src/api/client.ts` |
| UI page | `src/pages/` + route in `src/App.tsx` |
| Settings | `types.rs` `AppSettings`, `db/mod.rs`, `src/pages/Settings.tsx` |

## Commands

```bash
npm install
npm run tauri dev          # dev with hot reload
npm run tauri build        # production bundle
cd src-tauri && cargo test # Rust unit + integration tests
npm run build              # frontend only
```

## Default paths

- LM Studio: `~/.lmstudio-home-pointer` → home, else `~/.lmstudio` or `~/.cache/lm-studio`; models under `{home}/models/`
- Unsloth/HF: `HF_HUB_CACHE` → `HF_HOME/hub` → `~/.cache/huggingface/hub`

## Safety rules (do not bypass)

- Destructive ops check if LM Studio/Unsloth is running when `warn_if_app_running` is enabled.
- Never edit HF blob store internals — operate on whole snapshot directories.
- Verify drive is mounted before backup/sync/offload.
- Bulk ops (`backup_all`, `sync_all`) process models sequentially in one job; partial failure is reported in job message.

## UI patterns

- Dark theme CSS in `src/App.css`; use existing classes (`drive-form`, `form-field`, `toolbar`, `data-table`, badges).
- Forms: label above input, full-width fields, card-style containers for grouped inputs.
- Bulk actions: drive selector + "Backup all" / "Sync all" on Models and per-drive on Backup Drives page.

## Testing

- Unit tests in adapter modules (`#[cfg(test)]`)
- Integration tests in `src-tauri/tests/integration_test.rs`
- Prefer temp dirs via `tempfile` crate; no real user model paths in tests

## Out of scope (v2 roadmap)

OMLX, Ollama, standalone HF UI, custom folder watches, auto-sync on mount, cloud export.
